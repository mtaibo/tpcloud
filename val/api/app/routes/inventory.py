import asyncio
import logging

from fastapi import APIRouter, Depends, HTTPException, Query, Request
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount, RiotCredentials
from app import crypto, riot_client

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/api/val/inventory", tags=["inventory"])

WEAPON_CATEGORIES = {
    "Vandal": "63e6c2b6-4a8e-869c-3d4c-e38355226584",
    "Phantom": "ee8e8d15-496b-07ac-e5f6-8fae5d4c7b1a",
    "Operator": "a03b24d3-4319-996d-0f8c-94bbfba1dfc7",
    "Ghost": "29a0cfab-485b-f5d5-779a-b59f85e204a8",
    "Classic": "29a0cfab-485b-f5d5-779a-b59f85e204a8",
    "Knife": "2f59173c-4bed-b6c3-2191-dea9b58be9c7",
}


@router.get("")
async def get_inventory(
    request: Request,
    db: Session = Depends(get_session),
    weapon: str = Query(None),
):
    user = await get_current_user(request)

    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        raise HTTPException(status_code=404, detail="No linked Riot account")

    # Fast path: in-memory cached tokens (e.g. from browser OAuth login)
    tokens = riot_client.get_cached_tokens(user["email"])
    if tokens is None:
        creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
        if not creds or not creds.encrypted_ssid:
            return {"requires_credentials": True}

        ssid = crypto.decrypt(creds.encrypted_ssid)
        tokens = await riot_client.get_tokens(user["email"], ssid=ssid)

    puuid = tokens.get("puuid") or acc.puuid
    if not puuid:
        raise HTTPException(status_code=400, detail="PUUID not available")

    raw_items = await riot_client.get_inventory(tokens, puuid, region=acc.region)
    logger.info("Inventory raw_items count=%d, sample=%s", len(raw_items), raw_items[:2])

    # Enrich with names and images (concurrent but rate-limited)
    skin_uuids = [item.get("ItemID", "") for item in raw_items if item.get("ItemID")]
    logger.info("Skin UUIDs to enrich: %s", skin_uuids[:3])
    sem = asyncio.Semaphore(5)

    async def fetch_one(uuid: str) -> dict:
        async with sem:
            return await riot_client.get_skin_info(uuid)

    infos = await asyncio.gather(*[fetch_one(u) for u in skin_uuids[:100]])

    skins = []
    for uuid, info in zip(skin_uuids[:100], infos):
        if not info:
            continue
        skins.append({
            "uuid": uuid,
            "name": info.get("displayName", uuid),
            "display_icon": info.get("displayIcon"),
            "weapon_type": info.get("weapon", {}).get("displayName"),
            "weapon_uuid": info.get("weapon", {}).get("uuid"),
            "content_tier_uuid": info.get("contentTierUuid"),
        })

    if weapon:
        skins = [s for s in skins if (s.get("weapon_type") or "").lower() == weapon.lower()]

    return {"skins": skins, "total": len(raw_items)}
