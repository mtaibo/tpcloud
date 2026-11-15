"""The file browser's Favourites: folders a user pinned, stored per account so the web on every
browser and the app all show the same list. Paths stay as they were saved; a favourite whose
folder was renamed, moved or deleted is still listed with `exists: false` so it can be removed."""

from typing import List

from fastapi import APIRouter, Depends, Query, Request
from pydantic import BaseModel
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import FolderIcon, StoreFavorite
from app.utils import check_access, get_base, resolve_path

router = APIRouter(prefix="/api/favorites", tags=["favorites"])

MAX_FAVORITES = 200


def _clean(path: str) -> str:
    return path.strip("/")


def _list(db: Session, email: str) -> list[dict]:
    favorites = db.exec(
        select(StoreFavorite).where(StoreFavorite.owner_email == email).order_by(StoreFavorite.id)
    ).all()
    icons = {
        (i.location, i.path): i.icon_name
        for i in db.exec(select(FolderIcon).where(FolderIcon.owner_email == email)).all()
    }
    out = []
    for f in favorites:
        try:
            exists = resolve_path(get_base(f.location), f.path).is_dir()
        except Exception:
            exists = False
        out.append({
            "id": f"{f.location}:{f.path}",
            "location": f.location,
            "path": f.path,
            "label": f.path.rsplit("/", 1)[-1],
            "icon_name": icons.get((f.location, f.path)),
            "exists": exists,
        })
    return out


@router.get("")
async def list_favorites(request: Request, db: Session = Depends(get_session)):
    """In the order they were added."""
    user = await get_current_user(request)
    return _list(db, user["email"])


class FavoriteItem(BaseModel):
    path: str
    location: str = "external"


class FavoriteAdd(BaseModel):
    items: List[FavoriteItem]


@router.post("")
async def add_favorites(body: FavoriteAdd, request: Request, db: Session = Depends(get_session)):
    """Adds one or several folders (several when a device hands over the list it used to keep on
    its own). Ones already there are skipped. Answers with the whole list and which were new."""
    user = await get_current_user(request)
    email = user["email"]
    existing = {
        (f.location, f.path)
        for f in db.exec(select(StoreFavorite).where(StoreFavorite.owner_email == email)).all()
    }
    added = []
    for item in body.items[:MAX_FAVORITES]:
        path = _clean(item.path)
        check_access(item.location, path, email)
        key = (item.location, path)
        if key in existing or len(existing) >= MAX_FAVORITES:
            continue
        if not resolve_path(get_base(item.location), path).is_dir():
            continue
        db.add(StoreFavorite(owner_email=email, location=item.location, path=path))
        existing.add(key)
        added.append(f"{item.location}:{path}")
    db.commit()
    return {"added": added, "favorites": _list(db, email)}


@router.delete("")
async def remove_favorite(
    request: Request,
    path: str = Query(...),
    location: str = Query(default="external"),
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    favorite = db.exec(
        select(StoreFavorite).where(
            StoreFavorite.owner_email == user["email"],
            StoreFavorite.location == location,
            StoreFavorite.path == _clean(path),
        )
    ).first()
    if favorite:
        db.delete(favorite)
        db.commit()
    return {"favorites": _list(db, user["email"])}
