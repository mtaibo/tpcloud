from datetime import datetime, timezone

import bcrypt
from fastapi import APIRouter, Depends, HTTPException
from sqlmodel import Session as DBSession, select, func

from database import get_session
from dependencies import require_admin
from models import LoginEvent, PasskeyCredential, Session as SessionModel, User
from sessions import session_json
from utils import iso

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
        raise HTTPException(status_code=400, detail="Email requerido")
    if not password or len(password) < 12:
        raise HTTPException(status_code=400, detail="La contraseña debe tener al menos 12 caracteres")
    if not display_name:
        display_name = email.split("@")[0]

    existing = session.exec(select(User).where(User.email == email)).first()
    if existing:
        raise HTTPException(status_code=409, detail="Ya existe un usuario con ese email")

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
        raise HTTPException(status_code=400, detail="No puedes cambiar tu propio rol")
    user = session.get(User, user_id)
    if not user:
        raise HTTPException(status_code=404, detail="Usuario no encontrado")
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
        raise HTTPException(status_code=400, detail="No puedes eliminarte a ti mismo")
    user = session.get(User, user_id)
    if not user:
        raise HTTPException(status_code=404, detail="Usuario no encontrado")

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
        raise HTTPException(status_code=404, detail="Sesión no encontrada")
    session.delete(db_session)
    session.commit()
    return {"status": "revocada"}
