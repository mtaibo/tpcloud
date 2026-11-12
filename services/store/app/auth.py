import asyncio
import os
import time

import httpx
from fastapi import HTTPException, Request

AUTH_SERVICE_URL = os.getenv("AUTH_SERVICE_URL", "http://auth:8000")

_client = httpx.AsyncClient()
_cache: dict[str, tuple[dict, float]] = {}
_CACHE_TTL = 30.0
_in_flight: dict[str, asyncio.Future] = {}


async def get_current_user(request: Request) -> dict:
    cookie = request.headers.get("cookie", "")
    now = time.monotonic()

    cached = _cache.get(cookie)
    if cached and cached[1] > now:
        return cached[0]

    # A gallery asks for a couple of hundred thumbnails at once: they share one lookup instead of
    # each sending its own to auth while the cache is empty.
    pending = _in_flight.get(cookie)
    if pending is None:
        pending = asyncio.ensure_future(_lookup(cookie))
        _in_flight[cookie] = pending
        pending.add_done_callback(lambda _: _in_flight.pop(cookie, None))
    user = await asyncio.shield(pending)
    if user is None:
        raise HTTPException(status_code=401, detail="Not authenticated")
    return user


async def _lookup(cookie: str) -> dict | None:
    resp = await _client.get(
        f"{AUTH_SERVICE_URL}/auth/passkey/me",
        headers={"cookie": cookie},
    )
    if resp.status_code != 200:
        _cache.pop(cookie, None)
        return None

    now = time.monotonic()
    user = resp.json()
    _cache[cookie] = (user, now + _CACHE_TTL)
    if len(_cache) > 500:
        cutoff = time.monotonic()
        stale = [k for k, v in _cache.items() if v[1] <= cutoff]
        for k in stale:
            del _cache[k]
    return user
