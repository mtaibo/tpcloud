import json
import os
import time
from datetime import datetime, timezone

import bcrypt
from fastapi import APIRouter, Depends, HTTPException
from sqlmodel import Session as DBSession, select, func

from database import get_session
from dependencies import require_admin
from models import LoginEvent, PasskeyCredential, Session as SessionModel, User
from sessions import session_json
from config import VERSION
from utils import iso, say

router = APIRouter(prefix="/auth/admin")


@router.get("/users")
def list_users(
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    users = session.exec(select(User)).all()
    now = datetime.now(timezone.utc)
    result = []
    for u in users:
        passkey_count = session.exec(
            select(func.count()).where(PasskeyCredential.user_id == u.id)
        ).one()
        session_count = session.exec(
            select(func.count()).where(
                SessionModel.user_id == u.id,
                SessionModel.expires_at > now,
            )
        ).one()
        result.append({
            "id": u.id,
            "email": u.email,
            "display_name": u.display_name,
            "is_admin": u.is_admin,
            "has_password": u.password_hash is not None,
            "totp_enabled": u.totp_secret is not None,
            "created_at": iso(u.created_at),
            "passkey_count": passkey_count,
            "active_session_count": session_count,
        })
    return result


@router.post("/users")
def create_user(
    body: dict,
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    email = body.get("email", "").strip().lower()
    display_name = body.get("display_name", "").strip()
    password = body.get("password", "")

    if not email:
        raise HTTPException(status_code=400, detail=say(admin, "An email is required", "Email requerido"))
    if not password or len(password) < 12:
        raise HTTPException(status_code=400, detail=say(admin, "The password must be at least 12 characters long", "La contraseña debe tener al menos 12 caracteres"))
    if not display_name:
        display_name = email.split("@")[0]

    existing = session.exec(select(User).where(User.email == email)).first()
    if existing:
        raise HTTPException(status_code=409, detail=say(admin, "There's already a user with that email", "Ya existe un usuario con ese email"))

    password_hash = bcrypt.hashpw(password.encode(), bcrypt.gensalt()).decode()
    new_user = User(
        email=email,
        display_name=display_name,
        password_hash=password_hash,
        is_admin=False,
    )
    session.add(new_user)
    session.commit()
    session.refresh(new_user)

    return {
        "id": new_user.id,
        "email": new_user.email,
        "display_name": new_user.display_name,
        "is_admin": new_user.is_admin,
    }


@router.patch("/users/{user_id}")
def toggle_admin(
    user_id: int,
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    if user_id == admin.id:
        raise HTTPException(status_code=400, detail=say(admin, "You can't change your own role", "No puedes cambiar tu propio rol"))
    user = session.get(User, user_id)
    if not user:
        raise HTTPException(status_code=404, detail=say(admin, "User not found", "Usuario no encontrado"))
    user.is_admin = not user.is_admin
    session.add(user)
    session.commit()
    return {"id": user.id, "is_admin": user.is_admin}


@router.delete("/users/{user_id}")
def delete_user(
    user_id: int,
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    if user_id == admin.id:
        raise HTTPException(status_code=400, detail=say(admin, "You can't delete yourself", "No puedes eliminarte a ti mismo"))
    user = session.get(User, user_id)
    if not user:
        raise HTTPException(status_code=404, detail=say(admin, "User not found", "Usuario no encontrado"))

    for p in session.exec(select(PasskeyCredential).where(PasskeyCredential.user_id == user_id)).all():
        session.delete(p)
    for s in session.exec(select(SessionModel).where(SessionModel.user_id == user_id)).all():
        session.delete(s)
    for e in session.exec(select(LoginEvent).where(LoginEvent.user_id == user_id)).all():
        session.delete(e)
    session.delete(user)
    session.commit()
    return {"status": "eliminado"}


@router.get("/sessions")
def list_sessions(
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    now = datetime.now(timezone.utc)
    sessions = session.exec(
        select(SessionModel).where(SessionModel.expires_at > now, SessionModel.totp_verified == True)  # noqa: E712
    ).all()
    result = []
    for s in sessions:
        user = session.get(User, s.user_id)
        result.append({
            **session_json(s),
            "user_id": s.user_id,
            "user_email": user.email if user else "—",
            "user_display_name": user.display_name if user else "—",
        })
    result.sort(key=lambda s: s["last_seen_at"] or "", reverse=True)
    return result


@router.delete("/sessions/{session_row_id}")
def revoke_session(
    session_row_id: int,
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    db_session = session.get(SessionModel, session_row_id)
    if not db_session:
        raise HTTPException(status_code=404, detail=say(admin, "Session not found", "Sesión no encontrada"))
    session.delete(db_session)
    session.commit()
    return {"status": "revocada"}


# ── The host: its version, a newer one, and updating it from an app (`tpcloud` does the work) ──

_STATE_DIR = os.environ.get("TPCLOUD_STATE_DIR", "")
_UPDATING = {"checking", "downloading", "installing"}


def _read_json(name: str) -> dict | None:
    try:
        with open(os.path.join(_STATE_DIR, "run", name)) as f:
            return json.load(f)
    except (OSError, ValueError):
        return None


def _newer(a: str, b: str) -> bool:
    def parts(v: str) -> list[int]:
        return [int(p) if p.isdigit() else 0 for p in v.split(".")]
    return parts(a) > parts(b)


def _request(name: str) -> None:
    requests = os.path.join(_STATE_DIR, "requests")
    if not _STATE_DIR or not os.path.isdir(requests):
        raise HTTPException(status_code=501, detail="This server can't be updated from here (it isn't run by the tpcloud command).")
    with open(os.path.join(requests, name), "w"):
        pass


@router.get("/host")
def host(admin: User = Depends(require_admin)):
    """This server's version, the newest TPCloud the host found, and how an update is going."""
    latest = _read_json("latest.json") or {}
    update = _read_json("update.json")
    version = VERSION
    newest = latest.get("version")
    # The daily look may predate an update (it said 0.5.0 on a host already running 0.5.3):
    # what's running is never older than the newest.
    if version and (not newest or _newer(version, newest)):
        newest = version
    # Running the version it was installing: it's done (a version before 0.4.1 didn't say so).
    if update and update.get("state") == "installing" and update.get("detail") == version:
        update = {**update, "state": "done"}
    # A finished update that's already running is history, not news.
    if update and update.get("state") == "done" and update.get("detail") == version and time.time() - update.get("at", 0) > 600:
        update = None
    return {
        "version": version,
        "latest": newest,
        "checked": latest.get("checked"),
        "available": bool(newest and version and _newer(newest, version)),
        "update": update,
        "can_update": bool(_STATE_DIR and os.path.isdir(os.path.join(_STATE_DIR, "requests"))),
    }


@router.post("/host/check")
def host_check(admin: User = Depends(require_admin)):
    """Look for a newer TPCloud now (the host checks once a day by itself)."""
    _request("check")
    return {"status": "checking"}


@router.post("/host/update")
def host_update(admin: User = Depends(require_admin)):
    """Update the host to the newest TPCloud: it downloads it, checks it against GitHub's checksum
    and installs it, restarting in a few seconds."""
    update = _read_json("update.json") or {}
    if update.get("state") in _UPDATING and time.time() - update.get("at", 0) < 900:
        raise HTTPException(status_code=409, detail="It's updating already.")
    _request("update")
    return {"status": "starting"}
