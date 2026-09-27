import json
from datetime import datetime, timezone, timedelta

from fastapi import APIRouter, Depends, HTTPException, Request
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount, RiotCredentials, ShopCache
from app import crypto, riot_client

router = APIRouter(prefix="/api/val/shop", tags=["shop"])

SKIN_LEVEL_TYPE = "e7c63390-eda7-46e0-bb9a-14f0a3a9f911"


async def _enrich_offers(raw_offers: list) -> list:
    enriched = []
    for offer in raw_offers:
        offer_id = offer.get("OfferID", "")
        cost_map = offer.get("Cost", {})
        vp_cost = cost_map.get("85ad13f7-3d1b-5128-9eb2-7cd8ee0b5741", 0)  # VP currency ID

        skin_info = await riot_client.get_skin_info(offer_id)
        enriched.append({
            "offer_id": offer_id,
            "vp_cost": vp_cost,
            "skin_name": skin_info.get("displayName", offer_id),
            "display_icon": skin_info.get("displayIcon"),
            "content_tier_uuid": skin_info.get("contentTierUuid"),
        })
    return enriched


def _next_shop_reset() -> datetime:
    now = datetime.now(timezone.utc)
    # Store resets daily at 00:00 UTC
    tomorrow = now.replace(hour=0, minute=0, second=0, microsecond=0) + timedelta(days=1)
    return tomorrow


@router.get("")
async def get_shop(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)

    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        raise HTTPException(status_code=404, detail="No linked Riot account")

    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    if not creds:
        return {"requires_credentials": True}

    # Check cache
    cached = db.exec(select(ShopCache).where(ShopCache.user_email == user["email"])).first()
    now = datetime.now(timezone.utc)
    if cached and cached.expires_at > now:
        return {
            "offers": json.loads(cached.offers_json),
            "bundle": json.loads(cached.bundle_json) if cached.bundle_json else None,
            "expires_at": cached.expires_at.isoformat(),
            "cached": True,
        }

    username = crypto.decrypt(creds.encrypted_username)
    password = crypto.decrypt(creds.encrypted_password)

    tokens = await riot_client.get_tokens(user["email"], username, password)
    puuid = tokens.get("puuid") or acc.puuid
    if not puuid:
        raise HTTPException(status_code=400, detail="PUUID not available")

    store_data = await riot_client.get_store(tokens, puuid, region=acc.region)

    raw_offers = store_data.get("SkinsPanelLayout", {}).get("SingleItemStoreOffers", [])
    offers = await _enrich_offers(raw_offers)

    bundle_raw = store_data.get("FeaturedBundle", {}).get("Bundle")
    bundle = None
    if bundle_raw:
        bundle = {
            "uuid": bundle_raw.get("ID"),
            "name": bundle_raw.get("DataAssetID"),
            "items_count": len(bundle_raw.get("Items", [])),
        }

    expires_at = _next_shop_reset()
    offers_json = json.dumps(offers)
    bundle_json = json.dumps(bundle) if bundle else None

    if cached:
        cached.offers_json = offers_json
        cached.bundle_json = bundle_json
        cached.cached_at = now
        cached.expires_at = expires_at
        db.add(cached)
    else:
        db.add(ShopCache(
            user_email=user["email"],
            offers_json=offers_json,
            bundle_json=bundle_json,
            expires_at=expires_at,
        ))
    db.commit()

    return {
        "offers": offers,
        "bundle": bundle,
        "expires_at": expires_at.isoformat(),
        "cached": False,
    }


@router.post("/refresh")
async def refresh_shop(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    cached = db.exec(select(ShopCache).where(ShopCache.user_email == user["email"])).first()
    if cached:
        db.delete(cached)
        db.commit()
    riot_client.invalidate_tokens(user["email"])
    return {"ok": True}
