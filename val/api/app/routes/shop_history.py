import asyncio
import json

from fastapi import APIRouter, Depends, Query, Request
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import ShopHistoryEntry
from app import riot_client

router = APIRouter(prefix="/api/val/shop-history", tags=["shop_history"])


@router.get("")
async def list_history(
    request: Request,
    db: Session = Depends(get_session),
    limit: int = Query(30, ge=1, le=120),
):
    user = await get_current_user(request)
    rows = db.exec(
        select(ShopHistoryEntry)
        .where(ShopHistoryEntry.user_email == user["email"])
        .order_by(ShopHistoryEntry.shop_date.desc())
        .limit(limit)
    ).all()

    return {
        "entries": [
            {
                "shop_date": r.shop_date,
                "offers": json.loads(r.offers_json),
                "bundle": json.loads(r.bundle_json) if r.bundle_json else None,
                "recorded_at": r.recorded_at.isoformat(),
            }
            for r in rows
        ]
    }
