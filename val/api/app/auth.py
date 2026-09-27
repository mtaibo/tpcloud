import os
import time

import httpx
from fastapi import HTTPException, Request

AUTH_SERVICE_URL = os.getenv("AUTH_SERVICE_URL", "http://auth:8000")

_client = httpx.AsyncClient()
_cache: dict[str, tuple[dict, float]] = {}
_CACHE_TTL = 30.0


async def get_current_user(request: Request) -> dict:
    cookie = request.headers.get("cookie", "")
    now = time.monotonic()

    cached = _cache.get(cookie)
    if cached and cached[1] > now:
        return cached[0]

    resp = await _client.get(
        f"{AUTH_SERVICE_URL}/auth/passkey/me",
        headers={"cookie": cookie},
    )
    if resp.status_code != 200:
        _cache.pop(cookie, None)
        raise HTTPException(status_code=401, detail="Not authenticated")

    user = resp.json()
    _cache[cookie] = (user, now + _CACHE_TTL)
    if len(_cache) > 500:
        cutoff = time.monotonic()
        stale = [k for k, v in _cache.items() if v[1] <= cutoff]
        for k in stale:
            del _cache[k]
    return user
