from datetime import datetime, timezone

import bcrypt
from fastapi import APIRouter, BackgroundTasks, Depends, HTTPException, Request
from fastapi.responses import JSONResponse
from sqlmodel import Session as DBSession, select

from config import SESSION_COOKIE
from database import get_session
from models import Session as SessionModel, User
from security import ip_failures, login_failures, user_failures, verify_totp
from sessions import complete_totp, start_session
from utils import as_utc, get_client_ip, record_event

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
    remember = bool(body.get("remember", True))

    if not email or not password:
        raise HTTPException(status_code=400, detail="Email and password are required.")

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
        if user:
            record_event(request, user.id, "failed", "password", "Contraseña incorrecta")
        raise HTTPException(status_code=401, detail="Incorrect email or password.")

    login_failures.reset(email_key)

    totp_needed = user.totp_secret is not None
    response = JSONResponse({"totp_required": totp_needed})
    start_session(session, background_tasks, request, response, user, "password", remember, totp_pending=totp_needed)
    return response


@router.post("/totp/verify")
def verify_totp_code(
    body: dict,
    request: Request,
    session: DBSession = Depends(get_session),
):
    session_id = request.cookies.get(SESSION_COOKIE)
    if not session_id:
        raise HTTPException(status_code=401, detail="Not signed in.")

    db_session = session.exec(
        select(SessionModel).where(SessionModel.session_id == session_id)
    ).first()
    if not db_session:
        raise HTTPException(status_code=401, detail="Invalid session.")
    if as_utc(db_session.expires_at) < datetime.now(timezone.utc):
        raise HTTPException(status_code=401, detail="That took too long. Please sign in again.")
    if db_session.totp_verified:
        return {"status": "ok"}

    user = session.get(User, db_session.user_id)
    if not user or not user.totp_secret:
        raise HTTPException(status_code=400, detail="Two-step verification isn't set up.")

    user_key = f"user:{user.id}"
    user_failures.check(user_key)

    if not verify_totp(user.totp_secret, str(body.get("code", "")), user.id):
        user_failures.fail(user_key)
        record_event(request, user.id, "failed", "password", "Código de verificación incorrecto")
        attempts = _totp_attempts.get(session_id, 0) + 1
        if attempts >= _TOTP_ATTEMPTS_PER_SESSION:
            _totp_attempts.pop(session_id, None)
            session.delete(db_session)
            session.commit()
            raise HTTPException(status_code=401, detail="Too many incorrect codes. Please sign in again.")
        _totp_attempts[session_id] = attempts
        raise HTTPException(status_code=401, detail="Incorrect code.")

    _totp_attempts.pop(session_id, None)
    user_failures.reset(user_key)

    response = JSONResponse({"status": "ok"})
    complete_totp(session, request, response, db_session)
    return response
