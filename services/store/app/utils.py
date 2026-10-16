import mimetypes
import os
from pathlib import Path

from fastapi import HTTPException

EXTERNAL_BASE = Path(os.getenv("EXTERNAL_DISK_PATH", "/mnt/external"))
SERVER_BASE = Path("/mnt/server")

# mimetypes.guess_type() depends on mime.types files that may or may not be present on
# the host/container (e.g. python:3.12-slim has none), so common photo formats like HEIC/HEIF
# - the default capture format on iPhone - can silently fail to resolve there even though they
# resolve fine on a dev machine that happens to have an old Apache mime.types lying around.
# Keep the extensions this app actually needs to recognize as images independent of that.
_IMAGE_EXTENSIONS = {
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".png": "image/png",
    ".gif": "image/gif",
    ".webp": "image/webp",
    ".bmp": "image/bmp",
    ".tif": "image/tiff",
    ".tiff": "image/tiff",
    ".heic": "image/heic",
    ".heif": "image/heif",
}


def guess_media_type(path: Path) -> str:
    media_type = _IMAGE_EXTENSIONS.get(path.suffix.lower())
    if media_type:
        return media_type
    media_type, _ = mimetypes.guess_type(str(path))
    return media_type or "application/octet-stream"


def get_base(location: str) -> Path:
    if location == "external":
        return EXTERNAL_BASE
    if location == "server":
        return SERVER_BASE
    raise HTTPException(400, "Invalid location. Use 'external' or 'server'.")


def resolve_path(base: Path, user_path: str) -> Path:
    clean = user_path.lstrip("/")
    target = (base / clean).resolve() if clean else base.resolve()
    if not str(target).startswith(str(base.resolve())):
        raise HTTPException(400, "Invalid path")
    return target


def check_access(location: str, path: str, email: str, is_admin: bool):
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
    if first == "photos" and len(parts) >= 2 and parts[1] == email:
        return
    raise HTTPException(403, "Access denied")


def list_directory_entries(dir_path: Path) -> list[dict]:
    try:
        items = sorted(dir_path.iterdir(), key=lambda x: (x.is_file(), x.name.lower()))
    except PermissionError:
        raise HTTPException(403, "Permission denied")
    entries = []
    for item in items:
        if item.name.startswith('.'):
            continue
        try:
            stat = item.stat()
            entries.append({
                "name": item.name,
                "type": "file" if item.is_file() else "directory",
                "size": stat.st_size if item.is_file() else None,
                "modified": stat.st_mtime,
            })
        except (PermissionError, OSError):
            continue
    return entries
