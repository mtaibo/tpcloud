"""Albums of backed-up photos. Every user has Favoritos (kind "favorites"), and can make albums of
their own, hide them (they move to a Hidden section) and take an album's photos out of the library
so they're only seen inside it."""

from typing import List, Optional

from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import BaseModel
from sqlalchemy import func
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import Album, AlbumPhoto, Photo

router = APIRouter(prefix="/api/albums", tags=["albums"])

FAVORITES_NAME = "Favoritos"
MAX_NAME = 120


def favorites_album(db: Session, email: str) -> Album:
    album = db.exec(select(Album).where(Album.owner_email == email, Album.kind == "favorites")).first()
    if album is None:
        album = Album(owner_email=email, name=FAVORITES_NAME, kind="favorites")
        db.add(album)
        db.commit()
        db.refresh(album)
    return album


def own_album(db: Session, album_id: int, email: str) -> Album:
    album = db.get(Album, album_id)
    if not album or album.owner_email != email:
        raise HTTPException(404, "Album not found")
    return album


def library_excluded(email: str):
    """The ids of a user's photos kept out of the library: those in any of their albums marked
    `hides_from_library`. A subquery, for `Photo.id.not_in(...)`."""
    return (
        select(AlbumPhoto.photo_id)
        .join(Album, Album.id == AlbumPhoto.album_id)
        .where(Album.owner_email == email, Album.hides_from_library == True)  # noqa: E712
    )


def _own_photo(db: Session, photo_id: int, email: str) -> Photo:
    photo = db.get(Photo, photo_id)
    if not photo or photo.owner_email != email:
        raise HTTPException(404, "Photo not found")
    return photo


def _clean_name(name: str) -> str:
    name = " ".join(name.split())
    if not name:
        raise HTTPException(400, "The album needs a name")
    return name[:MAX_NAME]


def _serialize(db: Session, album: Album) -> dict:
    count = db.exec(select(func.count()).select_from(AlbumPhoto).where(AlbumPhoto.album_id == album.id)).one()
    cover = db.exec(
        select(Photo)
        .join(AlbumPhoto, AlbumPhoto.photo_id == Photo.id)
        .where(AlbumPhoto.album_id == album.id)
        .order_by(Photo.taken_at.desc(), Photo.id.desc())
        .limit(1)
    ).first()
    return {
        "id": album.id,
        "name": album.name,
        "kind": album.kind,
        "count": count,
        "hidden": album.hidden,
        "hides_from_library": album.hides_from_library,
        "cover": {"id": cover.id, "path": cover.path, "checksum": cover.checksum} if cover else None,
    }


@router.get("")
async def list_albums(request: Request, db: Session = Depends(get_session)):
    """Favoritos first, then the user's own by name (hidden ones too, flagged), each with how many
    photos it has and the newest one as its cover."""
    user = await get_current_user(request)
    favorites_album(db, user["email"])
    albums = db.exec(select(Album).where(Album.owner_email == user["email"])).all()
    albums.sort(key=lambda a: (a.kind != "favorites", a.name.casefold()))
    return [_serialize(db, a) for a in albums]


class AlbumCreate(BaseModel):
    name: str
    photo_ids: List[int] = []


@router.post("")
async def create_album(body: AlbumCreate, request: Request, db: Session = Depends(get_session)):
    """A new album of the user's own, optionally with photos in it already."""
    user = await get_current_user(request)
    album = Album(owner_email=user["email"], name=_clean_name(body.name), kind="user")
    db.add(album)
    db.commit()
    db.refresh(album)
    if body.photo_ids:
        _add(db, album, body.photo_ids, user["email"])
    return _serialize(db, album)


class AlbumUpdate(BaseModel):
    name: Optional[str] = None
    hidden: Optional[bool] = None
    hides_from_library: Optional[bool] = None


@router.patch("/{album_id}")
async def update_album(album_id: int, body: AlbumUpdate, request: Request, db: Session = Depends(get_session)):
    """Renames, hides or shows it, or keeps its photos out of the library or lets them back in.
    Favoritos keeps its name and stays where it is."""
    user = await get_current_user(request)
    album = own_album(db, album_id, user["email"])
    if album.kind == "favorites" and (body.name is not None or body.hidden or body.hides_from_library):
        raise HTTPException(400, "Favoritos can't be changed")
    if body.name is not None:
        album.name = _clean_name(body.name)
    if body.hidden is not None:
        album.hidden = body.hidden
    if body.hides_from_library is not None:
        album.hides_from_library = body.hides_from_library
    db.add(album)
    db.commit()
    db.refresh(album)
    return _serialize(db, album)


@router.delete("/{album_id}")
async def delete_album(album_id: int, request: Request, db: Session = Depends(get_session)):
    """Deletes the album, not its photos (which go back to the library if it was keeping them out)."""
    user = await get_current_user(request)
    album = own_album(db, album_id, user["email"])
    if album.kind == "favorites":
        raise HTTPException(400, "Favoritos can't be deleted")
    for link in db.exec(select(AlbumPhoto).where(AlbumPhoto.album_id == album.id)).all():
        db.delete(link)
    db.delete(album)
    db.commit()
    return {"deleted": album_id}


class PhotoIds(BaseModel):
    ids: List[int]


def _add(db: Session, album: Album, ids: List[int], email: str) -> int:
    ids = list(dict.fromkeys(ids))
    owned = set(db.exec(select(Photo.id).where(Photo.id.in_(ids), Photo.owner_email == email)).all())
    present = set(db.exec(select(AlbumPhoto.photo_id).where(AlbumPhoto.album_id == album.id, AlbumPhoto.photo_id.in_(ids))).all())
    added = 0
    for photo_id in ids:
        if photo_id in owned and photo_id not in present:
            db.add(AlbumPhoto(album_id=album.id, photo_id=photo_id))
            added += 1
    db.commit()
    return added


@router.post("/{album_id}/photos")
async def add_photos(album_id: int, body: PhotoIds, request: Request, db: Session = Depends(get_session)):
    """Adds several photos at once (the app's selection)."""
    user = await get_current_user(request)
    album = own_album(db, album_id, user["email"])
    added = _add(db, album, body.ids, user["email"])
    return {**_serialize(db, album), "added": added}


@router.post("/{album_id}/photos/remove")
async def remove_photos(album_id: int, body: PhotoIds, request: Request, db: Session = Depends(get_session)):
    """Takes several photos out of the album (not off the server)."""
    user = await get_current_user(request)
    album = own_album(db, album_id, user["email"])
    for link in db.exec(select(AlbumPhoto).where(AlbumPhoto.album_id == album.id, AlbumPhoto.photo_id.in_(body.ids))).all():
        db.delete(link)
    db.commit()
    return _serialize(db, album)


@router.put("/{album_id}/photos/{photo_id}")
async def add_photo(album_id: int, photo_id: int, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    own_album(db, album_id, user["email"])
    _own_photo(db, photo_id, user["email"])
    if not db.get(AlbumPhoto, (album_id, photo_id)):
        db.add(AlbumPhoto(album_id=album_id, photo_id=photo_id))
        db.commit()
    return {"album_id": album_id, "photo_id": photo_id}


@router.delete("/{album_id}/photos/{photo_id}")
async def remove_photo(album_id: int, photo_id: int, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    own_album(db, album_id, user["email"])
    link = db.get(AlbumPhoto, (album_id, photo_id))
    if link:
        db.delete(link)
        db.commit()
    return {"album_id": album_id, "photo_id": photo_id}
