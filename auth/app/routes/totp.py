import pyotp
from fastapi import APIRouter, Depends, HTTPException
from sqlmodel import Session as DBSession

from database import get_session
from dependencies import require_login
from config import RP_NAME
from models import User
from security import user_failures, verify_totp

router = APIRouter(prefix="/auth/account/totp")

_pending_totp_secrets: dict[int, str] = {}


@router.get("/status")
def totp_status(user: User = Depends(require_login)):
    return {"totp_enabled": user.totp_secret is not None}


@router.post("/setup/begin")
def totp_setup_begin(user: User = Depends(require_login)):
    secret = pyotp.random_base32()
    _pending_totp_secrets[user.id] = secret
    uri = pyotp.TOTP(secret).provisioning_uri(name=user.email, issuer_name=RP_NAME)
    return {"secret": secret, "otpauth_uri": uri}


@router.post("/setup/complete")
def totp_setup_complete(
    body: dict,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    secret = _pending_totp_secrets.get(user.id)
    if not secret:
        raise HTTPException(status_code=400, detail="No hay configuración TOTP pendiente")

    user_key = f"user:{user.id}"
    user_failures.check(user_key)
    if not verify_totp(secret, str(body.get("code", "")), user.id):
        user_failures.fail(user_key)
        raise HTTPException(status_code=401, detail="Código incorrecto")

    db_user = session.get(User, user.id)
    db_user.totp_secret = secret
    session.add(db_user)
    session.commit()
    del _pending_totp_secrets[user.id]

    return {"status": "TOTP configurado"}


@router.delete("")
def totp_disable(
    body: dict,
    user: User = Depends(require_login),
    session: DBSession = Depends(get_session),
):
    if not user.totp_secret:
        raise HTTPException(status_code=400, detail="TOTP no está configurado")

    user_key = f"user:{user.id}"
    user_failures.check(user_key)
    if not verify_totp(user.totp_secret, str(body.get("code", "")), user.id):
        user_failures.fail(user_key)
        raise HTTPException(status_code=401, detail="Código incorrecto")

    db_user = session.get(User, user.id)
    db_user.totp_secret = None
    session.add(db_user)
    session.commit()

    return {"status": "TOTP desactivado"}
