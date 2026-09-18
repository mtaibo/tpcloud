import io
import mimetypes
import os
import secrets
import shutil
import zipfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import List

import aiofiles
from fastapi import APIRouter, Depends, HTTPException, Query, Request, UploadFile, File
from fastapi.responses import FileResponse, StreamingResponse
from pydantic import BaseModel
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import FileViewToken
from app.utils import get_base as _base, resolve_path as _resolve, check_access as _check_access

router = APIRouter(prefix="/api/files", tags=["files"])


@router.get("/list")
async def list_directory(
    request: Request,
    path: str = Query(default=""),
    location: str = Query(default="external"),
):
    user = await get_current_user(request)
    _check_access(location, path, user["email"], user["is_admin"])
    base = _base(location)
    dir_path = _resolve(base, path)

    if not dir_path.exists():
        clean = path.lstrip("/")
        parts = Path(clean).parts if clean else ()
        if (
            location == "external"
            and len(parts) >= 2
            and parts[0] == "users"
            and parts[1] == user["email"]
        ):
            dir_path.mkdir(parents=True, exist_ok=True)
        else:
            raise HTTPException(404, "Directory not found")

    if not dir_path.is_dir():
        raise HTTPException(400, "Path is not a directory")

    entries = []
    try:
        items = sorted(dir_path.iterdir(), key=lambda x: (x.is_file(), x.name.lower()))
    except PermissionError:
        raise HTTPException(403, "Permission denied")

    for item in items:
        try:
            stat = item.stat()
            entries.append(
                {
                    "name": item.name,
                    "type": "file" if item.is_file() else "directory",
                    "size": stat.st_size if item.is_file() else None,
                    "modified": stat.st_mtime,
                }
            )
        except (PermissionError, OSError):
            continue

    return {"path": path, "entries": entries}


@router.post("/upload")
async def upload_files(
    request: Request,
    path: str = Query(default=""),
    location: str = Query(default="external"),
    files: List[UploadFile] = File(...),
):
    user = await get_current_user(request)
    _check_access(location, path, user["email"], user["is_admin"])
    base = _base(location)
    dir_path = _resolve(base, path)

    if not dir_path.exists() or not dir_path.is_dir():
        raise HTTPException(404, "Directory not found")

    uploaded = []
    for file in files:
        filename = Path(file.filename).name
        if not filename:
            continue
        dest = (dir_path / filename).resolve()
        if not str(dest).startswith(str(dir_path.resolve())):
            continue
        async with aiofiles.open(dest, "wb") as f:
            while chunk := await file.read(1024 * 1024):
                await f.write(chunk)
        uploaded.append(filename)

    return {"uploaded": uploaded}


class TokenBody(BaseModel):
    path: str
    location: str = "external"


