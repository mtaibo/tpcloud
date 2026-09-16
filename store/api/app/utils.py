import os
from pathlib import Path

from fastapi import HTTPException

EXTERNAL_BASE = Path(os.getenv("EXTERNAL_DISK_PATH", "/mnt/external"))
SERVER_BASE = Path("/mnt/server")


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
    raise HTTPException(403, "Access denied")
