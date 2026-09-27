from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import BaseModel
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount, RiotCredentials
from app import henrik_client, crypto, riot_client

router = APIRouter(prefix="/api/val/account", tags=["account"])


class LinkAccountBody(BaseModel):
    riot_name: str
    riot_tag: str
    region: str


class CookieAuthBody(BaseModel):
    ssid: str


class TokenAuthBody(BaseModel):
    access_token: str


@router.get("")
async def get_account(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        return None
    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    has_credentials = (
        riot_client.get_cached_tokens(user["email"]) is not None
        or (creds is not None and bool(creds.encrypted_ssid))
    )
    return {
        "riot_name": acc.riot_name,
        "riot_tag": acc.riot_tag,
        "region": acc.region,
        "puuid": acc.puuid,
        "has_credentials": has_credentials,
        "linked_at": acc.created_at.isoformat(),
    }


@router.post("")
async def link_account(body: LinkAccountBody, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)

    try:
        account_data = await henrik_client.get_account(body.riot_name, body.riot_tag)
    except HTTPException as e:
        if e.status_code == 404:
            raise HTTPException(status_code=404, detail="Riot account not found — check name and tag")
        raise

    puuid = account_data.get("puuid")

    existing = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if existing:
        existing.riot_name = body.riot_name
        existing.riot_tag = body.riot_tag
        existing.region = body.region
        existing.puuid = puuid
        existing.updated_at = datetime.now(timezone.utc)
        db.add(existing)
    else:
        acc = LinkedAccount(
            user_email=user["email"],
            riot_name=body.riot_name,
            riot_tag=body.riot_tag,
            region=body.region,
            puuid=puuid,
        )
        db.add(acc)

    db.commit()
    return {"riot_name": body.riot_name, "riot_tag": body.riot_tag, "region": body.region, "puuid": puuid}


@router.delete("")
async def unlink_account(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if acc:
        db.delete(acc)
    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    if creds:
        db.delete(creds)
    db.commit()
    riot_client.invalidate_tokens(user["email"])
    return {"ok": True}


@router.post("/credentials/token")
async def save_credentials_token(body: TokenAuthBody, request: Request, db: Session = Depends(get_session)):
    """Auth using a browser access_token (from Riot OAuth popup). Cached in memory — no persistent storage."""
    user = await get_current_user(request)

    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        raise HTTPException(status_code=400, detail="Link a Riot account first")

    tokens = await riot_client.auth_with_access_token(user["email"], body.access_token.strip())

    if tokens.get("puuid") and tokens["puuid"] != acc.puuid:
        acc.puuid = tokens["puuid"]
        acc.updated_at = datetime.now(timezone.utc)
        db.add(acc)
        db.commit()

    riot_client.cache_tokens(user["email"], tokens)
    return {"ok": True}


@router.post("/credentials/cookie")
async def save_credentials_cookie(body: CookieAuthBody, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)

    if not crypto.is_configured():
        raise HTTPException(status_code=503, detail="RIOT_CRED_KEY not configured on server")

    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        raise HTTPException(status_code=400, detail="Link a Riot account first")

    ssid = body.ssid.strip()
    tokens = await riot_client.auth_with_ssid(user["email"], ssid)

    if tokens.get("puuid") and tokens["puuid"] != acc.puuid:
        acc.puuid = tokens["puuid"]
        acc.updated_at = datetime.now(timezone.utc)
        db.add(acc)

    enc_ssid = crypto.encrypt(ssid)
    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    if creds:
        creds.encrypted_ssid = enc_ssid
        creds.updated_at = datetime.now(timezone.utc)
        db.add(creds)
    else:
        db.add(RiotCredentials(user_email=user["email"], encrypted_ssid=enc_ssid))

    riot_client.cache_tokens(user["email"], tokens)
    db.commit()
    return {"ok": True}


@router.delete("/credentials")
async def delete_credentials(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    if creds:
        db.delete(creds)
        db.commit()
    riot_client.invalidate_tokens(user["email"])
    return {"ok": True}
