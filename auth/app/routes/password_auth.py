from datetime import datetime, timedelta, timezone

import bcrypt
from fastapi import APIRouter, Depends, HTTPException, Request, Response
from sqlmodel import Session as DBSession, select

from database import get_session
from models import Session as SessionModel, User
from utils import get_client_ip, lookup_location, parse_device_info

router = APIRouter(prefix="/auth/password")

SESSION_DURATION = timedelta(hours=1)


@router.post("/login")
def password_login(
    body: dict,
    request: Request,
    session: DBSession = Depends(get_session),
):
    email = body.get("email", "").strip().lower()
    password = body.get("password", "")

    if not email or not password:
        raise HTTPException(status_code=400, detail="Email y contraseña requeridos")

    user = session.exec(select(User).where(User.email == email)).first()
    if not user:
        raise HTTPException(status_code=401, detail="Credenciales inválidas")

    if not user.password_hash:
        raise HTTPException(
            status_code=403,
            detail="Esta cuenta no tiene contraseña. Usa tu passkey.",
        )

    if not bcrypt.checkpw(password.encode(), user.password_hash.encode()):
        raise HTTPException(status_code=401, detail="Credenciales inválidas")

    totp_needed = user.totp_secret is not None
    ip = get_client_ip(request)
    ua_string = request.headers.get("User-Agent", "")

    new_session = SessionModel(
        user_id=user.id,
        expires_at=datetime.now(timezone.utc) + SESSION_DURATION,
        ip_address=ip,
        user_agent=ua_string,
        totp_verified=not totp_needed,
        auth_method="password",
        device_info=parse_device_info(ua_string),
        location=lookup_location(ip),
    )
    session.add(new_session)
    session.commit()

    response = Response(
        content=f'{{"totp_required": {str(totp_needed).lower()}}}',
        media_type="application/json",
        status_code=200,
    )
    response.set_cookie(
        key="session_id",
        value=new_session.session_id,
        domain=".migueltaibo.com",
        httponly=True,
        secure=True,
        samesite="lax",
        max_age=int(SESSION_DURATION.total_seconds()),
    )
    return response


@router.post("/totp/verify")
def verify_totp(
    body: dict,
    request: Request,
    session: DBSession = Depends(get_session),
):
    import pyotp

    session_id = request.cookies.get("session_id")
    if not session_id:
        raise HTTPException(status_code=401, detail="No autenticado")

    db_session = session.exec(
        select(SessionModel).where(SessionModel.session_id == session_id)
    ).first()
    if not db_session:
        raise HTTPException(status_code=401, detail="Sesión inválida")
    if db_session.expires_at.replace(tzinfo=timezone.utc) < datetime.now(timezone.utc):
        raise HTTPException(status_code=401, detail="Sesión expirada")
    if db_session.totp_verified:
        return {"status": "ok"}

    user = session.get(User, db_session.user_id)
    if not user or not user.totp_secret:
        raise HTTPException(status_code=400, detail="TOTP no configurado")

    code = str(body.get("code", "")).strip()
    totp = pyotp.TOTP(user.totp_secret)
    if not totp.verify(code, valid_window=1):
        raise HTTPException(status_code=401, detail="Código incorrecto")

    db_session.totp_verified = True
    session.add(db_session)
    session.commit()

    return {"status": "ok"}
