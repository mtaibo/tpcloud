import os

import httpx
from fastapi import HTTPException

HENRIK_API_KEY = os.getenv("HENRIK_API_KEY", "")
BASE_URL = "https://api.henrikdev.tech"

_client: httpx.AsyncClient | None = None


async def startup():
    global _client
    headers = {"Authorization": HENRIK_API_KEY} if HENRIK_API_KEY else {}
    _client = httpx.AsyncClient(base_url=BASE_URL, headers=headers, timeout=15.0)


async def shutdown():
    global _client
    if _client:
        await _client.aclose()
        _client = None


def _check():
    if not _client:
        raise HTTPException(status_code=503, detail="API client not initialized")
    return _client


async def get_account(name: str, tag: str) -> dict:
    c = _check()
    r = await c.get(f"/valorant/v1/account/{name}/{tag}")
    if r.status_code == 404:
        raise HTTPException(status_code=404, detail="Account not found")
    if r.status_code == 429:
        raise HTTPException(status_code=429, detail="Rate limited")
    r.raise_for_status()
    return r.json().get("data", {})


async def get_mmr(region: str, name: str, tag: str) -> dict:
    c = _check()
    r = await c.get(f"/valorant/v3/mmr/{region}/pc/{name}/{tag}")
    if r.status_code == 404:
        raise HTTPException(status_code=404, detail="MMR data not found")
    if r.status_code == 429:
        raise HTTPException(status_code=429, detail="Rate limited")
    r.raise_for_status()
    return r.json().get("data", {})


async def get_mmr_history(region: str, name: str, tag: str) -> list:
    c = _check()
    r = await c.get(f"/valorant/v1/mmr-history/pc/{region}/{name}/{tag}")
    if r.status_code == 404:
        return []
    if r.status_code == 429:
        raise HTTPException(status_code=429, detail="Rate limited")
    r.raise_for_status()
    return r.json().get("data", [])


async def get_matches(region: str, name: str, tag: str, mode: str = "competitive", size: int = 20) -> list:
    c = _check()
    params = {"mode": mode, "size": size}
    r = await c.get(f"/valorant/v4/matches/{region}/pc/{name}/{tag}", params=params)
    if r.status_code == 404:
        return []
    if r.status_code == 429:
        raise HTTPException(status_code=429, detail="Rate limited")
    r.raise_for_status()
    return r.json().get("data", [])
