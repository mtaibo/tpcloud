from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, Request
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount, RankHistory
from app import henrik_client

router = APIRouter(prefix="/api/val/player", tags=["player"])


async def _get_linked(user_email: str, db: Session) -> LinkedAccount:
    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user_email)).first()
    if not acc:
        raise HTTPException(status_code=404, detail="No linked Riot account")
    return acc


@router.get("/profile")
async def get_profile(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    acc = await _get_linked(user["email"], db)

    data = await henrik_client.get_account(acc.riot_name, acc.riot_tag)
    return {
        "name": data.get("name"),
        "tag": data.get("tag"),
        "puuid": data.get("puuid"),
        "region": data.get("region"),
        "account_level": data.get("account_level"),
        "card": data.get("card", {}),
        "title": data.get("title"),
        "last_update": data.get("last_update"),
    }


@router.get("/rank")
async def get_rank(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    acc = await _get_linked(user["email"], db)

    data = await henrik_client.get_mmr(acc.region, acc.riot_name, acc.riot_tag)
    current = data.get("current_data", {})
    highest = data.get("highest_rank", {})

    return {
        "tier": current.get("currenttierpatched"),
        "tier_id": current.get("currenttier"),
        "rr": current.get("ranking_in_tier"),
        "mmr_change": current.get("mmr_change_to_last_game"),
        "elo": current.get("elo"),
        "images": current.get("images", {}),
        "peak": {
            "tier": highest.get("patched_tier"),
            "season": highest.get("season"),
        },
    }


@router.get("/mmr-history")
async def get_mmr_history(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    acc = await _get_linked(user["email"], db)

    # Fetch from Henrik
    raw = await henrik_client.get_mmr_history(acc.region, acc.riot_name, acc.riot_tag)

    # Persist new entries to DB for future charting
    existing_ids = {
        r.match_id for r in db.exec(
            select(RankHistory).where(RankHistory.user_email == user["email"])
        ).all()
    }

    new_entries = []
    for entry in raw:
        match_id = entry.get("match_id", "")
        if match_id and match_id not in existing_ids:
            new_entries.append(RankHistory(
                user_email=user["email"],
                match_id=match_id,
                tier=entry.get("currenttierpatched", ""),
                tier_image_url=entry.get("images", {}).get("small"),
                mmr=entry.get("elo", 0),
                mmr_change=entry.get("mmr_change_to_last_game", 0),
                recorded_at=datetime.now(timezone.utc),
            ))

    if new_entries:
        for e in new_entries:
            db.add(e)
        db.commit()

    return [
        {
            "match_id": e.get("match_id"),
            "tier": e.get("currenttierpatched"),
            "tier_id": e.get("currenttier"),
            "rr": e.get("ranking_in_tier"),
            "mmr": e.get("elo"),
            "mmr_change": e.get("mmr_change_to_last_game"),
            "date": e.get("date"),
            "images": e.get("images", {}),
        }
        for e in raw
    ]
