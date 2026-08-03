import os
import base64
import secrets
from datetime import datetime, timedelta, timezone

# API imports
from fastapi import APIRouter, Depends, HTTPException, Request, Response
from fastapi.responses import RedirectResponse

# Database imports
from database import get_session
from sqlmodel import Session as DBSession, select
from models import AllowedEmail, PasskeyCredential, Session as SessionModel, User

# Passkey authentication imports
import webauthn
from dependencies import require_allowed_email
from webauthn import generate_authentication_options, verify_authentication_response
from webauthn.helpers import options_to_json
from webauthn.helpers.structs import (
    AuthenticatorSelectionCriteria,
    PublicKeyCredentialDescriptor,
    UserVerificationRequirement,
)

'''

Auxiliar functions used to encode ...

'''

def b64url_encode(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()

def b64url_decode(data: str) -> bytes:
    padding = "=" * (-len(data) % 4)
    return base64.urlsafe_b64decode(data + padding)


router = APIRouter(prefix="/auth/passkey")

RP_ID = os.environ["RP_ID"] # Relying Party ID as migueltaibo.com to use the authentication on all migueltaibo.com subdomains (like cloud.migueltaibo.com)
RP_NAME = os.environ["RP_NAME"] # Relying Party Name, just a text naming the passkey on the user keychain, just cosmetic
ORIGIN = os.environ["ORIGIN"] # URL where the navigator.credentials.create()/.get() is made

SESSION_DURATION = timedelta(hours=1)

_pending_registration_challenges: dict[str, bytes] = {}
_pending_login_challenges: dict[str, bytes] = {}
_pending_states: dict[str, str] = {}


@router.post("/register/begin")
def register_begin( email: str = Depends(require_allowed_email) ):

    # Options needed for the webauthn to generate a passkey registration
    options = webauthn.generate_registration_options(

        rp_id=RP_ID,
        rp_name=RP_NAME,
        user_name=email,
        user_id=email.encode(),
        user_display_name=email.split("@")[0],

        # Authenticator type and options allowed
        authenticator_selection=AuthenticatorSelectionCriteria(
            user_verification=UserVerificationRequirement.REQUIRED, # Require Biometric verification
        ),
    )

    # Register to keep the challenge of the new register to check it on register_complete
    _pending_registration_challenges[email] = options.challenge

    # Send options to the navigator after converting it to json
    return Response(content=options_to_json(options), media_type="application/json")


@router.post("/register/complete")
def register_complete(
    request_body: dict,
    email: str = Depends(require_allowed_email),
    session: DBSession = Depends(get_session),
):

    # Recover the challenge asigned to the registration on register_begin
    challenge = _pending_registration_challenges.get(email)
    if not challenge: raise HTTPException(status_code=400, detail="Not pending register for this email")

    # Check or verify if the registration was correct, with the expected challenge, rp_id and origin
    try:
        verification = webauthn.verify_registration_response(
            credential=request_body,
            expected_challenge=challenge,
            expected_rp_id=RP_ID,
            expected_origin=ORIGIN,
        )
    except Exception: raise HTTPException(status_code=400, detail="Passkey verification failed")

    # Get/create the user asigned to the new passkey
    user = session.exec(select(User).where(User.email == email)).first()
    if not user:
        user = User(email=email, display_name=email.split("@")[0])
        session.add(user)
        session.commit()
        session.refresh(user)

    # Save the new passkey credential on the  database
    credential = PasskeyCredential(
        user_id=user.id,
        credential_id=b64url_encode(verification.credential_id),
        public_key=b64url_encode(verification.credential_public_key),
        device_name=request_body.get("device_name", "unnamed device"),
    )
    session.add(credential)

    # Set the email that was used to register as used on the database
    allowed = session.exec(select(AllowedEmail).where(AllowedEmail.email == email)).first()
    allowed.used = True
    session.add(allowed)

    # Commit changes to the database and remove the pending registration challenge from its dict
    session.commit()
    del _pending_registration_challenges[email]

    return {"status": "passkey registered", "user_id": user.id}


@router.post("/login/begin")
def login_begin(email: str, session: DBSession = Depends(get_session)):

    # 
    user = session.exec(select(User).where(User.email == email)).first()
    if not user: raise HTTPException(status_code=404, detail="user not found")

    #
    credentials = session.exec(select(PasskeyCredential).where(PasskeyCredential.user_id == user.id)).all()
    if not credentials: raise HTTPException(status_code=404, detail="not passkeys found for this user")

    #
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
    redirect: str,
    state: str,
    session: DBSession = Depends(get_session),
):
    challenge = _pending_login_challenges.get(email)
    if not challenge:
        raise HTTPException(status_code=400, detail="no hay login pendiente para este email")

    user = session.exec(select(User).where(User.email == email)).first()
    if not user:
        raise HTTPException(status_code=401, detail="credenciales inválidas")

    credential_id = request_body.get("id")
    stored_credential = session.exec(
        select(PasskeyCredential).where(
            PasskeyCredential.credential_id == credential_id,
            PasskeyCredential.user_id == user.id,
        )
    ).first()

    if not stored_credential:
        raise HTTPException(status_code=401, detail="credenciales inválidas")

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
        raise HTTPException(status_code=401, detail="firma inválida")

    stored_credential.last_used_at = datetime.now(timezone.utc)
    session.add(stored_credential)

    new_session = SessionModel(
        user_id=user.id,
        expires_at=datetime.now(timezone.utc) + SESSION_DURATION,
    )
    session.add(new_session)
    session.commit()
    del _pending_login_challenges[email]

    response = RedirectResponse(url=f"https://{redirect}?state={state}")
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


@router.get("/validate")
def validate(request: Request, session: DBSession = Depends(get_session)):
    session_id = request.cookies.get("session_id")

    if session_id:
        db_session = session.exec(
            select(SessionModel).where(SessionModel.session_id == session_id)
        ).first()
        if db_session and db_session.expires_at > datetime.now(timezone.utc):
            return Response(status_code=200)

    original_host = request.headers.get("X-Forwarded-Host", "")
    original_uri = request.headers.get("X-Forwarded-Uri", "/")
    original_url = f"https://{original_host}{original_uri}"

    state = secrets.token_urlsafe(32)
    _pending_states[state] = original_url

    login_url = f"https://login.migueltaibo.com/auth?redirect={original_host}&state={state}"
    return RedirectResponse(url=login_url, status_code=302)