@router.post("/token")
async def create_file_token(
    request: Request,
    body: TokenBody,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    _check_access(body.location, body.path, user["email"], user["is_admin"])
    while True:
        token = secrets.token_urlsafe(8)
        if not db.exec(select(FileViewToken).where(FileViewToken.token == token)).first():
            break
    db.add(FileViewToken(
        token=token,
        path=body.path,
        location=body.location,
        owner_email=user["email"],
    ))
    db.commit()
    return {"token": token}


@router.get("/open/{token}")
async def open_file_by_token(
    token: str,
    request: Request,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    file_token = db.exec(select(FileViewToken).where(FileViewToken.token == token)).first()
    if not file_token:
        raise HTTPException(404, "Token not found")
    if file_token.first_accessed_at:
        expiry = file_token.first_accessed_at.replace(tzinfo=timezone.utc) + timedelta(hours=24)
        if datetime.now(timezone.utc) > expiry:
            raise HTTPException(410, "Token expired")
    if not file_token.first_accessed_at:
        file_token.first_accessed_at = datetime.now(timezone.utc)
        db.add(file_token)
        db.commit()
    _check_access(file_token.location, file_token.path, user["email"], user["is_admin"])
    base = _base(file_token.location)
    file_path = _resolve(base, file_token.path)
    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found")
    media_type, _ = mimetypes.guess_type(str(file_path))
    media_type = media_type or "application/octet-stream"
    return FileResponse(
        path=str(file_path),
        media_type=media_type,
        headers={"Content-Disposition": f'inline; filename="{file_path.name}"'},
    )


@router.get("/thumbnail")
async def thumbnail_file(
    request: Request,
    path: str = Query(...),
    location: str = Query(default="external"),
    size: int = Query(default=220),
):
    user = await get_current_user(request)
    _check_access(location, path, user["email"], user["is_admin"])
    base = _base(location)
    file_path = _resolve(base, path)

    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found")

    media_type, _ = mimetypes.guess_type(str(file_path))
    if not media_type or not media_type.startswith("image/"):
        return FileResponse(
            path=str(file_path),
            media_type=media_type or "application/octet-stream",
            headers={"Content-Disposition": f'inline; filename="{file_path.name}"'},
        )

    try:
        from PIL import Image
        import io as _io

        size = max(50, min(size, 800))
        with Image.open(file_path) as img:
            img = img.convert("RGB")
            w, h = img.size
            min_dim = min(w, h)
            left = (w - min_dim) // 2
            top = (h - min_dim) // 2
            img = img.crop((left, top, left + min_dim, top + min_dim))
            img = img.resize((size, size), Image.LANCZOS)
            buf = _io.BytesIO()
            img.save(buf, format="JPEG", quality=82, optimize=True)
            buf.seek(0)
        return StreamingResponse(
            buf,
            media_type="image/jpeg",
            headers={"Cache-Control": "max-age=86400", "Content-Disposition": "inline"},
        )
    except Exception:
        return FileResponse(
            path=str(file_path),
            media_type=media_type,
            headers={"Content-Disposition": f'inline; filename="{file_path.name}"'},
        )


@router.get("/view")
async def view_file(
    request: Request,
    path: str = Query(...),
    location: str = Query(default="external"),
):
    user = await get_current_user(request)
    _check_access(location, path, user["email"], user["is_admin"])
    base = _base(location)
    file_path = _resolve(base, path)

    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found")

    media_type, _ = mimetypes.guess_type(str(file_path))
    media_type = media_type or "application/octet-stream"

    return FileResponse(
        path=str(file_path),
        media_type=media_type,
        headers={"Content-Disposition": f'inline; filename="{file_path.name}"'},
    )


@router.get("/download")
async def download_file(
    request: Request,
    path: str = Query(...),
    location: str = Query(default="external"),
):
    user = await get_current_user(request)
    _check_access(location, path, user["email"], user["is_admin"])
    base = _base(location)
    file_path = _resolve(base, path)

    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found")

    return FileResponse(
        path=file_path,
        filename=file_path.name,
        media_type="application/octet-stream",
    )


@router.delete("")
async def delete_item(
    request: Request,
    path: str = Query(...),
    location: str = Query(default="external"),
):
    user = await get_current_user(request)
    _check_access(location, path, user["email"], user["is_admin"])
    base = _base(location)
    item_path = _resolve(base, path)

    if item_path == base.resolve():
        raise HTTPException(400, "Cannot delete root")

    clean = path.lstrip("/")
    parts = Path(clean).parts if clean else ()
    if not user["is_admin"] and len(parts) == 1 and parts[0] == "shared":
        raise HTTPException(403, "Cannot delete the shared folder")

    if not item_path.exists():
        raise HTTPException(404, "Not found")

    if item_path.is_file() or item_path.is_symlink():
        item_path.unlink()
    else:
        shutil.rmtree(item_path)

    return {"deleted": path}


class MkdirBody(BaseModel):
    path: str
    location: str = "external"


@router.post("/mkdir")
async def create_directory(request: Request, body: MkdirBody):
    user = await get_current_user(request)
    _check_access(body.location, body.path, user["email"], user["is_admin"])
    base = _base(body.location)
    new_dir = _resolve(base, body.path)

    if new_dir.exists():
        raise HTTPException(400, "Already exists")

    try:
        new_dir.mkdir(parents=True)
    except PermissionError:
        raise HTTPException(403, "Permission denied")

    return {"created": body.path}


class TouchBody(BaseModel):
    path: str
    location: str = "external"


@router.post("/touch")
async def create_file(request: Request, body: TouchBody):
    user = await get_current_user(request)
    _check_access(body.location, body.path, user["email"], user["is_admin"])
    base = _base(body.location)
    file_path = _resolve(base, body.path)

    if file_path.exists():
        raise HTTPException(400, "Already exists")

    try:
        file_path.touch()
    except PermissionError:
        raise HTTPException(403, "Permission denied")

    return {"created": body.path}


class RenameBody(BaseModel):
    path: str
    new_name: str
    location: str = "external"


@router.post("/rename")
async def rename_item(request: Request, body: RenameBody):
    user = await get_current_user(request)
    _check_access(body.location, body.path, user["email"], user["is_admin"])
    base = _base(body.location)
    item = _resolve(base, body.path)

    if not item.exists():
        raise HTTPException(404, "Not found")

    new_name = Path(body.new_name).name
    if not new_name:
        raise HTTPException(400, "Invalid name")

    dest = item.parent / new_name
    if dest.exists():
        raise HTTPException(400, "Already exists")

    try:
        item.rename(dest)
    except PermissionError:
        raise HTTPException(403, "Permission denied")

    return {"renamed": body.path}


def _copy_name(parent: Path, name: str) -> Path:
    if '.' in name and not name.startswith('.'):
        stem, ext = name.rsplit('.', 1)
        ext = f'.{ext}'
    else:
        stem, ext = name, ''
    candidate = parent / f'{stem} copy{ext}'
    n = 2
    while candidate.exists():
        candidate = parent / f'{stem} copy {n}{ext}'
        n += 1
    return candidate


class CopyBody(BaseModel):
    path: str
    dest_dir: str
    location: str = "external"


@router.post("/copy")
async def copy_item(request: Request, body: CopyBody):
    user = await get_current_user(request)
    _check_access(body.location, body.path, user["email"], user["is_admin"])
    _check_access(body.location, body.dest_dir, user["email"], user["is_admin"])
    base = _base(body.location)
    item = _resolve(base, body.path)
    dest_dir = _resolve(base, body.dest_dir)

    if not item.exists():
        raise HTTPException(404, "Source not found")
    if not dest_dir.is_dir():
        raise HTTPException(400, "Destination is not a directory")

    dest = dest_dir / item.name
    if dest.exists():
        raise HTTPException(400, "Item already exists at destination")

    try:
        if item.is_dir():
            shutil.copytree(str(item), str(dest))
        else:
            shutil.copy2(str(item), str(dest))
    except PermissionError:
        raise HTTPException(403, "Permission denied")

    return {"copied": str(dest.relative_to(base))}


class DuplicateBody(BaseModel):
    path: str
    location: str = "external"


@router.post("/duplicate")
async def duplicate_item(request: Request, body: DuplicateBody):
    user = await get_current_user(request)
    _check_access(body.location, body.path, user["email"], user["is_admin"])
    base = _base(body.location)
    item = _resolve(base, body.path)

    if not item.exists():
        raise HTTPException(404, "Not found")

    dest = _copy_name(item.parent, item.name)

    try:
        if item.is_dir():
            shutil.copytree(str(item), str(dest))
        else:
            shutil.copy2(str(item), str(dest))
    except PermissionError:
        raise HTTPException(403, "Permission denied")

    return {"duplicated": str(dest.relative_to(base))}


class MoveBody(BaseModel):
    path: str
    dest_dir: str
    location: str = "external"


@router.post("/move")
async def move_item(request: Request, body: MoveBody):
    user = await get_current_user(request)
    _check_access(body.location, body.path, user["email"], user["is_admin"])
    _check_access(body.location, body.dest_dir, user["email"], user["is_admin"])
    base = _base(body.location)
    item = _resolve(base, body.path)
    dest_dir = _resolve(base, body.dest_dir)

    if not item.exists():
        raise HTTPException(404, "Source not found")
    if not dest_dir.is_dir():
        raise HTTPException(400, "Destination is not a directory")

    if item.is_dir() and str(dest_dir).startswith(str(item) + os.sep):
        raise HTTPException(400, "Cannot move a folder into itself")

    dest = dest_dir / item.name
    if dest.exists():
        raise HTTPException(400, "Item already exists at destination")

    try:
        shutil.move(str(item), str(dest))
    except PermissionError:
        raise HTTPException(403, "Permission denied")

    return {"moved": str(dest.relative_to(base))}


@router.get("/download-zip")
async def download_folder_zip(
    request: Request,
    path: str = Query(...),
    location: str = Query(default="external"),
):
    user = await get_current_user(request)
    _check_access(location, path, user["email"], user["is_admin"])
    base = _base(location)
    dir_path = _resolve(base, path)

    if not dir_path.exists() or not dir_path.is_dir():
        raise HTTPException(400, "Path is not a directory")

    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zf:
        for f in sorted(dir_path.rglob("*")):
            if f.is_file():
                zf.write(f, Path(dir_path.name) / f.relative_to(dir_path))
    buf.seek(0)

    return StreamingResponse(
        buf,
        media_type="application/zip",
        headers={"Content-Disposition": f'attachment; filename="{dir_path.name}.zip"'},
    )


class CompressBody(BaseModel):
    path: str
    location: str = "external"


@router.post("/compress")
async def compress_item(request: Request, body: CompressBody):
    user = await get_current_user(request)
    _check_access(body.location, body.path, user["email"], user["is_admin"])
    base = _base(body.location)
    item = _resolve(base, body.path)

    if not item.exists():
        raise HTTPException(404, "Not found")

    zip_path = item.parent / f"{item.name}.zip"
    if zip_path.exists():
        raise HTTPException(400, "Zip file already exists")

    try:
        with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
            if item.is_dir():
                for f in sorted(item.rglob("*")):
                    if f.is_file():
                        zf.write(f, Path(item.name) / f.relative_to(item))
            else:
                zf.write(item, item.name)
    except PermissionError:
        raise HTTPException(403, "Permission denied")

    return {"compressed": str(zip_path.relative_to(base))}


class DecompressBody(BaseModel):
    path: str
    location: str = "external"


@router.post("/decompress")
async def decompress_item(request: Request, body: DecompressBody):
    user = await get_current_user(request)
    _check_access(body.location, body.path, user["email"], user["is_admin"])
    base = _base(body.location)
    file_path = _resolve(base, body.path)

    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found")

    if file_path.suffix.lower() != ".zip":
        raise HTTPException(400, "Not a zip file")

    dest_dir = file_path.parent.resolve()

    try:
        with zipfile.ZipFile(file_path, "r") as zf:
            for member in zf.namelist():
                member_path = (dest_dir / member).resolve()
                if not str(member_path).startswith(str(dest_dir) + os.sep) and str(member_path) != str(dest_dir):
                    raise HTTPException(400, "Zip contains unsafe paths")
            zf.extractall(dest_dir)
    except zipfile.BadZipFile:
        raise HTTPException(400, "Invalid or corrupted zip file")
    except PermissionError:
        raise HTTPException(403, "Permission denied")

    return {"decompressed": body.path}
