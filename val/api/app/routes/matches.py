from fastapi import APIRouter, Depends, HTTPException, Query, Request
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount
from app import henrik_client

router = APIRouter(prefix="/api/val/matches", tags=["matches"])


async def _get_linked(user_email: str, db: Session) -> LinkedAccount:
    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user_email)).first()
    if not acc:
        raise HTTPException(status_code=404, detail="No linked Riot account")
    return acc


def _format_match(match: dict, puuid: str) -> dict:
    if not isinstance(match, dict):
        return {}
    metadata = match.get("metadata", {})
    players_raw = match.get("players", [])
    # Henrik v4: players is a flat list; v3: players is {"all_players": [...]}
    players = players_raw if isinstance(players_raw, list) else players_raw.get("all_players", [])
    teams = match.get("teams", {})

    me = next((p for p in players if p.get("puuid") == puuid), None)
    if not me:
        me = players[0] if players else {}

    team_id = me.get("team_id", "Blue").lower()
    won = teams.get(team_id, {}).get("won", False)
    red = teams.get("red", {})
    blue = teams.get("blue", {})

    return {
        "match_id": metadata.get("match_id"),
        "map": metadata.get("map", {}).get("name"),
        "map_id": metadata.get("map", {}).get("id"),
        "queue": metadata.get("queue", {}).get("name", "Custom"),
        "started_at": metadata.get("started_at"),
        "game_length_ms": metadata.get("game_length_in_ms"),
        "season": metadata.get("season", {}).get("short"),
        "won": won,
        "score": f"{blue.get('rounds_won', 0)}-{red.get('rounds_won', 0)}",
        "my_team": team_id,
        "agent": me.get("agent", {}).get("name"),
        "agent_id": me.get("agent", {}).get("id"),
        "stats": me.get("stats", {}),
        "tier": me.get("currenttier_patched"),
    }


@router.get("")
async def list_matches(
    request: Request,
    db: Session = Depends(get_session),
    mode: str = Query("competitive"),
    size: int = Query(20, ge=1, le=50),
):
    user = await get_current_user(request)
    acc = await _get_linked(user["email"], db)

    raw = await henrik_client.get_matches(acc.region, acc.riot_name, acc.riot_tag, mode=mode, size=size)
    puuid = acc.puuid or ""
    return [_format_match(m, puuid) for m in raw]
