import time

import httpx

BASE_URL = "https://esports-api.lolesports.com/persisted/gw"
API_KEY = "0TvQnueqKa5mxJntVWt0w4LpLfEkrV1Ta8rQBb9Z"
_HEADERS = {"x-api-key": API_KEY}
_CACHE_TTL = 43200.0  # 12 hours

_cache: dict[str, tuple[any, float]] = {}


async def _fetch(path: str, params: dict | None = None) -> dict:
    now = time.monotonic()
    key = path + str(params)
    cached = _cache.get(key)
    if cached and cached[1] > now:
        return cached[0]

    try:
        async with httpx.AsyncClient(base_url=BASE_URL, headers=_HEADERS, timeout=10.0) as c:
            r = await c.get(path, params=params or {})
            r.raise_for_status()
            data = r.json()
            _cache[key] = (data, now + _CACHE_TTL)
            return data
    except Exception:
        return {}


async def get_vct_leagues() -> list:
    data = await _fetch("/getLeagues", {"hl": "en-US"})
    leagues = data.get("data", {}).get("leagues", [])
    return [l for l in leagues if l.get("sport") == "val" or "valorant" in l.get("name", "").lower()]


async def get_schedule(league_id: str) -> dict:
    return await _fetch("/getSchedule", {"hl": "en-US", "leagueId": league_id})


async def get_standings(tournament_id: str) -> dict:
    return await _fetch("/getStandings", {"hl": "en-US", "tournamentId": tournament_id})


async def get_live() -> dict:
    return await _fetch("/getLive", {"hl": "en-US"})


async def get_vct_overview() -> dict:
    leagues = await get_vct_leagues()
    if not leagues:
        return {"leagues": [], "schedule": [], "live": []}

    live_data = await get_live()
    live_events = [
        e for e in live_data.get("data", {}).get("schedule", {}).get("events", [])
        if e.get("league", {}).get("sport") == "val"
    ]

    schedule_data: list = []
    for league in leagues[:4]:
        sched = await get_schedule(league["id"])
        events = sched.get("data", {}).get("schedule", {}).get("events", [])
        for event in events:
            event["leagueName"] = league.get("name", "")
            event["leagueSlug"] = league.get("slug", "")
        schedule_data.extend(events)

    return {
        "leagues": leagues,
        "schedule": schedule_data,
        "live": live_events,
    }
