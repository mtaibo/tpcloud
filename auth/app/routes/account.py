import base64
from datetime import datetime, timezone

import bcrypt
from fastapi import APIRouter, Depends, HTTPException, Request
from fastapi.responses import Response
from sqlmodel import Session as DBSession, select

from database import get_session
from dependencies import require_login
from config import ORIGIN, RP_ID, RP_NAME, SESSION_COOKIE
from models import LoginEvent, PasskeyCredential, Session as SessionModel, User
from security import ExpiringDict, user_failures
from sessions import session_json
from utils import device_for_request, iso, record_event
import webauthn
from webauthn.helpers import options_to_json
from webauthn.helpers.structs import (
    AuthenticatorSelectionCriteria,
    PublicKeyCredentialDescriptor,
    ResidentKeyRequirement,
    UserVerificationRequirement,
)

router = APIRouter(prefix="/auth/account")

_pending_add_challenges = ExpiringDict(ttl=300)


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
            SessionModel.expires_at > datetime.now(timezone.utc),
            SessionModel.totp_verified == True,  # noqa: E712
        )
    ).all()

    current_session_id = request.cookies.get(SESSION_COOKIE)

    return {
        "id": user.id,
        "email": user.email,
        "display_name": user.display_name,
        "is_admin": user.is_admin,
        "has_password": user.password_hash is not None,
        "totp_enabled": user.totp_secret is not None,
        "failures_reviewed_at": iso(user.failures_reviewed_at),
        "passkeys": [
            {
                "credential_id": p.credential_id,
                "device_name": p.device_name,
                "created_at": iso(p.created_at),
                "last_used_at": iso(p.last_used_at),
            }
            for p in passkeys
        ],
        "sessions": _newest_first([session_json(s, current_session_id) for s in sessions]),
    }


def _newest_first(sessions: list[dict]) -> list[dict]:
    """This session first, then the rest from the most recently used."""
    sessions.sort(key=lambda s: s["last_seen_at"] or "", reverse=True)
    sessions.sort(key=lambda s: not s["current"])
    return sessions


@router.get("/activity")
def activity(
    limit: int = 100,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    events = session.exec(
        select(LoginEvent)
        .where(LoginEvent.user_id == user.id)
        .order_by(LoginEvent.at.desc())
        .limit(max(1, min(limit, 500)))
    ).all()
    return [
        {
            "id": e.id,
            "at": iso(e.at),
            "kind": e.kind,
            "method": e.method,
            "detail": e.detail,
            "ip_address": e.ip_address,
            "location": e.location,
            "device_info": e.device_info,
            "client_type": e.client_type,
        }
        for e in events
    ]


@router.post("/failures/reviewed")
def failures_reviewed(
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    """The owner says the failed sign-ins so far were theirs: the warning goes away everywhere
    until there's a new one."""
    user.failures_reviewed_at = datetime.now(timezone.utc)
    session.add(user)
    session.commit()
    return {"failures_reviewed_at": iso(user.failures_reviewed_at)}


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


@router.patch("/password")
def change_password(
    body: dict,
    request: Request,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    new_password = str(body.get("new_password", "")).strip()
    if not new_password or len(new_password) < 12:
        raise HTTPException(status_code=400, detail="La contraseña debe tener al menos 12 caracteres")

    if user.password_hash:
        user_key = f"user:{user.id}"
        user_failures.check(user_key)
        current_password = str(body.get("current_password", ""))
        if not bcrypt.checkpw(current_password.encode(), user.password_hash.encode()):
            user_failures.fail(user_key)
            raise HTTPException(status_code=401, detail="Contraseña actual incorrecta")

    db_user = session.get(User, user.id)
    db_user.password_hash = bcrypt.hashpw(new_password.encode(), bcrypt.gensalt()).decode()
    session.add(db_user)
    # Whoever knew the old password is signed out everywhere except here.
    current_session_id = request.cookies.get(SESSION_COOKIE)
    for s in session.exec(select(SessionModel).where(SessionModel.user_id == user.id)).all():
        if s.session_id != current_session_id:
            session.delete(s)
    session.commit()

    return {"status": "Contraseña actualizada"}


@router.delete("/passkey/{credential_id}")
def delete_passkey(
    credential_id: str,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    passkeys = session.exec(
        select(PasskeyCredential).where(PasskeyCredential.user_id == user.id)
    ).all()

    target = next((p for p in passkeys if p.credential_id == credential_id), None)
    if not target:
        raise HTTPException(status_code=404, detail="Passkey no encontrada")

    # Prevent lockout: require either a password or another passkey
    has_password = user.password_hash is not None
    if not has_password and len(passkeys) <= 1:
        raise HTTPException(status_code=400, detail="No puedes eliminar tu única passkey sin tener contraseña configurada")

    session.delete(target)
    session.commit()
    return {"status": "deleted"}


@router.post("/passkey/add/begin")
def add_passkey_begin(user: User = Depends(require_login), session: DBSession = Depends(get_session)):
    existing = session.exec(select(PasskeyCredential).where(PasskeyCredential.user_id == user.id)).all()
    options = webauthn.generate_registration_options(
        rp_id=RP_ID,
        rp_name=RP_NAME,
        user_name=user.email,
        user_id=user.email.encode(),
        user_display_name=user.display_name,
        # Discoverable, so signing in with it doesn't need the email typed first.
        authenticator_selection=AuthenticatorSelectionCriteria(
            resident_key=ResidentKeyRequirement.REQUIRED,
            user_verification=UserVerificationRequirement.REQUIRED,
        ),
        exclude_credentials=[PublicKeyCredentialDescriptor(id=b64url_decode(c.credential_id)) for c in existing],
    )
    _pending_add_challenges.set(user.id, options.challenge)
    return Response(content=options_to_json(options), media_type="application/json")


@router.post("/passkey/add/complete")
def add_passkey_complete(
    request_body: dict,
    request: Request,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    challenge = _pending_add_challenges.pop(user.id)
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
        device_name=(str(request_body.get("device_name") or "").strip() or device_for_request(request)["label"])[:100],
    )
    session.add(credential)
    session.commit()

    return {"status": "passkey añadida"}


@router.delete("/session/{session_row_id}")
def revoke_session(
    session_row_id: int,
    request: Request,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    db_session = session.exec(
        select(SessionModel).where(
            SessionModel.id == session_row_id,
            SessionModel.user_id == user.id,
        )
    ).first()

    if not db_session:
        raise HTTPException(status_code=404, detail="Sesión no encontrada")

    record_event(request, user.id, "revoked", db_session.auth_method, db_session.device_info)
    session.delete(db_session)
    session.commit()
    return {"status": "revocada"}


@router.delete("/sessions/others")
def revoke_other_sessions(
    request: Request,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    current_session_id = request.cookies.get(SESSION_COOKIE)
    others = [
        s for s in session.exec(select(SessionModel).where(SessionModel.user_id == user.id)).all()
        if s.session_id != current_session_id
    ]
    for s in others:
        session.delete(s)
    session.commit()
    if others:
        record_event(request, user.id, "revoked", None, f"Cerraste las demás sesiones ({len(others)})")
    return {"revoked": len(others)}
