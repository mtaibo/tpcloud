import logging

from fastapi import APIRouter, Depends, HTTPException, Query, Request
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount
from app import henrik_client

router = APIRouter(prefix="/api/val/matches", tags=["matches"])
logger = logging.getLogger(__name__)

FALLBACK_REGION = "eu"


async def _resolve_player(user_email: str, db: Session, name: str | None, tag: str | None, region: str | None):
    if name and tag:
        return name, tag, region or FALLBACK_REGION, None
    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user_email)).first()
    if not acc:
        raise HTTPException(status_code=404, detail="No linked Riot account")
    return acc.riot_name, acc.riot_tag, acc.region, acc.puuid


def _team_rounds(team: dict) -> int:
    """Extract rounds won from a team object, handling different API versions."""
    v = team.get("rounds_won")
    if v is not None:
        return int(v)
    nested = team.get("rounds")
    if isinstance(nested, dict):
        return int(nested.get("won", 0))
    return 0


def _format_match(match: dict, puuid: str, *, _logged: list) -> dict:
    if not isinstance(match, dict):
        return {}
    metadata = match.get("metadata", {})
    players_raw = match.get("players", [])
    # Henrik v4: players is a flat list; v3: players is {"all_players": [...]}
    players = players_raw if isinstance(players_raw, list) else players_raw.get("all_players", [])
    teams_raw = match.get("teams", {})

    # Henrik v4: teams is a list; v3: teams is a dict keyed by team name
    if isinstance(teams_raw, list):
        teams = {t.get("team_id", "").lower(): t for t in teams_raw}
    else:
        teams = teams_raw

    me = next((p for p in players if p.get("puuid") == puuid), None)
    if not me:
        me = players[0] if players else {}

    # Log first match structure once per request for debugging
    if not _logged:
        _logged.append(True)
        logger.info("Henrik match debug — teams keys: %s, team sample: %s, player keys: %s, agent field: %s",
                    list(teams.keys()),
                    next(iter(teams.values()), {}),
                    list(me.keys())[:15] if me else [],
                    me.get("agent") if me else None)

    team_id = me.get("team_id", "Blue").lower()
    won = teams.get(team_id, {}).get("won", False)
    red = teams.get("red", {})
    blue = teams.get("blue", {})

    agent_obj = me.get("agent") or {}
    if isinstance(agent_obj, str):
        agent_name, agent_id, agent_img = agent_obj, None, None
    else:
        agent_name = agent_obj.get("name")
        agent_id = agent_obj.get("id")
        agent_img = agent_obj.get("assets", {}).get("small") or agent_obj.get("assets", {}).get("displayIcon")

    blue_r = _team_rounds(blue)
    red_r = _team_rounds(red)
    score = f"{blue_r}-{red_r}" if (blue_r or red_r) else ""

    return {
        "match_id": metadata.get("match_id"),
        "map": metadata.get("map", {}).get("name"),
        "map_id": metadata.get("map", {}).get("id"),
        "queue": metadata.get("queue", {}).get("name", "Custom"),
        "started_at": metadata.get("started_at"),
        "game_length_ms": metadata.get("game_length_in_ms"),
        "season": metadata.get("season", {}).get("short"),
        "won": won,
        "score": score,
        "my_team": team_id,
        "agent": agent_name,
        "agent_id": agent_id,
        "agent_img": agent_img,
        "stats": me.get("stats", {}),
        "tier": me.get("currenttier_patched"),
    }


@router.get("")
async def list_matches(
    request: Request,
    db: Session = Depends(get_session),
    mode: str = Query("competitive"),
    size: int = Query(20, ge=1, le=50),
    name: str = Query(None),
    tag: str = Query(None),
    region: str = Query(None),
    puuid: str = Query(None),
):
    user = await get_current_user(request)
    riot_name, riot_tag, riot_region, linked_puuid = await _resolve_player(user["email"], db, name, tag, region)
    player_puuid = puuid or linked_puuid or ""

    raw = await henrik_client.get_matches(riot_region, riot_name, riot_tag, mode=mode, size=size)
    _logged: list = []
    return [_format_match(m, player_puuid, _logged=_logged) for m in raw]
