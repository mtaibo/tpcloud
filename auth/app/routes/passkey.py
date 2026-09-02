import os
import base64
import secrets
from datetime import datetime, timedelta, timezone

from fastapi import APIRouter, Depends, HTTPException, Request, Response
from fastapi.responses import RedirectResponse
from database import get_session
from sqlmodel import Session as DBSession, select
from models import PasskeyCredential, Session as SessionModel, User

import webauthn
from dependencies import require_password_session
from webauthn import generate_authentication_options, verify_authentication_response
from webauthn.helpers import options_to_json
from webauthn.helpers.structs import (
    AuthenticatorSelectionCriteria,
    PublicKeyCredentialDescriptor,
    UserVerificationRequirement,
)
from utils import get_client_ip, lookup_location, parse_device_info


def b64url_encode(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()

def b64url_decode(data: str) -> bytes:
    padding = "=" * (-len(data) % 4)
    return base64.urlsafe_b64decode(data + padding)


router = APIRouter(prefix="/auth/passkey")

RP_ID = os.environ["RP_ID"]
RP_NAME = os.environ["RP_NAME"]
ORIGIN = os.environ["ORIGIN"]
ACCOUNTS_ORIGIN = os.environ["ACCOUNTS_ORIGIN"]

SESSION_DURATION = timedelta(hours=1)

_pending_registration_challenges: dict[str, bytes] = {}
_pending_login_challenges: dict[str, bytes] = {}
_pending_states: dict[str, str] = {}


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
    _pending_registration_challenges[user.email] = options.challenge
    return Response(content=options_to_json(options), media_type="application/json")


@router.post("/register/complete")
def register_complete(
    request_body: dict,
    user: User = Depends(require_password_session),
    session: DBSession = Depends(get_session),
):
    challenge = _pending_registration_challenges.get(user.email)
    if not challenge:
        raise HTTPException(status_code=400, detail="No hay registro pendiente para este usuario")

    try:
        verification = webauthn.verify_registration_response(
            credential=request_body,
            expected_challenge=challenge,
            expected_rp_id=RP_ID,
            expected_origin=ACCOUNTS_ORIGIN,
        )
    except Exception:
        raise HTTPException(status_code=400, detail="Verificación de passkey fallida")

    credential = PasskeyCredential(
        user_id=user.id,
        credential_id=b64url_encode(verification.credential_id),
        public_key=b64url_encode(verification.credential_public_key),
        device_name=request_body.get("device_name", "nuevo dispositivo"),
    )
    session.add(credential)
    session.commit()
    del _pending_registration_challenges[user.email]

    return {"status": "passkey registrada", "user_id": user.id}


@router.post("/login/begin")
def login_begin(email: str, session: DBSession = Depends(get_session)):
    user = session.exec(select(User).where(User.email == email)).first()
    if not user:
        raise HTTPException(status_code=404, detail="Usuario no encontrado")

    credentials = session.exec(
        select(PasskeyCredential).where(PasskeyCredential.user_id == user.id)
    ).all()
    if not credentials:
        raise HTTPException(status_code=404, detail="No hay passkeys para este usuario")

    allow_credentials = [
        PublicKeyCredentialDescriptor(id=b64url_decode(c.credential_id))
        for c in credentials
    ]

    options = generate_authentication_options(
        rp_id=RP_ID,
        allow_credentials=allow_credentials,
    )
    _pending_login_challenges[email] = options.challenge

    return Response(content=options_to_json(options), media_type="application/json")


@router.post("/login/complete")
def login_complete(
    request_body: dict,
    email: str,
    request: Request,
    session: DBSession = Depends(get_session),
):
    challenge = _pending_login_challenges.get(email)
    if not challenge:
        raise HTTPException(status_code=400, detail="No hay login pendiente para este email")

    user = session.exec(select(User).where(User.email == email)).first()
    if not user:
        raise HTTPException(status_code=401, detail="Credenciales inválidas")

    credential_id = request_body.get("id")
    stored_credential = session.exec(
        select(PasskeyCredential).where(
            PasskeyCredential.credential_id == credential_id,
            PasskeyCredential.user_id == user.id,
        )
    ).first()
    if not stored_credential:
        raise HTTPException(status_code=401, detail="Credenciales inválidas")

    try:
        verify_authentication_response(
            credential=request_body,
            expected_challenge=challenge,
            expected_rp_id=RP_ID,
            expected_origin=ORIGIN,
            credential_public_key=b64url_decode(stored_credential.public_key),
            credential_current_sign_count=0,
        )
    except Exception:
        raise HTTPException(status_code=401, detail="Firma inválida")

    stored_credential.last_used_at = datetime.now(timezone.utc)
    session.add(stored_credential)

    ip = get_client_ip(request)
    ua_string = request.headers.get("User-Agent", "")

    new_session = SessionModel(
        user_id=user.id,
        expires_at=datetime.now(timezone.utc) + SESSION_DURATION,
        ip_address=ip,
        user_agent=ua_string,
        totp_verified=True,
        auth_method="passkey",
        device_info=parse_device_info(ua_string),
        location=lookup_location(ip),
    )
    session.add(new_session)
    session.commit()
    del _pending_login_challenges[email]

    response = Response(status_code=200)
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


@router.get("/check")
def check(request: Request, session: DBSession = Depends(get_session)):
    session_id = request.cookies.get("session_id")
    if session_id:
        db_session = session.exec(
            select(SessionModel).where(SessionModel.session_id == session_id)
        ).first()
        if (
            db_session
            and db_session.expires_at.replace(tzinfo=timezone.utc) > datetime.now(timezone.utc)
            and db_session.totp_verified
        ):
            return Response(status_code=200)
    raise HTTPException(status_code=401, detail="no autenticado")


@router.get("/me")
def me(request: Request, session: DBSession = Depends(get_session)):
    session_id = request.cookies.get("session_id")
    if session_id:
        db_session = session.exec(
            select(SessionModel).where(SessionModel.session_id == session_id)
        ).first()
        if (
            db_session
            and db_session.expires_at.replace(tzinfo=timezone.utc) > datetime.now(timezone.utc)
            and db_session.totp_verified
        ):
            user = session.exec(select(User).where(User.id == db_session.user_id)).first()
            if user:
                return {
                    "email": user.email,
                    "display_name": user.display_name,
                    "is_admin": user.is_admin,
                }
    raise HTTPException(status_code=401, detail="no autenticado")


@router.post("/logout")
def logout(request: Request, session: DBSession = Depends(get_session)):
    session_id = request.cookies.get("session_id")
    if session_id:
        db_session = session.exec(
            select(SessionModel).where(SessionModel.session_id == session_id)
        ).first()
        if db_session:
            session.delete(db_session)
            session.commit()
    response = Response(status_code=200)
    response.delete_cookie(key="session_id", domain=".migueltaibo.com", path="/")
    return response


@router.get("/validate")
def validate(request: Request, session: DBSession = Depends(get_session)):
    session_id = request.cookies.get("session_id")

    if session_id:
        db_session = session.exec(
            select(SessionModel).where(SessionModel.session_id == session_id)
        ).first()
        if (
            db_session
            and db_session.expires_at.replace(tzinfo=timezone.utc) > datetime.now(timezone.utc)
            and db_session.totp_verified
        ):
            return Response(status_code=200)

    original_host = request.headers.get("X-Forwarded-Host", "")
    original_uri = request.headers.get("X-Forwarded-Uri", "/")
    original_url = f"https://{original_host}{original_uri}"

    state = secrets.token_urlsafe(32)
    _pending_states[state] = original_url

    login_url = f"https://login.migueltaibo.com/?redirect={original_host}&state={state}"
    return RedirectResponse(url=login_url, status_code=302)
