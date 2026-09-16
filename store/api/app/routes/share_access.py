import io
import mimetypes
import os
import secrets
import shutil
import zipfile
from datetime import datetime, timezone, timedelta
from pathlib import Path
from typing import List, Optional

import aiofiles
import bcrypt
from fastapi import APIRouter, Depends, HTTPException, Query, Request, UploadFile, File
from fastapi.responses import FileResponse, StreamingResponse
from pydantic import BaseModel
from sqlmodel import Session, select

from app.database import get_session
from app.models import ShareLink, ShareSession
from app.utils import get_base, resolve_path

router = APIRouter(prefix="/api/share", tags=["share-access"])

SESSION_TTL_HOURS = 24


def _get_share(token: str, db: Session) -> ShareLink:
    share = db.exec(select(ShareLink).where(ShareLink.token == token)).first()
    if not share:
        raise HTTPException(404, "Share not found")
    return share


def _verify_session(share: ShareLink, request: Request, db: Session):
    if not share.password_hash:
        return
    session_token = request.headers.get("X-Share-Session")
    if not session_token:
        raise HTTPException(401, "Password required")
    sess = db.exec(
        select(ShareSession)
        .where(ShareSession.session_token == session_token)
        .where(ShareSession.share_token == share.token)
    ).first()
    if not sess or sess.expires_at < datetime.now(timezone.utc):
        raise HTTPException(401, "Session expired or invalid")


def _require_editable(share: ShareLink):
    if not share.editable:
        raise HTTPException(403, "This share is read-only")


def _share_resolve(share: ShareLink, subpath: str) -> Path:
    base = get_base(share.location)
    share_root = (base / share.path).resolve()
    if not str(share_root).startswith(str(base.resolve())):
        raise HTTPException(400, "Invalid share path")
    return resolve_path(share_root, subpath)


# ── Metadata & auth ──────────────────────────────────────────────────────────

@router.get("/{token}")
def get_share_info(token: str, db: Session = Depends(get_session)):
    share = _get_share(token, db)
    return {
        "token": share.token,
        "label": share.path.split("/")[-1] or share.token,
        "has_password": share.password_hash is not None,
        "editable": share.editable,
    }


class AuthBody(BaseModel):
    password: str


@router.post("/{token}/auth")
def authenticate_share(token: str, body: AuthBody, db: Session = Depends(get_session)):
    share = _get_share(token, db)
    if not share.password_hash:
        raise HTTPException(400, "Share has no password")
    if not bcrypt.checkpw(body.password.encode(), share.password_hash.encode()):
        raise HTTPException(401, "Incorrect password")

    session_token = secrets.token_urlsafe(32)
    expires_at = datetime.now(timezone.utc) + timedelta(hours=SESSION_TTL_HOURS)
    sess = ShareSession(session_token=session_token, share_token=token, expires_at=expires_at)
    db.add(sess)
    db.commit()
    return {"session_token": session_token}


# ── File access ───────────────────────────────────────────────────────────────

@router.get("/{token}/files/list")
def list_files(
    token: str,
    request: Request,
    path: str = Query(default=""),
    db: Session = Depends(get_session),
):
    share = _get_share(token, db)
    _verify_session(share, request, db)
    dir_path = _share_resolve(share, path)

    if not dir_path.exists() or not dir_path.is_dir():
        raise HTTPException(404, "Directory not found")

    entries = []
    try:
        items = sorted(dir_path.iterdir(), key=lambda x: (x.is_file(), x.name.lower()))
    except PermissionError:
        raise HTTPException(403, "Permission denied")

    for item in items:
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

    return {"path": path, "entries": entries}


@router.get("/{token}/files/view")
def view_file(
    token: str,
    request: Request,
    path: str = Query(...),
    db: Session = Depends(get_session),
):
    share = _get_share(token, db)
    _verify_session(share, request, db)
    file_path = _share_resolve(share, path)

    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found")

    media_type, _ = mimetypes.guess_type(str(file_path))
    media_type = media_type or "application/octet-stream"
    return FileResponse(
        path=str(file_path),
        media_type=media_type,
        headers={"Content-Disposition": f'inline; filename="{file_path.name}"'},
    )


@router.get("/{token}/files/download")
def download_file(
    token: str,
    request: Request,
    path: str = Query(...),
    db: Session = Depends(get_session),
):
    share = _get_share(token, db)
    _verify_session(share, request, db)
    file_path = _share_resolve(share, path)

    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found")

    return FileResponse(path=file_path, filename=file_path.name, media_type="application/octet-stream")


@router.get("/{token}/files/download-zip")
def download_folder_zip(
    token: str,
    request: Request,
    path: str = Query(...),
    db: Session = Depends(get_session),
):
    share = _get_share(token, db)
    _verify_session(share, request, db)
    dir_path = _share_resolve(share, path)

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


# ── Write operations (editable shares only) ───────────────────────────────────

@router.post("/{token}/files/upload")
async def upload_files(
    token: str,
    request: Request,
    path: str = Query(default=""),
    files: List[UploadFile] = File(...),
    db: Session = Depends(get_session),
):
    share = _get_share(token, db)
    _verify_session(share, request, db)
    _require_editable(share)
    dir_path = _share_resolve(share, path)

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


class MkdirBody(BaseModel):
    path: str


@router.post("/{token}/files/mkdir")
def create_directory(
    token: str,
    request: Request,
    body: MkdirBody,
    db: Session = Depends(get_session),
):
    share = _get_share(token, db)
    _verify_session(share, request, db)
    _require_editable(share)
    new_dir = _share_resolve(share, body.path)

    if new_dir.exists():
        raise HTTPException(400, "Already exists")
    try:
        new_dir.mkdir(parents=True)
    except PermissionError:
        raise HTTPException(403, "Permission denied")
    return {"created": body.path}


class RenameBody(BaseModel):
    path: str
    new_name: str


@router.post("/{token}/files/rename")
def rename_item(
    token: str,
    request: Request,
    body: RenameBody,
    db: Session = Depends(get_session),
):
    share = _get_share(token, db)
    _verify_session(share, request, db)
    _require_editable(share)
    item = _share_resolve(share, body.path)

    if not item.exists():
        raise HTTPException(404, "Not found")
    new_name = Path(body.new_name).name
    if not new_name:
        raise HTTPException(400, "Invalid name")
    dest = item.parent / new_name
    if dest.exists():
        raise HTTPException(400, "Already exists")
    item.rename(dest)
    return {"renamed": body.path}


@router.delete("/{token}/files")
def delete_item(
    token: str,
    request: Request,
    path: str = Query(...),
    db: Session = Depends(get_session),
):
    share = _get_share(token, db)
    _verify_session(share, request, db)
    _require_editable(share)
    item = _share_resolve(share, path)

    if not item.exists():
        raise HTTPException(404, "Not found")
    if item.is_file() or item.is_symlink():
        item.unlink()
    else:
        shutil.rmtree(item)
    return {"deleted": path}
