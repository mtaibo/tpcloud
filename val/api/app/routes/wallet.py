from fastapi import APIRouter, Depends, HTTPException, Request
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount, RiotCredentials
from app import riot_client

router = APIRouter(prefix="/api/val/wallet", tags=["wallet"])


@router.get("")
async def get_wallet(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)

    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        raise HTTPException(status_code=404, detail="No linked Riot account")

    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    tokens = await riot_client.resolve_tokens_from_credentials(user["email"], creds)
    if tokens is None:
        return {"requires_credentials": True}

    puuid = tokens.get("puuid") or acc.puuid
    if not puuid:
        raise HTTPException(status_code=400, detail="PUUID not available")

    return await riot_client.get_wallet(tokens, puuid, region=acc.region)
