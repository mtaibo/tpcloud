import asyncio

from fastapi import APIRouter, Depends, HTTPException, Request
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount, RiotCredentials
from app import riot_client

router = APIRouter(prefix="/api/val/night-market", tags=["night_market"])


@router.get("")
async def get_night_market(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)

    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        raise HTTPException(status_code=404, detail="No linked Riot account")

    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    tokens = await riot_client.resolve_tokens_from_credentials(user["email"], creds)
    if tokens is None:
        return {"requires_credentials": True}

    puuid = tokens.get("puuid") or acc.puuid
    store = await riot_client.get_store(tokens, puuid, region=acc.region)

    bonus = store.get("BonusStore")
    if not bonus:
        return {"active": False, "offers": []}

    raw = bonus.get("BonusStoreOffers", [])
    sem = asyncio.Semaphore(5)

    async def enrich(entry: dict) -> dict:
        offer = entry.get("Offer", {})
        skin_uuid = offer.get("OfferID", "")
        cost_map = offer.get("Cost", {})
        vp_cost = cost_map.get(riot_client.VP_CURRENCY, 0)
        discounted_map = entry.get("DiscountCosts", {})
        discounted = discounted_map.get(riot_client.VP_CURRENCY, vp_cost)
        async with sem:
            info = await riot_client.get_skin_info(skin_uuid)
        return {
            "offer_id": skin_uuid,
            "vp_cost": vp_cost,
            "discounted_cost": discounted,
            "discount_percent": entry.get("DiscountPercent", 0),
            "seen": entry.get("IsSeen", False),
            "skin_name": info.get("displayName", skin_uuid),
            "display_icon": info.get("displayIcon"),
            "content_tier_uuid": info.get("contentTierUuid"),
        }

    offers = await asyncio.gather(*[enrich(x) for x in raw])
    return {
        "active": True,
        "expires_seconds": bonus.get("BonusStoreRemainingDurationInSeconds"),
        "offers": offers,
    }
