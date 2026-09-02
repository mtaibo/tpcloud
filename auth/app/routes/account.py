import os
import base64
from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, Request
from fastapi.responses import Response
from sqlmodel import Session as DBSession, select

from database import get_session
from dependencies import require_login
from models import PasskeyCredential, Session as SessionModel, User
import webauthn
from webauthn.helpers import options_to_json
from webauthn.helpers.structs import AuthenticatorSelectionCriteria, UserVerificationRequirement

router = APIRouter(prefix="/auth/account")

RP_ID = os.environ["RP_ID"]
RP_NAME = os.environ["RP_NAME"]
ORIGIN = os.environ["ORIGIN"]

_pending_add_challenges: dict[int, bytes] = {}


def b64url_encode(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()

def b64url_decode(data: str) -> bytes:
    padding = "=" * (-len(data) % 4)
    return base64.urlsafe_b64decode(data + padding)


@router.get("/profile")
def get_profile(
    request: Request,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    passkeys = session.exec(
        select(PasskeyCredential).where(PasskeyCredential.user_id == user.id)
    ).all()

    sessions = session.exec(
        select(SessionModel).where(
            SessionModel.user_id == user.id,
            SessionModel.expires_at > datetime.now(timezone.utc).replace(tzinfo=None),
        )
    ).all()

    current_session_id = request.cookies.get("session_id")

    return {
        "id": user.id,
        "email": user.email,
        "display_name": user.display_name,
        "is_admin": user.is_admin,
        "passkeys": [
            {
                "credential_id": p.credential_id,
                "device_name": p.device_name,
                "created_at": p.created_at.isoformat(),
                "last_used_at": p.last_used_at.isoformat() if p.last_used_at else None,
            }
            for p in passkeys
        ],
        "sessions": [
            {
                "session_id": s.session_id,
                "created_at": s.created_at.isoformat(),
                "expires_at": s.expires_at.isoformat(),
                "ip_address": s.ip_address,
                "current": s.session_id == current_session_id,
            }
            for s in sessions
        ],
    }


@router.patch("/profile")
def update_profile(
    body: dict,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    new_name = body.get("display_name", "").strip()
    if not new_name:
        raise HTTPException(status_code=400, detail="display_name no puede estar vacío")
    user.display_name = new_name
    session.add(user)
    session.commit()
    return {"display_name": user.display_name}


@router.delete("/passkey/{credential_id}")
def delete_passkey(
    credential_id: str,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    passkeys = session.exec(
        select(PasskeyCredential).where(PasskeyCredential.user_id == user.id)
    ).all()

    if len(passkeys) <= 1:
        raise HTTPException(status_code=400, detail="No puedes eliminar tu única passkey")

    target = next((p for p in passkeys if p.credential_id == credential_id), None)
    if not target:
        raise HTTPException(status_code=404, detail="Passkey no encontrada")

    session.delete(target)
    session.commit()
    return {"status": "deleted"}


@router.post("/passkey/add/begin")
def add_passkey_begin(user: User = Depends(require_login)):
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
    _pending_add_challenges[user.id] = options.challenge
    return Response(content=options_to_json(options), media_type="application/json")


@router.post("/passkey/add/complete")
def add_passkey_complete(
    request_body: dict,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    challenge = _pending_add_challenges.get(user.id)
    if not challenge:
        raise HTTPException(status_code=400, detail="No hay registro pendiente")

    try:
        verification = webauthn.verify_registration_response(
            credential=request_body,
            expected_challenge=challenge,
            expected_rp_id=RP_ID,
            expected_origin=ORIGIN,
        )
    except Exception:
        raise HTTPException(status_code=400, detail="Verificación fallida")

    credential = PasskeyCredential(
        user_id=user.id,
        credential_id=b64url_encode(verification.credential_id),
        public_key=b64url_encode(verification.credential_public_key),
        device_name=request_body.get("device_name", "nuevo dispositivo"),
    )
    session.add(credential)
    session.commit()
    del _pending_add_challenges[user.id]

    return {"status": "passkey añadida"}


@router.delete("/session/{session_id}")
def revoke_session(
    session_id: str,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    db_session = session.exec(
        select(SessionModel).where(
            SessionModel.session_id == session_id,
            SessionModel.user_id == user.id,
        )
    ).first()

    if not db_session:
        raise HTTPException(status_code=404, detail="Sesión no encontrada")

    session.delete(db_session)
    session.commit()
    return {"status": "revocada"}
