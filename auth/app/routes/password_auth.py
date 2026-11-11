from datetime import datetime, timezone

import bcrypt
from fastapi import APIRouter, BackgroundTasks, Depends, HTTPException, Request, Response
from sqlmodel import Session as DBSession, select

from config import SESSION_COOKIE
from database import get_session
from models import Session as SessionModel, User
from security import ip_failures, login_failures, set_session_cookie, user_failures, verify_totp
from utils import detect_client_type, get_client_ip, parse_device_info, session_duration_for, update_session_location

router = APIRouter(prefix="/auth/password")

# Checked against when the email doesn't exist or has no password, so those answers take as long
# as a wrong password and the timing doesn't tell which accounts exist.
_DUMMY_HASH = bcrypt.hashpw(b"tpcloud-dummy-password", bcrypt.gensalt()).decode()

# Wrong codes a half-signed-in session may try before it's thrown away and the password has to be
# entered again.
_TOTP_ATTEMPTS_PER_SESSION = 5
_totp_attempts: dict[str, int] = {}


@router.post("/login")
def password_login(
    body: dict,
    request: Request,
    background_tasks: BackgroundTasks,
    session: DBSession = Depends(get_session),
):
    email = str(body.get("email", "")).strip().lower()
    password = str(body.get("password", ""))

    if not email or not password:
        raise HTTPException(status_code=400, detail="Email y contraseña requeridos")

    ip = get_client_ip(request)
    email_key, ip_key = f"email:{email}", f"ip:{ip}"
    login_failures.check(email_key)
    ip_failures.check(ip_key)

    user = session.exec(select(User).where(User.email == email)).first()
    password_hash = user.password_hash if user and user.password_hash else _DUMMY_HASH
    password_ok = bcrypt.checkpw(password.encode(), password_hash.encode())
    if not user or not user.password_hash or not password_ok:
        login_failures.fail(email_key)
        ip_failures.fail(ip_key)
        raise HTTPException(status_code=401, detail="Credenciales inválidas")

    login_failures.reset(email_key)

    totp_needed = user.totp_secret is not None
    ua_string = request.headers.get("User-Agent", "")
    client_type = detect_client_type(request)
    duration = session_duration_for(client_type)

    new_session = SessionModel(
        user_id=user.id,
        expires_at=datetime.now(timezone.utc) + duration,
        ip_address=ip,
        user_agent=ua_string,
        totp_verified=not totp_needed,
        auth_method="password",
        device_info=parse_device_info(ua_string),
        location=ip,
        client_type=client_type,
    )
    session.add(new_session)
    session.commit()
    background_tasks.add_task(update_session_location, new_session.session_id, ip)

    response = Response(
        content=f'{{"totp_required": {str(totp_needed).lower()}}}',
        media_type="application/json",
        status_code=200,
    )
    set_session_cookie(response, new_session.session_id, int(duration.total_seconds()))
    return response


@router.post("/totp/verify")
def verify_totp_code(
    body: dict,
    request: Request,
    session: DBSession = Depends(get_session),
):
    session_id = request.cookies.get(SESSION_COOKIE)
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

    user_key = f"user:{user.id}"
    user_failures.check(user_key)

    if not verify_totp(user.totp_secret, str(body.get("code", "")), user.id):
        user_failures.fail(user_key)
        attempts = _totp_attempts.get(session_id, 0) + 1
        if attempts >= _TOTP_ATTEMPTS_PER_SESSION:
            _totp_attempts.pop(session_id, None)
            session.delete(db_session)
            session.commit()
            raise HTTPException(status_code=401, detail="Demasiados códigos incorrectos. Vuelve a iniciar sesión.")
        _totp_attempts[session_id] = attempts
        raise HTTPException(status_code=401, detail="Código incorrecto")

    _totp_attempts.pop(session_id, None)
    user_failures.reset(user_key)
    db_session.totp_verified = True
    session.add(db_session)
    session.commit()

    return {"status": "ok"}
