from fastapi import Depends, HTTPException, Request
from sqlmodel import Session as DBSession, select
from datetime import datetime, timezone

from database import get_session
from models import AllowedEmail, Session as SessionModel, User


def require_allowed_email(email: str, session: DBSession = Depends(get_session)) -> str:

    allowed = session.exec(select(AllowedEmail).where(AllowedEmail.email == email)).first()

    if not allowed: raise HTTPException(status_code=403, detail="Email not authorized to register")
    if allowed.used: raise HTTPException(status_code=403, detail="Email was used before")

    return email


def require_login(request: Request, session: DBSession = Depends(get_session)) -> User:

    # Check for the session cookie
    session_id = request.cookies.get("session_id")
    if not session_id: raise HTTPException(status_code=401, detail="Not authenticated")

    # Check if the session cookie is on the database and it is not expired
    db_session = session.exec(select(SessionModel).where(SessionModel.session_id == session_id)).first()
    if not db_session: raise HTTPException(status_code=401, detail="sesión inválida")
    if db_session.expires_at < datetime.now(timezone.utc): raise HTTPException(status_code=401, detail="sesión expirada")

    #
    user = session.get(User, db_session.user_id)
    if not user: raise HTTPException(status_code=401, detail="usuario no encontrado")

    return user