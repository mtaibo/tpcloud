import hashlib
from datetime import datetime, timezone
from pathlib import Path
from typing import List, Optional

from fastapi import APIRouter, Depends, HTTPException, Query, Request
from fastapi.concurrency import run_in_threadpool
from pydantic import BaseModel
from sqlalchemy import and_, or_
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import Photo
from app.utils import get_base, resolve_path

router = APIRouter(prefix="/api/photos", tags=["photos"])


def _check_photo_access(location: str, path: str, email: str) -> None:
    if location != "external":
        raise HTTPException(403, "Access denied")
    clean = path.lstrip("/")
    parts = Path(clean).parts
    if len(parts) < 2 or parts[0] != "users" or parts[1] != email:
        raise HTTPException(403, "Access denied")


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(1024 * 1024):
            h.update(chunk)
    return h.hexdigest()


def _parse_iso(value: str) -> Optional[datetime]:
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None


def _serialize(photo: Photo) -> dict:
    return {
        "id": photo.id,
        "path": photo.path,
        "location": photo.location,
        "checksum": photo.checksum,
        "source_asset_id": photo.source_asset_id,
        "original_filename": photo.original_filename,
        "file_size": photo.file_size,
        "mime_type": photo.mime_type,
        "width": photo.width,
        "height": photo.height,
        "taken_at": photo.taken_at.isoformat() if photo.taken_at else None,
        "is_screenshot": photo.is_screenshot,
        "is_camera_original": photo.is_camera_original,
        "source_type": photo.source_type,
        "camera_make": photo.camera_make,
        "camera_model": photo.camera_model,
        "gps_lat": photo.gps_lat,
        "gps_lon": photo.gps_lon,
        "status": photo.status,
        "uploaded_at": photo.uploaded_at.isoformat(),
        "verified_at": photo.verified_at.isoformat() if photo.verified_at else None,
    }


class PhotoCreateBody(BaseModel):
    path: str
    location: str = "external"
    checksum: str
    original_filename: str
    source_asset_id: Optional[str] = None
    file_size: int
    mime_type: Optional[str] = None
    width: Optional[int] = None
    height: Optional[int] = None
    taken_at: Optional[str] = None
    is_screenshot: bool = False
    is_camera_original: bool = False
    source_type: str = "unknown"
    camera_make: Optional[str] = None
    camera_model: Optional[str] = None
    gps_lat: Optional[float] = None
    gps_lon: Optional[float] = None


class CheckExistingBody(BaseModel):
    source_asset_ids: List[str]


