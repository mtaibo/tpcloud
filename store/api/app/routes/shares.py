import re
import secrets
from typing import Optional

import bcrypt
from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import BaseModel
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import ShareLink
from app.utils import check_access

router = APIRouter(prefix="/api/shares", tags=["shares"])

SLUG_RE = re.compile(r'^[a-zA-Z0-9_-]{1,64}$')


def _share_url(token: str) -> str:
    return f"https://share.migueltaibo.com/{token}"


def _serialize(share: ShareLink) -> dict:
    return {
        "token": share.token,
        "url": _share_url(share.token),
        "location": share.location,
        "path": share.path,
        "hidden": share.hidden,
        "has_password": share.password_hash is not None,
        "editable": share.editable,
        "created_at": share.created_at.isoformat(),
    }


class CreateShareBody(BaseModel):
    location: str
    path: str
    hidden: bool = False
    slug: Optional[str] = None
    password: Optional[str] = None
    editable: bool = False


class UpdateShareBody(BaseModel):
    password: Optional[str] = None
    clear_password: bool = False
    editable: Optional[bool] = None


@router.get("")
async def list_shares(
    request: Request,
    path: Optional[str] = None,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    stmt = select(ShareLink).where(ShareLink.owner_email == user["email"])
    if path is not None:
        stmt = stmt.where(ShareLink.path == path)
    shares = db.exec(stmt).all()
    return [_serialize(s) for s in shares]


@router.post("")
async def create_share(
    request: Request,
    body: CreateShareBody,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    check_access(body.location, body.path, user["email"], user["is_admin"])

    if body.hidden or not body.slug:
        token = secrets.token_urlsafe(8)
        while db.exec(select(ShareLink).where(ShareLink.token == token)).first():
            token = secrets.token_urlsafe(8)
    else:
        token = body.slug.strip()
        if not SLUG_RE.match(token):
            raise HTTPException(400, "Slug must be 1–64 alphanumeric/dash/underscore characters")
        if db.exec(select(ShareLink).where(ShareLink.token == token)).first():
            raise HTTPException(409, "Slug already in use")

    password_hash = None
    if body.password:
        password_hash = bcrypt.hashpw(body.password.encode(), bcrypt.gensalt()).decode()

    share = ShareLink(
        token=token,
        location=body.location,
        path=body.path.strip("/"),
        owner_email=user["email"],
        hidden=body.hidden or not body.slug,
        password_hash=password_hash,
        editable=body.editable,
    )
    db.add(share)
    db.commit()
    db.refresh(share)
    return _serialize(share)


@router.patch("/{token}")
async def update_share(
    token: str,
    request: Request,
    body: UpdateShareBody,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    share = db.exec(select(ShareLink).where(ShareLink.token == token)).first()
    if not share or share.owner_email != user["email"]:
        raise HTTPException(404, "Share not found")

    if body.clear_password:
        share.password_hash = None
    elif body.password:
        share.password_hash = bcrypt.hashpw(body.password.encode(), bcrypt.gensalt()).decode()

    if body.editable is not None:
        share.editable = body.editable

    db.add(share)
    db.commit()
    db.refresh(share)
    return _serialize(share)


@router.delete("/{token}")
async def delete_share(
    token: str,
    request: Request,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    share = db.exec(select(ShareLink).where(ShareLink.token == token)).first()
    if not share or share.owner_email != user["email"]:
        raise HTTPException(404, "Share not found")
    db.delete(share)
    db.commit()
    return {"deleted": token}
