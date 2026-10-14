import json
import os
import subprocess
import threading
from datetime import datetime, timezone
from pathlib import Path

from fastapi import APIRouter, HTTPException, Request

from app.auth import get_current_user

router = APIRouter(prefix="/api/backup", tags=["backup"])

_lock = threading.Lock()
_running = False

_STATUS_FILE = Path(os.getenv("EXTERNAL_DISK_PATH", "/mnt/external")) / ".backup_status.json"
_SOURCE = "/mnt/server/mnt/storage/"
_DEST = "/mnt/server/mnt/backup/"


def _read_status() -> dict:
    try:
        return json.loads(_STATUS_FILE.read_text())
    except Exception:
        return {"last_run": None, "last_exit_code": None}


def _do_backup():
    global _running
    started_at = datetime.now(timezone.utc).isoformat()
    try:
        result = subprocess.run(
            ["rsync", "-avxHAX", "--delete", _SOURCE, _DEST],
            capture_output=True,
            text=True,
        )
        exit_code = result.returncode
    except Exception:
        exit_code = -1
    finally:
        _running = False
        _STATUS_FILE.write_text(json.dumps({
            "last_run": started_at,
            "last_exit_code": exit_code,
        }))


@router.post("/run")
async def run_backup(request: Request):
    global _running
    user = await get_current_user(request)
    if not user["is_admin"]:
        raise HTTPException(403, "Admin access required")
    with _lock:
        if _running:
            raise HTTPException(409, "Backup already running")
        _running = True
    threading.Thread(target=_do_backup, daemon=True).start()
    return {"status": "started"}


@router.get("/status")
async def backup_status(request: Request):
    user = await get_current_user(request)
    if not user["is_admin"]:
        raise HTTPException(403, "Admin access required")
    status = _read_status()
    status["running"] = _running
    return status