@router.post("")
async def register_photo(
    request: Request,
    body: PhotoCreateBody,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    email = user["email"]
    _check_photo_access(body.location, body.path, email)

    base = get_base(body.location)
    file_path = resolve_path(base, body.path)

    existing = db.exec(
        select(Photo).where(Photo.owner_email == email, Photo.checksum == body.checksum)
    ).first()
    if existing:
        existing_path = resolve_path(base, existing.path)
        if file_path.exists() and file_path.resolve() != existing_path.resolve():
            await run_in_threadpool(file_path.unlink)
        return _serialize(existing)

    if not file_path.exists() or not file_path.is_file():
        raise HTTPException(404, "File not found - upload it via /api/files/upload first")

    actual_checksum = await run_in_threadpool(_sha256_file, file_path)
    verified = actual_checksum == body.checksum

    photo = Photo(
        owner_email=email,
        checksum=body.checksum,
        source_asset_id=body.source_asset_id,
        path=body.path,
        location=body.location,
        original_filename=body.original_filename,
        file_size=body.file_size,
        mime_type=body.mime_type,
        width=body.width,
        height=body.height,
        taken_at=_parse_iso(body.taken_at) if body.taken_at else None,
        is_screenshot=body.is_screenshot,
        is_camera_original=body.is_camera_original,
        source_type=body.source_type,
        camera_make=body.camera_make,
        camera_model=body.camera_model,
        gps_lat=body.gps_lat,
        gps_lon=body.gps_lon,
        status="verified" if verified else "checksum_mismatch",
        verified_at=datetime.now(timezone.utc) if verified else None,
    )
    db.add(photo)
    db.commit()
    db.refresh(photo)
    return _serialize(photo)


@router.post("/check-existing")
async def check_existing(
    request: Request,
    body: CheckExistingBody,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    if not body.source_asset_ids:
        return {"known_asset_ids": []}
    rows = db.exec(
        select(Photo.source_asset_id).where(
            Photo.owner_email == user["email"],
            Photo.source_asset_id.in_(body.source_asset_ids),
        )
    ).all()
    return {"known_asset_ids": [r for r in rows if r]}


@router.get("")
async def list_photos(
    request: Request,
    filter: str = Query(default="all"),
    from_date: Optional[str] = Query(default=None),
    to_date: Optional[str] = Query(default=None),
    cursor: Optional[str] = Query(default=None),
    limit: int = Query(default=100),
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    limit = max(1, min(limit, 200))

    stmt = select(Photo).where(Photo.owner_email == user["email"])

    if filter == "screenshots":
        stmt = stmt.where(Photo.is_screenshot == True)  # noqa: E712
    elif filter == "camera":
        stmt = stmt.where(Photo.is_camera_original == True)  # noqa: E712
    elif filter == "shared":
        stmt = stmt.where(Photo.source_type == "cloud_shared")
    elif filter != "all":
        raise HTTPException(400, "Invalid filter")

    if from_date:
        parsed = _parse_iso(from_date)
        if parsed is None:
            raise HTTPException(400, "Invalid from_date")
        stmt = stmt.where(Photo.taken_at >= parsed)
    if to_date:
        parsed = _parse_iso(to_date)
        if parsed is None:
            raise HTTPException(400, "Invalid to_date")
        stmt = stmt.where(Photo.taken_at <= parsed)

    if cursor:
        try:
            taken_at_str, id_str = cursor.rsplit("|", 1)
            cursor_id = int(id_str)
        except ValueError:
            raise HTTPException(400, "Invalid cursor")
        if taken_at_str:
            cursor_taken_at = _parse_iso(taken_at_str)
            if cursor_taken_at is None:
                raise HTTPException(400, "Invalid cursor")
            stmt = stmt.where(
                or_(
                    Photo.taken_at < cursor_taken_at,
                    and_(Photo.taken_at == cursor_taken_at, Photo.id < cursor_id),
                )
            )
        else:
            stmt = stmt.where(Photo.taken_at.is_(None), Photo.id < cursor_id)

    stmt = stmt.order_by(Photo.taken_at.desc(), Photo.id.desc()).limit(limit + 1)
    rows = db.exec(stmt).all()

    has_more = len(rows) > limit
    rows = rows[:limit]
    next_cursor = None
    if has_more and rows:
        last = rows[-1]
        next_cursor = f"{last.taken_at.isoformat() if last.taken_at else ''}|{last.id}"

    return {"items": [_serialize(p) for p in rows], "next_cursor": next_cursor}


@router.get("/{photo_id}")
async def get_photo(
    photo_id: int,
    request: Request,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    photo = db.get(Photo, photo_id)
    if not photo or photo.owner_email != user["email"]:
        raise HTTPException(404, "Photo not found")
    return _serialize(photo)


@router.delete("/{photo_id}")
async def delete_photo(
    photo_id: int,
    request: Request,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    photo = db.get(Photo, photo_id)
    if not photo or photo.owner_email != user["email"]:
        raise HTTPException(404, "Photo not found")

    base = get_base(photo.location)
    file_path = resolve_path(base, photo.path)
    if file_path.exists():
        await run_in_threadpool(file_path.unlink)

    db.delete(photo)
    db.commit()
    return {"deleted": photo_id}
