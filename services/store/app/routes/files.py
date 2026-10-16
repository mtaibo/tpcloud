import asyncio
import hashlib
import io
import json
import os
import secrets
import shutil
import threading
import zipfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import List

import aiofiles
from fastapi import APIRouter, Depends, HTTPException, Query, Request, UploadFile, File
from fastapi.concurrency import run_in_threadpool
from fastapi.responses import FileResponse, StreamingResponse
from pydantic import BaseModel
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session, DATA_DIR
from app.models import FileViewToken, FolderIcon, PendingUpload
from app.utils import (
    get_base as _base,
    resolve_path as _resolve,
    check_access as _check_access,
    list_directory_entries as _list_entries,
    guess_media_type as _guess_media_type,
)

_THUMB_DIR = DATA_DIR / "thumbs"
_thumb_sem = asyncio.Semaphore(4)

# Per-upload-id lock guarding the read-modify-write of PendingUpload.received_chunks, so two
# chunks of the same upload landing concurrently (parallel connections, or a client retry racing
# the original request) can't lose one or the other's update to the received-chunk set.
_upload_locks_guard = threading.Lock()
_upload_locks: dict[str, threading.Lock] = {}


def _lock_for_upload(upload_id: str) -> threading.Lock:
    with _upload_locks_guard:
        lock = _upload_locks.get(upload_id)
        if lock is None:
            lock = threading.Lock()
            _upload_locks[upload_id] = lock
        return lock


def _release_upload_lock(upload_id: str) -> None:
    with _upload_locks_guard:
        _upload_locks.pop(upload_id, None)


def _thumb_cache_path(file_path: Path, size: int) -> Path:
    mtime = int(file_path.stat().st_mtime)
    key = f"{file_path}|{mtime}|{size}"
    h = hashlib.sha256(key.encode()).hexdigest()[:24]
    return _THUMB_DIR / f"{h}.jpg"


def _build_thumbnail(file_path: Path, cache_path: Path, size: int) -> None:
    from PIL import Image
    with Image.open(file_path) as img:
        img = img.convert("RGB")
        w, h = img.size
        min_dim = min(w, h)
        left = (w - min_dim) // 2
        top = (h - min_dim) // 2
        img = img.crop((left, top, left + min_dim, top + min_dim))
        img = img.resize((size, size), Image.LANCZOS)
        tmp = cache_path.with_suffix(".tmp")
        img.save(str(tmp), format="JPEG", quality=75, optimize=True)
        tmp.replace(cache_path)


async def _ensure_thumbnail(file_path: Path, cache_path: Path, size: int) -> None:
    if cache_path.exists():
        return
    async with _thumb_sem:
        if not cache_path.exists():
            await run_in_threadpool(_build_thumbnail, file_path, cache_path, size)

router = APIRouter(prefix="/api/files", tags=["files"])


def _inline_response(path: Path, media_type: str) -> FileResponse:
    return FileResponse(
        path=str(path),
        media_type=media_type,
        headers={"Content-Disposition": f'inline; filename="{path.name}"'},
    )


