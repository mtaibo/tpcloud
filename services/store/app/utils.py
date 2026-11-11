import mimetypes
import os
from pathlib import Path, PurePosixPath

from fastapi import HTTPException
from fastapi.responses import FileResponse

EXTERNAL_BASE = Path(os.getenv("EXTERNAL_DISK_PATH", "/mnt/external"))

# Where tpcloud is served from; share links are <this>/share/<token>.
PUBLIC_URL = os.getenv("TPCLOUD_URL", "").rstrip("/")

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


# Anyone with an editable share link can upload files, and they're served from the same origin as
# everything else, so a file shown in the browser must never run scripts there: the sandbox gives
# it an opaque origin of its own with scripts off. PDFs are left out because browsers refuse to
# show them sandboxed, and their viewer doesn't run page scripts anyway.
_SANDBOX = "sandbox; default-src 'none'; img-src 'self' data: blob:; media-src 'self' blob:; style-src 'unsafe-inline'"


def file_headers(media_type: str) -> dict[str, str]:
    return {} if media_type == "application/pdf" else {"Content-Security-Policy": _SANDBOX}


def inline_response(path: Path, media_type: str, headers: dict[str, str] | None = None) -> FileResponse:
    """A user's file, to be shown in the browser rather than downloaded."""
    return FileResponse(
        path=str(path),
        media_type=media_type,
        filename=path.name,
        content_disposition_type="inline",
        headers={**file_headers(media_type), **(headers or {})},
    )


def get_base(location: str) -> Path:
    # The Pi's own filesystem ("server") used to be browsable here by admins; it no longer is.
    if location == "external":
        return EXTERNAL_BASE
    raise HTTPException(400, "Invalid location. Use 'external'.")


def resolve_path(base: Path, user_path: str) -> Path:
    clean = user_path.lstrip("/")
    root = base.resolve()
    target = (root / clean).resolve() if clean else root
    # A real containment check: comparing the strings let "/disk/shared/a" + "../ab" through,
    # since "/disk/shared/ab" starts with "/disk/shared/a".
    if not target.is_relative_to(root):
        raise HTTPException(400, "Invalid path")
    return target


def check_access(location: str, path: str, email: str, allow_photos: bool = False):
    """Everyone, admins included, gets only their own folder and the shared one: neither the
    disk root nor the Pi's own filesystem can be browsed anymore. `photos/<email>` is reachable
    only from the endpoints the Photos section needs (upload, thumbnail, view), never from the
    file browser."""
    if location != "external":
        raise HTTPException(403, "Access denied")
    clean = path.lstrip("/")
    if not clean:
        raise HTTPException(403, "Access denied")
    parts = PurePosixPath(clean).parts
    # Checked before the path is resolved, so ".." would have let "users/<me>/../<someone else>"
    # or "shared/../users/<someone>" past it into another user's folder.
    if any(p in ("..", ".") for p in parts):
        raise HTTPException(403, "Access denied")
    first = parts[0]
    if first == "shared":
        return
    if first == "users" and len(parts) >= 2 and parts[1] == email:
        return
    if allow_photos and first == "photos" and len(parts) >= 2 and parts[1] == email:
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
