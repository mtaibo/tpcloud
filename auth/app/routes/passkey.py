import base64
import json
from datetime import datetime, timezone

from fastapi import APIRouter, BackgroundTasks, Depends, HTTPException, Request, Response
from database import get_session
from sqlmodel import Session as DBSession, select
from models import PasskeyCredential, Session as SessionModel, User

from config import MODULES, ORIGIN, RP_ID, SESSION_COOKIE
from routes.flow import redirect_to_login
from security import ExpiringDict, clear_session_cookie, ip_failures, set_session_cookie
from sessions import start_session
from webauthn import generate_authentication_options, verify_authentication_response
from webauthn.helpers import options_to_json
from webauthn.helpers.structs import PublicKeyCredentialDescriptor, UserVerificationRequirement
from utils import (
    LAST_SEEN_INTERVAL,
    as_utc,
    cookie_lifetime,
    get_client_ip,
    record_event,
    session_duration_for,
)


def b64url_encode(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()

def b64url_decode(data: str) -> bytes:
    padding = "=" * (-len(data) % 4)
    return base64.urlsafe_b64decode(data + padding)


router = APIRouter(prefix="/auth/passkey")

# Anyone can start a passkey sign-in, so its challenges expire and are capped. Keyed by the
# challenge itself (the browser signs it into clientDataJSON), so a sign-in doesn't need an email:
# with none, the browser offers whichever passkey this site has on the device.
_pending_login_challenges = ExpiringDict(ttl=300)


def _valid_session(request: Request, session: DBSession) -> SessionModel | None:
    session_id = request.cookies.get(SESSION_COOKIE)
    if not session_id:
        return None
    db_session = session.exec(
        select(SessionModel).where(SessionModel.session_id == session_id)
    ).first()
    if (
        db_session
        and as_utc(db_session.expires_at) > datetime.now(timezone.utc)
        and db_session.totp_verified
    ):
        return db_session
    return None


@router.post("/login/begin")
def login_begin(request: Request, email: str | None = None, session: DBSession = Depends(get_session)):
    ip_failures.check(f"ip:{get_client_ip(request)}")
    allow = []
    if email:
        user = session.exec(select(User).where(User.email == email.strip().lower())).first()
        credentials = (
            session.exec(select(PasskeyCredential).where(PasskeyCredential.user_id == user.id)).all()
            if user else []
        )
        # An unknown email gets the same answer as a known one (options with no credentials
        # listed), so this endpoint can't be used to find out which accounts exist.
        allow = [PublicKeyCredentialDescriptor(id=b64url_decode(c.credential_id)) for c in credentials]

    options = generate_authentication_options(
        rp_id=RP_ID,
        allow_credentials=allow,
        user_verification=UserVerificationRequirement.REQUIRED,
    )
    _pending_login_challenges.set(b64url_encode(options.challenge), options.challenge)
    return Response(content=options_to_json(options), media_type="application/json")


def _challenge_of(credential: dict) -> bytes | None:
    try:
        client_data = json.loads(b64url_decode(credential["response"]["clientDataJSON"]))
        return _pending_login_challenges.pop(str(client_data["challenge"]))
    except Exception:
        return None


@router.post("/login/complete")
def login_complete(
    request_body: dict,
    request: Request,
    background_tasks: BackgroundTasks,
    remember: bool = True,
    session: DBSession = Depends(get_session),
):
    ip = get_client_ip(request)
    ip_failures.check(f"ip:{ip}")

    challenge = _challenge_of(request_body)
    if not challenge:
        raise HTTPException(status_code=400, detail="La solicitud ha caducado. Vuelve a intentarlo.")

    stored_credential = session.exec(
        select(PasskeyCredential).where(PasskeyCredential.credential_id == str(request_body.get("id", "")))
    ).first()
    user = session.get(User, stored_credential.user_id) if stored_credential else None
    if not stored_credential or not user:
        ip_failures.fail(f"ip:{ip}")
        raise HTTPException(status_code=401, detail="Esta passkey no pertenece a ninguna cuenta de este servidor.")

    try:
        verify_authentication_response(
            credential=request_body,
            expected_challenge=challenge,
            expected_rp_id=RP_ID,
            expected_origin=ORIGIN,
            credential_public_key=b64url_decode(stored_credential.public_key),
            credential_current_sign_count=0,
            require_user_verification=True,
        )
    except Exception:
        ip_failures.fail(f"ip:{ip}")
        record_event(request, user.id, "failed", "passkey", "La passkey no se pudo verificar")
        raise HTTPException(status_code=401, detail="Credenciales inválidas")

    stored_credential.last_used_at = datetime.now(timezone.utc)
    session.add(stored_credential)

    response = Response(status_code=200)
    start_session(session, background_tasks, request, response, user, "passkey", remember)
    return response


@router.get("/check")
def check(request: Request, session: DBSession = Depends(get_session)):
    if _valid_session(request, session):
        return Response(status_code=200)
    raise HTTPException(status_code=401, detail="no autenticado")


@router.get("/me")
def me(request: Request, session: DBSession = Depends(get_session)):
    db_session = _valid_session(request, session)
    if db_session:
        user = session.get(User, db_session.user_id)
        if user:
            return {
                "email": user.email,
                "display_name": user.display_name,
                "is_admin": user.is_admin,
                "modules": MODULES,
            }
    raise HTTPException(status_code=401, detail="no autenticado")


@router.post("/logout")
def logout(request: Request, session: DBSession = Depends(get_session)):
    session_id = request.cookies.get(SESSION_COOKIE)
    if session_id:
        db_session = session.exec(
            select(SessionModel).where(SessionModel.session_id == session_id)
        ).first()
        if db_session:
            record_event(request, db_session.user_id, "signout", db_session.auth_method)
            session.delete(db_session)
            session.commit()
    response = Response(status_code=200)
    clear_session_cookie(response)
    return response


@router.get("/validate")
def validate(request: Request, session: DBSession = Depends(get_session)):
    db_session = _valid_session(request, session)
    if db_session:
        now = datetime.now(timezone.utc)
        response = Response(status_code=200)
        changed = False

        last_seen = as_utc(db_session.last_seen_at)
        ip = get_client_ip(request)
        if not last_seen or now - last_seen > LAST_SEEN_INTERVAL or db_session.last_ip != ip:
            db_session.last_seen_at = now
            db_session.last_ip = ip
            changed = True

        # Sliding: a session in use keeps being pushed forward, renewed once a twelfth of it has
        # gone by (every ~2.5 days for a remembered one) so it isn't rewritten on every request.
        duration = session_duration_for(db_session.client_type, db_session.remember)
        if as_utc(db_session.expires_at) - now < duration * 11 / 12:
            db_session.expires_at = now + duration
            changed = True
            # Refresh the cookie's own Max-Age too, otherwise the client drops it at its
            # original expiry regardless of this server-side renewal. Caddy's forward_auth
            # must have `copy_headers Set-Cookie` for this to actually reach the client.
            set_session_cookie(response, db_session.session_id, cookie_lifetime(db_session))

        if changed:
            session.add(db_session)
            session.commit()
        return response

    # Back to the same page after signing in, remembered in a cookie so /login stays clean. Only
    # for a page being opened: an API call from a page left open shouldn't be where you land.
    uri = request.headers.get("X-Forwarded-Uri", "/")
    mode = request.headers.get("Sec-Fetch-Mode")
    is_page = mode == "navigate" if mode else (
        request.headers.get("X-Forwarded-Method", "GET") == "GET" and "/api/" not in uri
    )
    return redirect_to_login(uri if is_page else None)
