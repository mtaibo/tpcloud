import mimetypes
from datetime import datetime, timezone, timedelta
from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import FileResponse
from sqlmodel import Session, select

from app.database import get_session
from app.models import ShareFileToken, ShareLink
from app.utils import get_base, resolve_path

router = APIRouter(prefix="/api/open", tags=["open"])

TOKEN_TTL_HOURS = 24


def _inline_response(path: Path, media_type: str) -> FileResponse:
    return FileResponse(
        path=str(path),
        media_type=media_type,
        headers={"Content-Disposition": f'inline; filename="{path.name}"'},
    )


def _resolve_token(token_str: str, db: Session):
    rec = db.exec(select(ShareFileToken).where(ShareFileToken.token == token_str)).first()
    if not rec:
        raise HTTPException(404, "Token not found")
    if rec.first_accessed_at:
        expiry = rec.first_accessed_at.replace(tzinfo=timezone.utc) + timedelta(hours=TOKEN_TTL_HOURS)
        if datetime.now(timezone.utc) > expiry:
            raise HTTPException(410, "Token expired")
    share = db.exec(select(ShareLink).where(ShareLink.token == rec.share_token)).first()
    if not share:
        raise HTTPException(404, "Share not found")
    if share.password_hash and not rec.session_token:
        raise HTTPException(401, "Password required")
    base = get_base(share.location)
    share_root = (base / share.path).resolve()
    file_path = resolve_path(share_root, rec.path)
    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found")
    return rec, file_path


@router.get("/{token}/info")
def get_share_file_info(token: str, db: Session = Depends(get_session)):
    _, file_path = _resolve_token(token, db)
    media_type, _ = mimetypes.guess_type(str(file_path))
    return {"filename": file_path.name, "content_type": media_type or "application/octet-stream"}


@router.get("/{token}")
def open_share_file(token: str, db: Session = Depends(get_session)):
    rec, file_path = _resolve_token(token, db)
    if not rec.first_accessed_at:
        rec.first_accessed_at = datetime.now(timezone.utc)
        db.add(rec)
        db.commit()
    media_type, _ = mimetypes.guess_type(str(file_path))
    return _inline_response(file_path, media_type or "application/octet-stream")
