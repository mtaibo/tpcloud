import asyncio
from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import BaseModel
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import WishlistItem
from app import riot_client

router = APIRouter(prefix="/api/val/wishlist", tags=["wishlist"])


class AddItemBody(BaseModel):
    skin_uuid: str
    priority: int = 0
    note: str | None = None


class UpdateItemBody(BaseModel):
    priority: int | None = None
    note: str | None = None


@router.get("")
async def list_wishlist(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    rows = db.exec(
        select(WishlistItem)
        .where(WishlistItem.user_email == user["email"])
        .order_by(WishlistItem.priority.desc(), WishlistItem.created_at.desc())
    ).all()

    sem = asyncio.Semaphore(5)

    async def enrich(item: WishlistItem) -> dict:
        async with sem:
            info = await riot_client.get_skin_info(item.skin_uuid)
        return {
            "id": item.id,
            "skin_uuid": item.skin_uuid,
            "priority": item.priority,
            "note": item.note,
            "created_at": item.created_at.isoformat(),
            "skin_name": info.get("displayName", item.skin_uuid),
            "display_icon": info.get("displayIcon"),
            "content_tier_uuid": info.get("contentTierUuid"),
            "weapon_type": info.get("weapon", {}).get("displayName"),
        }

    items = await asyncio.gather(*[enrich(r) for r in rows])
    return {"items": items}


@router.post("")
async def add_item(body: AddItemBody, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    existing = db.exec(
        select(WishlistItem).where(
            WishlistItem.user_email == user["email"],
            WishlistItem.skin_uuid == body.skin_uuid,
        )
    ).first()
    if existing:
        raise HTTPException(status_code=409, detail="Skin already in wishlist")
    item = WishlistItem(
        user_email=user["email"],
        skin_uuid=body.skin_uuid,
        priority=body.priority,
        note=body.note,
    )
    db.add(item)
    db.commit()
    db.refresh(item)
    return {"id": item.id, "skin_uuid": item.skin_uuid}


@router.patch("/{item_id}")
async def update_item(item_id: int, body: UpdateItemBody, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    item = db.exec(
        select(WishlistItem).where(
            WishlistItem.id == item_id,
            WishlistItem.user_email == user["email"],
        )
    ).first()
    if not item:
        raise HTTPException(status_code=404, detail="Not found")
    if body.priority is not None:
        item.priority = body.priority
    if body.note is not None:
        item.note = body.note
    db.add(item)
    db.commit()
    return {"ok": True}


@router.delete("/{item_id}")
async def delete_item(item_id: int, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    item = db.exec(
        select(WishlistItem).where(
            WishlistItem.id == item_id,
            WishlistItem.user_email == user["email"],
        )
    ).first()
    if item:
        db.delete(item)
        db.commit()
    return {"ok": True}


@router.get("/matches")
async def wishlist_matches_shop(request: Request, db: Session = Depends(get_session)):
    """Return wishlist items currently in the user's shop (call after shop refresh)."""
    from app.models import ShopCache
    import json

    user = await get_current_user(request)
    cache = db.exec(select(ShopCache).where(ShopCache.user_email == user["email"])).first()
    if not cache:
        return {"matches": []}
    offers = json.loads(cache.offers_json)
    offer_ids = {o.get("offer_id") for o in offers}
    rows = db.exec(
        select(WishlistItem).where(
            WishlistItem.user_email == user["email"],
            WishlistItem.skin_uuid.in_(offer_ids),
        )
    ).all()
    return {"matches": [{"skin_uuid": r.skin_uuid, "id": r.id} for r in rows]}
