from fastapi import APIRouter, Request

from app.auth import get_current_user
from app import esports_client

router = APIRouter(prefix="/api/val/esports", tags=["esports"])


@router.get("/overview")
async def get_overview(request: Request):
    await get_current_user(request)
    data = await esports_client.get_vct_overview()
    return data


@router.get("/leagues")
async def get_leagues(request: Request):
    await get_current_user(request)
    return await esports_client.get_vct_leagues()


@router.get("/schedule")
async def get_schedule(request: Request, league_id: str = ""):
    await get_current_user(request)
    if not league_id:
        leagues = await esports_client.get_vct_leagues()
        if not leagues:
            return {"events": []}
        league_id = leagues[0]["id"]
    data = await esports_client.get_schedule(league_id)
    events = data.get("data", {}).get("schedule", {}).get("events", [])
    return {"events": events}
