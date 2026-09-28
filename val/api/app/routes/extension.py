import json
import logging
import secrets
from datetime import datetime, timezone, timedelta

from fastapi import APIRouter, Depends, Header, HTTPException, Request
from pydantic import BaseModel
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount, RiotCredentials
from app import crypto, riot_client

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/api/val/account", tags=["extension"])

SESSION_DURATION_DAYS = 21
RESYNC_THRESHOLD_HOURS = 24


class ExtensionSyncBody(BaseModel):
    cookies: dict
    extension_version: str | None = None


def _now() -> datetime:
    return datetime.now(timezone.utc)


def _session_status(creds: RiotCredentials | None) -> dict:
    if creds is None or not creds.encrypted_cookies:
        return {
            "has_extension_synced": False,
            "last_sync_at": None,
            "session_expires_at": None,
            "needs_resync": True,
            "days_remaining": 0,
        }
    now = _now()
    expires = creds.session_expires_at
    days = 0
    if expires:
        # SQLite may return naive datetime; normalise to UTC
        if expires.tzinfo is None:
            expires = expires.replace(tzinfo=timezone.utc)
        days = max(0, (expires - now).days)
    needs_resync = creds.needs_resync or (expires is not None and expires < now + timedelta(hours=RESYNC_THRESHOLD_HOURS))
    return {
        "has_extension_synced": True,
        "last_sync_at": creds.last_sync_at.isoformat() if creds.last_sync_at else None,
        "session_expires_at": expires.isoformat() if expires else None,
        "needs_resync": bool(needs_resync),
        "days_remaining": days,
    }


@router.post("/extension/pair-token")
async def create_pair_token(request: Request, db: Session = Depends(get_session)):
    """Generate a fresh pair token for the extension. Overwrites any previous one for this user."""
    user = await get_current_user(request)
    email = user["email"]

    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == email)).first()
    if not acc:
        raise HTTPException(status_code=400, detail="Link a Riot account first")

    token = secrets.token_urlsafe(24)
    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == email)).first()
    if creds:
        creds.pair_token = token
        creds.updated_at = _now()
        db.add(creds)
    else:
        db.add(RiotCredentials(user_email=email, pair_token=token))
    db.commit()

    return {"pair_token": token}


@router.get("/session")
async def get_session_status(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    return _session_status(creds)


@router.post("/extension/sync")
async def extension_sync(
    body: ExtensionSyncBody,
    x_tpval_extension_token: str = Header(default=""),
    db: Session = Depends(get_session),
):
    """Receive cookies from the browser extension. Auth via pair_token header — NOT passkey."""
    if not crypto.is_configured():
        raise HTTPException(status_code=503, detail="RIOT_CRED_KEY not configured on server")
    token = x_tpval_extension_token.strip()
    if not token:
        raise HTTPException(status_code=401, detail="Missing X-TPVal-Extension-Token")

    creds = db.exec(select(RiotCredentials).where(RiotCredentials.pair_token == token)).first()
    if not creds:
        raise HTTPException(status_code=401, detail="Invalid pair token")

    cookies = {k: v for k, v in body.cookies.items() if v}
    if "ssid" not in cookies:
        raise HTTPException(status_code=400, detail="Missing ssid cookie")

    try:
        tokens = await riot_client.auth_with_cookies(creds.user_email, cookies)
    except HTTPException as e:
        creds.needs_resync = True
        creds.updated_at = _now()
        db.add(creds)
        db.commit()
        raise e

    refreshed_cookies = tokens.get("_cookies") or cookies
    merged = {**cookies, **refreshed_cookies}
    creds.encrypted_cookies = crypto.encrypt(json.dumps(merged))
    creds.last_sync_at = _now()
    creds.session_expires_at = _now() + timedelta(days=SESSION_DURATION_DAYS)
    creds.needs_resync = False
    creds.extension_version = body.extension_version
    creds.updated_at = _now()

    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == creds.user_email)).first()
    if acc and tokens.get("puuid") and tokens["puuid"] != acc.puuid:
        acc.puuid = tokens["puuid"]
        acc.updated_at = _now()
        db.add(acc)

    db.add(creds)
    db.commit()

    riot_client.cache_tokens(creds.user_email, tokens)
    logger.info("Extension sync SUCCESS for %s (v=%s)", creds.user_email, body.extension_version)

    return {
        "ok": True,
        "puuid": tokens.get("puuid"),
        "session_expires_at": creds.session_expires_at.isoformat(),
    }


@router.post("/extension/revoke")
async def revoke_pair_token(request: Request, db: Session = Depends(get_session)):
    """Clear the pair_token so the extension can no longer sync."""
    user = await get_current_user(request)
    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    if creds:
        creds.pair_token = None
        creds.updated_at = _now()
        db.add(creds)
        db.commit()
    return {"ok": True}
