from fastapi import Depends, HTTPException, Request
from sqlmodel import Session as DBSession, select
from datetime import datetime, timezone

from database import get_session
from models import Session as SessionModel, User


def _get_db_session_and_user(request: Request, session: DBSession):
    session_id = request.cookies.get("session_id")
    if not session_id:
        raise HTTPException(status_code=401, detail="Not authenticated")

    db_session = session.exec(
        select(SessionModel).where(SessionModel.session_id == session_id)
    ).first()
    if not db_session:
        raise HTTPException(status_code=401, detail="sesión inválida")
    if db_session.expires_at.replace(tzinfo=timezone.utc) < datetime.now(timezone.utc):
        raise HTTPException(status_code=401, detail="sesión expirada")

    user = session.get(User, db_session.user_id)
    if not user:
        raise HTTPException(status_code=401, detail="usuario no encontrado")

    return db_session, user


def require_login(request: Request, session: DBSession = Depends(get_session)) -> User:
    db_session, user = _get_db_session_and_user(request, session)
    if not db_session.totp_verified:
        raise HTTPException(status_code=403, detail="Verificación TOTP pendiente")
    return user


def require_password_session(request: Request, session: DBSession = Depends(get_session)) -> User:
    db_session, user = _get_db_session_and_user(request, session)
    if not db_session.totp_verified:
        raise HTTPException(status_code=403, detail="Verificación TOTP pendiente")
    if db_session.auth_method != "password":
        raise HTTPException(
            status_code=403,
            detail="Debes iniciar sesión con contraseña para registrar una passkey",
        )
    return user


def require_admin(user: User = Depends(require_login)) -> User:
    if not user.is_admin:
        raise HTTPException(status_code=403, detail="Admin required")
    return user
