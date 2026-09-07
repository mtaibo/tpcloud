import os
import shutil
from pathlib import Path
from typing import List

import aiofiles
from fastapi import APIRouter, HTTPException, Query, Request, UploadFile, File
from fastapi.responses import FileResponse
from pydantic import BaseModel

from app.auth import get_current_user

router = APIRouter(prefix="/api/files", tags=["files"])

EXTERNAL_BASE = Path(os.getenv("EXTERNAL_DISK_PATH", "/mnt/external"))
SERVER_BASE = Path("/mnt/server")


def _base(location: str) -> Path:
    if location == "external":
        return EXTERNAL_BASE
    if location == "server":
        return SERVER_BASE
    raise HTTPException(400, "Invalid location. Use 'external' or 'server'.")


def _resolve(base: Path, user_path: str) -> Path:
    clean = user_path.lstrip("/")
    target = (base / clean).resolve() if clean else base.resolve()
    if not str(target).startswith(str(base.resolve())):
        raise HTTPException(400, "Invalid path")
    return target


def _check_access(location: str, path: str, email: str, is_admin: bool):
    if is_admin:
        return
    if location == "server":
        raise HTTPException(403, "Admin access required")
    clean = path.lstrip("/")
    if not clean:
        raise HTTPException(403, "Access denied")
    parts = Path(clean).parts
    first = parts[0]
    if first == "shared":
        return
    if first == "users" and len(parts) >= 2 and parts[1] == email:
        return
    raise HTTPException(403, "Access denied")


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
