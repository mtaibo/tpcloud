import os
import threading
import time
import uuid
import zipfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException, Query, Request
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import engine, get_session
from app.models import CompressionJob
from app.utils import check_access, get_base, resolve_path

router = APIRouter(prefix="/api/compression", tags=["compression"])

_executor = ThreadPoolExecutor(max_workers=2)
_cancel_events: dict[str, threading.Event] = {}


def _do_extract(job_id: str, zip_path: Path, dest_dir: Path) -> None:
    cancel = _cancel_events.get(job_id)
    with Session(engine) as db:
        job = db.get(CompressionJob, job_id)
        if not job:
            return
        try:
            with zipfile.ZipFile(zip_path, "r") as zf:
                members = zf.infolist()

                # Validate paths before starting
                dest_str = str(dest_dir.resolve())
                for m in members:
                    member_path = (dest_dir / m.filename).resolve()
                    if not str(member_path).startswith(dest_str):
                        raise ValueError(f"Unsafe path in zip: {m.filename}")

                total = sum(m.file_size for m in members)
                job.total_bytes = total
                db.add(job)
                db.commit()

                extracted = 0
                last_commit = time.monotonic()
                for member in members:
                    if cancel and cancel.is_set():
                        job = db.get(CompressionJob, job_id)
                        if job:
                            job.status = "cancelled"
                            db.add(job)
                            db.commit()
                        return

                    zf.extract(member, dest_dir)
                    extracted += member.file_size

                    now = time.monotonic()
                    if now - last_commit >= 0.5:
                        job = db.get(CompressionJob, job_id)
                        if job:
                            job.extracted_bytes = extracted
                            db.add(job)
                            db.commit()
                        last_commit = now

                job = db.get(CompressionJob, job_id)
                if job:
                    job.status = "done"
                    job.extracted_bytes = total
                    db.add(job)
                    db.commit()

        except Exception as e:
            job = db.get(CompressionJob, job_id)
            if job and job.status == "active":
                job.status = "error"
                job.error = str(e)[:500]
                db.add(job)
                db.commit()
        finally:
            _cancel_events.pop(job_id, None)


@router.post("/jobs")
async def start_decompress(
    request: Request,
    path: str = Query(default=""),
    location: str = Query(default="external"),
    filename: str = Query(...),
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    check_access(location, path, user["email"], user["is_admin"])
    base = get_base(location)
    dir_path = resolve_path(base, path)
    safe_name = Path(filename).name
    zip_path = (dir_path / safe_name).resolve()

    if not str(zip_path).startswith(str(dir_path.resolve())):
        raise HTTPException(400, "Invalid path")
    if not zip_path.exists() or not zip_path.is_file():
        raise HTTPException(404, "File not found")
    if not zipfile.is_zipfile(zip_path):
        raise HTTPException(400, "Not a valid ZIP file")

    job_id = str(uuid.uuid4())
    cancel_event = threading.Event()
    _cancel_events[job_id] = cancel_event

    job = CompressionJob(
        job_id=job_id,
        owner_email=user["email"],
        filename=safe_name,
        path=path,
        location=location,
    )
    db.add(job)
    db.commit()

    _executor.submit(_do_extract, job_id, zip_path, dir_path)
    return {"job_id": job_id}


@router.get("/jobs")
async def list_jobs(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    jobs = db.exec(
        select(CompressionJob)
        .where(CompressionJob.owner_email == user["email"])
        .order_by(CompressionJob.created_at.desc())
    ).all()
    return [
        {
            "job_id": j.job_id,
            "filename": j.filename,
            "status": j.status,
            "total_bytes": j.total_bytes,
            "extracted_bytes": j.extracted_bytes,
            "error": j.error,
        }
        for j in jobs
        if j.status in ("active", "done", "error")
    ]


@router.delete("/jobs/{job_id}")
async def cancel_or_dismiss_job(
    job_id: str,
    request: Request,
    db: Session = Depends(get_session),
):
    user = await get_current_user(request)
    job = db.get(CompressionJob, job_id)
    if not job or job.owner_email != user["email"]:
        raise HTTPException(404, "Job not found")

    cancel = _cancel_events.get(job_id)
    if cancel:
        cancel.set()

    if job.status in ("done", "error", "cancelled"):
        db.delete(job)
    else:
        job.status = "cancelled"
        db.add(job)
    db.commit()
    return {"ok": True}
