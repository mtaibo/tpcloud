import asyncio
import hashlib
import mimetypes
import threading
from datetime import datetime, timezone
from pathlib import Path
from typing import List, Optional

from fastapi import APIRouter, Depends, HTTPException, Query, Request
from fastapi.concurrency import run_in_threadpool
from pydantic import BaseModel
from sqlalchemy import and_, or_
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import engine, get_session
from app.models import Photo
from app.utils import get_base, resolve_path

router = APIRouter(prefix="/api/photos", tags=["photos"])


def _check_photo_access(location: str, path: str, email: str) -> None:
    if location != "external":
        raise HTTPException(403, "Access denied")
    clean = path.lstrip("/")
    parts = Path(clean).parts
    if len(parts) < 2 or parts[0] != "photos" or parts[1] != email:
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


class AssetMetadataBody(BaseModel):
    source_asset_ids: List[str]


@router.post("/metadata-by-asset")
async def metadata_by_asset(
    request: Request,
    body: AssetMetadataBody,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    if not body.source_asset_ids:
        return {"items": {}}
    rows = db.exec(
        select(Photo).where(
            Photo.owner_email == user["email"],
            Photo.source_asset_id.in_(body.source_asset_ids),
        )
    ).all()
    return {
        "items": {
            row.source_asset_id: {
                "checksum": row.checksum,
                "taken_at": row.taken_at.isoformat() if row.taken_at else None,
                "width": row.width,
                "height": row.height,
                "original_filename": row.original_filename,
                "file_size": row.file_size,
            }
            for row in rows
            if row.source_asset_id
        }
    }


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


def _canonical_relative_path(owner_email: str, taken_at: Optional[datetime], checksum: str, ext: str) -> str:
    date = taken_at or datetime.now(timezone.utc)
    return f"photos/{owner_email}/{date.year}/{date.month:02d}/{checksum}{ext}"


def _exif_taken_at(file_path: Path) -> Optional[datetime]:
    try:
        from PIL import Image
        with Image.open(file_path) as img:
            exif = img.getexif()
            raw = exif.get(0x9003) or exif.get(0x0132)  # DateTimeOriginal, else DateTime
        if raw:
            return datetime.strptime(raw, "%Y:%m:%d %H:%M:%S").replace(tzinfo=timezone.utc)
    except Exception:
        pass
    return None


def _move_file(src: Path, dest: Path, dry_run: bool) -> None:
    if dry_run:
        return
    dest.parent.mkdir(parents=True, exist_ok=True)
    src.rename(dest)


def _run_sanitize(base: Path, dry_run: bool, db: Session) -> dict:
    report = {
        "dry_run": dry_run,
        "moved": 0,
        "recovered_orphans": 0,
        "duplicates_removed": 0,
        "removed_missing": 0,
        "errors": [],
    }

    photos = db.exec(select(Photo)).all()
    known_checksums: set[tuple[str, str]] = {(p.owner_email, p.checksum) for p in photos}
    # Tracks the file each (owner, checksum) now lives at, so step 2 can tell a correctly
    # placed registered photo apart from a stray duplicate copy left behind on disk.
    canonical_file_of: dict[tuple[str, str], Path] = {}

    # 1. Move every registered photo to its canonical photos/<email>/YYYY/MM path.
    for photo in photos:
        try:
            ext = Path(photo.path).suffix or ".jpg"
            canonical = _canonical_relative_path(photo.owner_email, photo.taken_at, photo.checksum, ext)
            key = (photo.owner_email, photo.checksum)
            src = resolve_path(base, photo.path)
            # Checked unconditionally, even when photo.path already equals canonical — files
            # removed directly from disk (outside the app) left their row's path untouched, so a
            # path-equality fast path here would never notice the file was gone, and the row
            # (plus its source_asset_id) would linger forever, permanently blocking a fresh
            # backup/check from ever re-uploading it.
            if not src.exists():
                if not dry_run:
                    db.delete(photo)
                    known_checksums.discard(key)
                report["removed_missing"] += 1
                continue
            if photo.path == canonical:
                canonical_file_of[key] = src
                continue
            dest = resolve_path(base, canonical)
            if dest.exists() and dest.resolve() != src.resolve():
                report["errors"].append(f"canonical path already occupied for photo {photo.id}: {canonical}")
                continue
            _move_file(src, dest, dry_run)
            if not dry_run:
                photo.path = canonical
                db.add(photo)
            canonical_file_of[key] = dest if not dry_run else src
            report["moved"] += 1
        except Exception as exc:
            report["errors"].append(f"photo {photo.id}: {exc}")

    if not dry_run:
        db.commit()

    # 2. Walk legacy and current photo trees for files with no matching Photo row. Files already
    # sitting at a known-canonical location were just accounted for in step 1 (either confirmed
    # correctly placed or moved there) — skipping the hash for those turns every run after the
    # first into an incremental one that only reads files it hasn't classified yet, instead of
    # re-hashing the entire library (which is what made a large library's sanitize pass slow
    # enough to blow past the app's HTTP request timeout).
    canonical_paths = {p.resolve() for p in canonical_file_of.values()}
    legacy_roots = sorted((base / "users").glob("*/Photos")) if (base / "users").exists() else []
    current_roots = sorted((base / "photos").glob("*")) if (base / "photos").exists() else []
    for root in legacy_roots + current_roots:
        owner_email = root.parent.name if root.name == "Photos" else root.name
        for file_path in sorted(root.rglob("*")):
            if not file_path.is_file() or file_path.name.startswith("."):
                continue
            if file_path.resolve() in canonical_paths:
                continue
            try:
                checksum = _sha256_file(file_path)
                key = (owner_email, checksum)
                if key in known_checksums:
                    canonical = canonical_file_of.get(key)
                    if canonical is None or file_path.resolve() != canonical.resolve():
                        # The registered copy of this photo already lives elsewhere (or this
                        # scan hasn't recorded its canonical location yet, e.g. a leftover
                        # from a previous partial run) — this on-disk copy is a stray duplicate.
                        if not dry_run:
                            file_path.unlink()
                        report["duplicates_removed"] += 1
                    continue

                taken_at = _exif_taken_at(file_path) or datetime.fromtimestamp(
                    file_path.stat().st_mtime, tz=timezone.utc
                )
                ext = file_path.suffix or ".jpg"
                canonical_rel = _canonical_relative_path(owner_email, taken_at, checksum, ext)
                dest = resolve_path(base, canonical_rel)

                if not dry_run:
                    mime_type, _ = mimetypes.guess_type(str(file_path))
                    width = height = None
                    try:
                        from PIL import Image
                        with Image.open(file_path) as img:
                            width, height = img.size
                    except Exception:
                        pass
                    _move_file(file_path, dest, dry_run=False)
                    db.add(Photo(
                        owner_email=owner_email,
                        checksum=checksum,
                        path=canonical_rel,
                        original_filename=file_path.name,
                        file_size=file_path.stat().st_size if file_path.exists() else dest.stat().st_size,
                        mime_type=mime_type,
                        width=width,
                        height=height,
                        taken_at=taken_at,
                        source_type="recovered_orphan",
                        status="verified",
                        verified_at=datetime.now(timezone.utc),
                    ))
                    db.commit()
                    known_checksums.add(key)
                    # Without this, a root scanned later in this same run (e.g. `photos/<email>`
                    # overlapping the directory this file was just moved into) would see the file
                    # again, find no recorded canonical location for it, and delete it as a
                    # supposed stray duplicate of the row we just created.
                    canonical_file_of[key] = dest
                report["recovered_orphans"] += 1
            except Exception as exc:
                report["errors"].append(f"{file_path}: {exc}")

    # 3. Clean up now-empty legacy Photos/<year>/<month> directories (never touch users/<email> itself).
    if not dry_run:
        for root in legacy_roots:
            if not root.exists():
                continue
            for dirpath in sorted(root.rglob("*"), key=lambda p: len(p.parts), reverse=True):
                if dirpath.is_dir():
                    try:
                        dirpath.rmdir()
                    except OSError:
                        pass
            try:
                root.rmdir()
            except OSError:
                pass

    return report


def _run_sanitize_job(base: Path, dry_run: bool) -> dict:
    # Runs on its own session rather than the request-scoped one from `get_session`, since that
    # session's lifetime ends when the HTTP handler returns — which now happens immediately,
    # well before this (possibly many-minutes-long) job finishes.
    with Session(engine) as db:
        return _run_sanitize(base, dry_run, db)


# A full sanitize pass over a large library can take minutes (every not-yet-classified file is
# SHA-256 hashed). Blocking the HTTP request for that long routinely exceeded the app's request
# timeout while the pass kept running server-side regardless, silently mutating the DB in the
# background with no way for the client to know. Sanitize now starts the pass in a background
# task and returns immediately; `_sanitize_lock` stops a second tap from starting a redundant
# concurrent pass, and `/sanitize/status` lets the app poll for the report.
_sanitize_lock = threading.Lock()
_sanitize_state: dict = {"running": False, "last_report": None}


@router.post("/sanitize")
async def sanitize_photos(
    request: Request,
    dry_run: bool = Query(default=True),
):
    user = await get_current_user(request)
    if not user["is_admin"]:
        raise HTTPException(403, "Admin access required")

    if not _sanitize_lock.acquire(blocking=False):
        return {"started": False, "already_running": True}

    _sanitize_state["running"] = True
    base = get_base("external")

    async def runner():
        try:
            report = await run_in_threadpool(_run_sanitize_job, base, dry_run)
            _sanitize_state["last_report"] = report
        finally:
            _sanitize_state["running"] = False
            _sanitize_lock.release()

    asyncio.create_task(runner())
    return {"started": True, "already_running": False}


@router.get("/sanitize/status")
async def sanitize_status(request: Request):
    user = await get_current_user(request)
    if not user["is_admin"]:
        raise HTTPException(403, "Admin access required")
    return _sanitize_state


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
