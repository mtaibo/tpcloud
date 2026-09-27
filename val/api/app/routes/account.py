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


class CredentialsBody(BaseModel):
    username: str
    password: str


@router.get("")
async def get_account(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        return None
    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    return {
        "riot_name": acc.riot_name,
        "riot_tag": acc.riot_tag,
        "region": acc.region,
        "puuid": acc.puuid,
        "has_credentials": creds is not None,
        "linked_at": acc.created_at.isoformat(),
    }


@router.post("")
async def link_account(body: LinkAccountBody, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)

    # Verify the account exists via Henrik
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


@router.post("/credentials")
async def save_credentials(body: CredentialsBody, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)

    if not crypto.is_configured():
        raise HTTPException(status_code=503, detail="RIOT_CRED_KEY not configured on server")

    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        raise HTTPException(status_code=400, detail="Link a Riot account first")

    # Test credentials before storing
    try:
        tokens = await riot_client.authenticate(body.username, body.password)
    except HTTPException:
        raise
    except Exception:
        raise HTTPException(status_code=401, detail="Invalid Riot credentials")

    # Update PUUID from tokens (authoritative)
    if tokens.get("puuid") and tokens["puuid"] != acc.puuid:
        acc.puuid = tokens["puuid"]
        acc.updated_at = datetime.now(timezone.utc)
        db.add(acc)

    existing = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    enc_user = crypto.encrypt(body.username)
    enc_pass = crypto.encrypt(body.password)

    if existing:
        existing.encrypted_username = enc_user
        existing.encrypted_password = enc_pass
        existing.updated_at = datetime.now(timezone.utc)
        db.add(existing)
    else:
        db.add(RiotCredentials(
            user_email=user["email"],
            encrypted_username=enc_user,
            encrypted_password=enc_pass,
        ))

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
