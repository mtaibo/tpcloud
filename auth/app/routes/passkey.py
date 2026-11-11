import base64
from datetime import datetime, timezone
from urllib.parse import quote

from fastapi import APIRouter, BackgroundTasks, Depends, HTTPException, Request, Response
from fastapi.responses import RedirectResponse
from database import get_session
from sqlmodel import Session as DBSession, select
from models import PasskeyCredential, Session as SessionModel, User

import webauthn
from config import MODULES, ORIGIN, PUBLIC_URL, RP_ID, RP_NAME, SESSION_COOKIE
from dependencies import require_password_session
from security import ExpiringDict, clear_session_cookie, ip_failures, set_session_cookie
from webauthn import generate_authentication_options, verify_authentication_response
from webauthn.helpers import options_to_json
from webauthn.helpers.structs import (
    AuthenticatorSelectionCriteria,
    PublicKeyCredentialDescriptor,
    UserVerificationRequirement,
)
from utils import detect_client_type, get_client_ip, parse_device_info, session_duration_for, update_session_location


def b64url_encode(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()

def b64url_decode(data: str) -> bytes:
    padding = "=" * (-len(data) % 4)
    return base64.urlsafe_b64decode(data + padding)


router = APIRouter(prefix="/auth/passkey")

# Anyone can start a passkey sign-in, so its challenges expire and are capped.
_pending_registration_challenges = ExpiringDict(ttl=300)
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
        and db_session.expires_at.replace(tzinfo=timezone.utc) > datetime.now(timezone.utc)
        and db_session.totp_verified
    ):
        return db_session
    return None


@router.post("/register/begin")
def register_begin(user: User = Depends(require_password_session)):
    options = webauthn.generate_registration_options(
        rp_id=RP_ID,
        rp_name=RP_NAME,
        user_name=user.email,
        user_id=user.email.encode(),
        user_display_name=user.display_name,
        authenticator_selection=AuthenticatorSelectionCriteria(
            user_verification=UserVerificationRequirement.REQUIRED,
        ),
    )
    _pending_registration_challenges.set(user.email, options.challenge)
    return Response(content=options_to_json(options), media_type="application/json")


@router.post("/register/complete")
def register_complete(
    request_body: dict,
    user: User = Depends(require_password_session),
    session: DBSession = Depends(get_session),
):
    challenge = _pending_registration_challenges.pop(user.email)
    if not challenge:
        raise HTTPException(status_code=400, detail="No hay registro pendiente para este usuario")

    try:
        verification = webauthn.verify_registration_response(
            credential=request_body,
            expected_challenge=challenge,
            expected_rp_id=RP_ID,
            expected_origin=ORIGIN,
        )
    except Exception:
        raise HTTPException(status_code=400, detail="Verificación de passkey fallida")

    credential = PasskeyCredential(
        user_id=user.id,
        credential_id=b64url_encode(verification.credential_id),
        public_key=b64url_encode(verification.credential_public_key),
        device_name=str(request_body.get("device_name", "nuevo dispositivo"))[:100],
    )
    session.add(credential)
    session.commit()

    return {"status": "passkey registrada", "user_id": user.id}


@router.post("/login/begin")
def login_begin(email: str, request: Request, session: DBSession = Depends(get_session)):
    ip_failures.check(f"ip:{get_client_ip(request)}")
    email = email.strip().lower()
    user = session.exec(select(User).where(User.email == email)).first()
    credentials = (
        session.exec(select(PasskeyCredential).where(PasskeyCredential.user_id == user.id)).all()
        if user else []
    )

    # An unknown email gets the same answer as a known one (options with no credentials listed),
    # so this endpoint can't be used to find out which accounts exist.
    options = generate_authentication_options(
        rp_id=RP_ID,
        allow_credentials=[PublicKeyCredentialDescriptor(id=b64url_decode(c.credential_id)) for c in credentials],
        user_verification=UserVerificationRequirement.REQUIRED,
    )
    _pending_login_challenges.set(email, options.challenge)

    return Response(content=options_to_json(options), media_type="application/json")


@router.post("/login/complete")
def login_complete(
    request_body: dict,
    email: str,
    request: Request,
    background_tasks: BackgroundTasks,
    session: DBSession = Depends(get_session),
):
    ip = get_client_ip(request)
    ip_failures.check(f"ip:{ip}")
    email = email.strip().lower()

    challenge = _pending_login_challenges.pop(email)
    if not challenge:
        raise HTTPException(status_code=400, detail="No hay login pendiente para este email")

    user = session.exec(select(User).where(User.email == email)).first()
    stored_credential = session.exec(
        select(PasskeyCredential).where(
            PasskeyCredential.credential_id == str(request_body.get("id", "")),
            PasskeyCredential.user_id == user.id,
        )
    ).first() if user else None
    if not stored_credential:
        ip_failures.fail(f"ip:{ip}")
        raise HTTPException(status_code=401, detail="Credenciales inválidas")

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
        raise HTTPException(status_code=401, detail="Credenciales inválidas")

    stored_credential.last_used_at = datetime.now(timezone.utc)
    session.add(stored_credential)

    ua_string = request.headers.get("User-Agent", "")
    client_type = detect_client_type(request)
    duration = session_duration_for(client_type)

    new_session = SessionModel(
        user_id=user.id,
        expires_at=datetime.now(timezone.utc) + duration,
        ip_address=ip,
        user_agent=ua_string,
        totp_verified=True,
        auth_method="passkey",
        device_info=parse_device_info(ua_string),
        location=ip,
        client_type=client_type,
    )
    session.add(new_session)
    session.commit()
    background_tasks.add_task(update_session_location, new_session.session_id, ip)

    response = Response(status_code=200)
    set_session_cookie(response, new_session.session_id, int(duration.total_seconds()))
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
        expires = db_session.expires_at.replace(tzinfo=timezone.utc)
        duration = session_duration_for(db_session.client_type)
        response = Response(status_code=200)
        if expires - now < duration / 2:
            db_session.expires_at = now + duration
            session.add(db_session)
            session.commit()
            # Refresh the cookie's own Max-Age too, otherwise the client drops it at its
            # original expiry regardless of this server-side renewal. Caddy's forward_auth
            # must have `copy_headers Set-Cookie` for this to actually reach the client.
            set_session_cookie(response, db_session.session_id, int(duration.total_seconds()))
        return response

    # Back to the same path after signing in. Only ever a path: the login page also refuses
    # anything that would leave this host.
    original_uri = request.headers.get("X-Forwarded-Uri", "/")
    if not original_uri.startswith("/") or original_uri.startswith("//"):
        original_uri = "/"
    return RedirectResponse(url=f"{PUBLIC_URL}/login/?next={quote(original_uri, safe='')}", status_code=302)