@router.get("/list")
async def list_directory(
    request: Request,
    path: str = Query(default=""),
    location: str = Query(default="external"),
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    _check_access(location, path, user["email"], user["is_admin"])
    base = _base(location)
    dir_path = _resolve(base, path)

    def _check_and_list() -> list[dict]:
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
        return _list_entries(dir_path)

    entries = await run_in_threadpool(_check_and_list)

    dir_names = [e["name"] for e in entries if e["type"] == "directory"]
    if dir_names:
        prefix = (path.rstrip("/") + "/") if path else ""
        paths_to_check = [f"{prefix}{name}" for name in dir_names]
        icons = db.exec(
            select(FolderIcon).where(
                FolderIcon.owner_email == user["email"],
                FolderIcon.location == location,
                FolderIcon.path.in_(paths_to_check),
            )
        ).all()
        icon_map = {i.path: i.icon_name for i in icons}
        for e in entries:
            if e["type"] == "directory":
                full_path = f"{prefix}{e['name']}"
                if full_path in icon_map:
                    e["icon_name"] = icon_map[full_path]

    existing_names = {e["name"] for e in entries}
    pending_uploads = db.exec(
        select(PendingUpload).where(
            PendingUpload.owner_email == user["email"],
            PendingUpload.location == location,
            PendingUpload.path == path,
        )
    ).all()
    for p in pending_uploads:
        if p.filename not in existing_names:
            entries.append({
                "name": p.filename,
                "type": "upload-pending",
                "size": p.file_size,
                "modified": p.created_at.timestamp(),
                "upload_id": p.upload_id,
                "bytes_received": p.bytes_received,
            })

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

    # Auto-create the destination directory (within the caller's already-checked-access subtree)
    # rather than 404-ing: a photo backup uploading into a fresh photos/<email>/YYYY/MM folder
    # has no other way to create it first without an extra, redundant round trip.
    dir_path.mkdir(parents=True, exist_ok=True)
    if not dir_path.is_dir():
        raise HTTPException(400, "Path is not a directory")

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


def _write_chunk_at_offset(part_file: Path, offset: int, data: bytes) -> None:
    # "r+b" requires the file to already exist; create it empty first if this is chunk 0 (or any
    # chunk arriving before chunk 0, since chunks may arrive out of order over parallel connections).
    if not part_file.exists():
        part_file.touch()
    with open(part_file, "r+b") as f:
        f.seek(offset)
        f.write(data)


@router.post("/upload-chunk")
async def upload_chunk(
    request: Request,
    upload_id: str = Query(...),
    chunk_index: int = Query(...),
    total_chunks: int = Query(...),
    chunk_size: int = Query(...),
    filename: str = Query(...),
    file_size: int = Query(...),
    path: str = Query(default=""),
    location: str = Query(default="external"),
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    _check_access(location, path, user["email"], user["is_admin"])
    base = _base(location)
    dir_path = _resolve(base, path)

    dir_path.mkdir(parents=True, exist_ok=True)
    if not dir_path.is_dir():
        raise HTTPException(400, "Path is not a directory")

    safe_name = Path(filename).name
    if not safe_name:
        raise HTTPException(400, "Invalid filename")
    if chunk_index < 0 or chunk_index >= total_chunks or chunk_size <= 0:
        raise HTTPException(400, "Invalid chunk parameters")

    dest = (dir_path / safe_name).resolve()
    if not str(dest).startswith(str(dir_path.resolve())):
        raise HTTPException(400, "Invalid path")

    # Read the request body before taking the lock - streaming it in is the slow part and
    # doesn't need to be serialized against other chunks of the same upload.
    body = b"".join([chunk async for chunk in request.stream()])
    if len(body) == 0:
        raise HTTPException(400, "Empty chunk")

    offset = chunk_index * chunk_size
    if offset + len(body) > file_size:
        raise HTTPException(400, "Chunk extends past declared file_size")

    part_file = dir_path / f".{upload_id}.part"
    # Writing at an explicit offset makes a retried or out-of-order chunk idempotent (it
    # overwrites the same bytes at the same place) instead of corrupting the file the way
    # append-only writes did. Done outside the lock below since it's the slow part and doesn't
    # touch any shared state the lock protects.
    await run_in_threadpool(_write_chunk_at_offset, part_file, offset, body)

    lock = _lock_for_upload(upload_id)
    # The lock only wraps this fast, synchronous bookkeeping (no `await` inside it) - holding a
    # plain threading.Lock across an `await` would block the whole asyncio event loop, not just
    # the other task waiting on it.
    with lock:
        pending = db.get(PendingUpload, upload_id)
        if pending is None:
            pending = PendingUpload(
                upload_id=upload_id,
                filename=safe_name,
                path=path,
                location=location,
                owner_email=user["email"],
                file_size=file_size,
                chunk_size=chunk_size,
                total_chunks=total_chunks,
            )
        received = set(json.loads(pending.received_chunks)) if pending.received_chunks else set()
        received.add(chunk_index)
        pending.received_chunks = json.dumps(sorted(received))
        pending.bytes_received = min(len(received) * chunk_size, file_size)
        db.add(pending)
        db.commit()
        # Completion is driven by having every chunk index, not by "did we just see the last
        # index" - chunks can legitimately arrive out of order (parallel connections, retries).
        is_complete = len(received) >= total_chunks

    if not is_complete:
        return {"status": "ok", "received_chunks": len(received), "total_chunks": total_chunks}

    actual_size = part_file.stat().st_size
    if actual_size != file_size:
        raise HTTPException(409, "Upload incomplete: assembled size does not match file_size")
    await run_in_threadpool(lambda: part_file.replace(dest))
    with lock:
        db.delete(pending)
        db.commit()
    _release_upload_lock(upload_id)
    return {"status": "complete", "filename": safe_name}


@router.get("/pending-uploads")
async def list_pending_uploads(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    pending = db.exec(
        select(PendingUpload).where(PendingUpload.owner_email == user["email"])
    ).all()
    return [
        {
            "upload_id": p.upload_id,
            "filename": p.filename,
            "path": p.path,
            "location": p.location,
            "file_size": p.file_size,
            "bytes_received": p.bytes_received,
            "total_chunks": p.total_chunks,
        }
        for p in pending
    ]


@router.delete("/cancel-upload/{upload_id}")
async def cancel_upload(upload_id: str, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    pending = db.get(PendingUpload, upload_id)
    if not pending or pending.owner_email != user["email"]:
        raise HTTPException(404, "Upload not found")
    base = _base(pending.location)
    dir_path = _resolve(base, pending.path)
    part_file = dir_path / f".{upload_id}.part"
    if part_file.exists():
        await run_in_threadpool(part_file.unlink)
    db.delete(pending)
    db.commit()
    _release_upload_lock(upload_id)
    return {"status": "cancelled"}


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


@router.get("/open/{token}/info")
async def get_file_token_info(
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
    _check_access(file_token.location, file_token.path, user["email"], user["is_admin"])
    base = _base(file_token.location)
    file_path = _resolve(base, file_token.path)
    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found")
    return {"filename": file_path.name, "content_type": _guess_media_type(file_path)}


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
    media_type = _guess_media_type(file_path)
    return _inline_response(file_path, media_type)


@router.get("/thumbnail")
async def thumbnail_file(
    request: Request,
    path: str = Query(...),
    location: str = Query(default="external"),
    size: int = Query(default=160),
):
    user = await get_current_user(request)
    _check_access(location, path, user["email"], user["is_admin"])
    base = _base(location)
    file_path = _resolve(base, path)

    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found")

    media_type = _guess_media_type(file_path)
    if not media_type.startswith("image/"):
        return _inline_response(file_path, media_type)

    size = max(40, min(size, 400))
    _THUMB_DIR.mkdir(parents=True, exist_ok=True)
    cache_path = _thumb_cache_path(file_path, size)
    try:
        # Always wait for (and serve) the actual resized thumbnail - never fall back to the full
        # original, since that's precisely the multi-MB download per photo this endpoint exists
        # to avoid for a device populating a gallery grid.
        await _ensure_thumbnail(file_path, cache_path, size)
        return FileResponse(
            path=str(cache_path),
            media_type="image/jpeg",
            headers={"Cache-Control": "max-age=604800", "Content-Disposition": "inline"},
        )
    except Exception:
        raise HTTPException(500, "Failed to build thumbnail")


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

    media_type = _guess_media_type(file_path)
    return _inline_response(file_path, media_type)


@router.get("/disk")
async def disk_usage(request: Request):
    import shutil
    await get_current_user(request)
    usage = await run_in_threadpool(shutil.disk_usage, str(_base("external")))
    return {"total": usage.total, "used": usage.used, "free": usage.free}


class IconBody(BaseModel):
    path: str
    location: str = "external"
    icon_name: str


@router.put("/icon")
async def set_folder_icon(
    request: Request,
    body: IconBody,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    _check_access(body.location, body.path, user["email"], user["is_admin"])
    existing = db.exec(
        select(FolderIcon).where(
            FolderIcon.owner_email == user["email"],
            FolderIcon.location == body.location,
            FolderIcon.path == body.path,
        )
    ).first()
    if existing:
        existing.icon_name = body.icon_name
        db.add(existing)
    else:
        db.add(FolderIcon(
            owner_email=user["email"],
            location=body.location,
            path=body.path,
            icon_name=body.icon_name,
        ))
    db.commit()
    return {"ok": True}


@router.delete("/icon")
async def delete_folder_icon(
    request: Request,
    path: str = Query(...),
    location: str = Query(default="external"),
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    existing = db.exec(
        select(FolderIcon).where(
            FolderIcon.owner_email == user["email"],
            FolderIcon.location == location,
            FolderIcon.path == path,
        )
    ).first()
    if existing:
        db.delete(existing)
        db.commit()
    return {"ok": True}


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
        path=str(file_path),
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
