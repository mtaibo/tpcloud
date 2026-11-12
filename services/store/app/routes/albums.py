"""Albums of backed-up photos. Every user has Favoritos (kind "favorites"); albums of their own
can be added on top later through the same endpoints."""

from fastapi import APIRouter, Depends, HTTPException, Request
from sqlalchemy import func
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import Album, AlbumPhoto, Photo

router = APIRouter(prefix="/api/albums", tags=["albums"])

FAVORITES_NAME = "Favoritos"


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


def _own_photo(db: Session, photo_id: int, email: str) -> Photo:
    photo = db.get(Photo, photo_id)
    if not photo or photo.owner_email != email:
        raise HTTPException(404, "Photo not found")
    return photo


@router.get("")
async def list_albums(request: Request, db: Session = Depends(get_session)):
    """Favoritos first, then the user's own by name, each with how many photos it has and the
    newest one as its cover."""
    user = await get_current_user(request)
    favorites_album(db, user["email"])
    albums = db.exec(select(Album).where(Album.owner_email == user["email"])).all()
    albums.sort(key=lambda a: (a.kind != "favorites", a.name.casefold()))
    result = []
    for album in albums:
        count = db.exec(select(func.count()).select_from(AlbumPhoto).where(AlbumPhoto.album_id == album.id)).one()
        cover = db.exec(
            select(Photo)
            .join(AlbumPhoto, AlbumPhoto.photo_id == Photo.id)
            .where(AlbumPhoto.album_id == album.id)
            .order_by(Photo.taken_at.desc(), Photo.id.desc())
            .limit(1)
        ).first()
        result.append({
            "id": album.id,
            "name": album.name,
            "kind": album.kind,
            "count": count,
            "cover": {"path": cover.path, "checksum": cover.checksum} if cover else None,
        })
    return result


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
