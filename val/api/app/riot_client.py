import base64
import json
import re
import time

import httpx
from fastapi import HTTPException

REGION = "eu"
WEAPON_SKIN_ITEM_TYPE = "e7c63390-eda7-46e0-bb9a-14f0a3a9f911"

CLIENT_PLATFORM = base64.b64encode(json.dumps({
    "platformType": "PC",
    "platformOS": "Windows",
    "platformOSVersion": "10.0.19042.1.256.64bit",
    "platformChipset": "Unknown",
}).encode()).decode()

_token_cache: dict[str, tuple[dict, float]] = {}
_version_cache: tuple[str, float] = ("", 0.0)


async def _get_client_version() -> str:
    global _version_cache
    now = time.monotonic()
    if _version_cache[0] and _version_cache[1] > now:
        return _version_cache[0]
    try:
        async with httpx.AsyncClient(timeout=5.0) as c:
            r = await c.get("https://valorant-version.com/v1/version")
            if r.status_code == 200:
                ver = r.json().get("riotClientVersion", "")
                if ver:
                    _version_cache = (ver, now + 3600)
                    return ver
    except Exception:
        pass
    fallback = "release-09.10-shipping-31-2637300"
    _version_cache = (fallback, now + 3600)
    return fallback


async def authenticate(username: str, password: str) -> dict:
    client_version = await _get_client_version()
    ua = f"RiotClient/{client_version} rso-auth (Windows;10;;Professional, x64)"

    async with httpx.AsyncClient(timeout=15.0) as client:
        # Initialize auth session
        await client.post(
            "https://auth.riotgames.com/api/v1/authorization",
            json={
                "client_id": "riot-client",
                "nonce": "1",
                "redirect_uri": "http://localhost/redirect",
                "response_type": "token id_token",
                "scope": "account openid",
            },
            headers={"User-Agent": ua, "Content-Type": "application/json"},
        )

        # Send credentials
        resp = await client.put(
            "https://auth.riotgames.com/api/v1/authorization",
            json={
                "type": "auth",
                "username": username,
                "password": password,
                "remember": False,
                "language": "en_US",
            },
            headers={"User-Agent": ua, "Content-Type": "application/json"},
        )

        data = resp.json()
        if data.get("type") == "error":
            raise HTTPException(status_code=401, detail="Invalid Riot credentials")
        if data.get("type") == "multifactor":
            raise HTTPException(status_code=400, detail="MFA is not supported")

        uri = data.get("response", {}).get("parameters", {}).get("uri", "")
        match = re.search(r"access_token=([^&]+)", uri)
        if not match:
            raise HTTPException(status_code=502, detail="Failed to extract Riot access token")
        access_token = match.group(1)

        # Entitlements token
        ent_resp = await client.post(
            "https://entitlements.auth.riotgames.com/api/token/v1",
            json={},
            headers={
                "Authorization": f"Bearer {access_token}",
                "User-Agent": ua,
                "Content-Type": "application/json",
            },
        )
        entitlements_token = ent_resp.json().get("entitlements_token", "")

        # PUUID
        user_resp = await client.get(
            "https://auth.riotgames.com/userinfo",
            headers={"Authorization": f"Bearer {access_token}", "User-Agent": ua},
        )
        puuid = user_resp.json().get("sub", "")

        return {
            "access_token": access_token,
            "entitlements_token": entitlements_token,
            "puuid": puuid,
            "client_version": client_version,
        }


async def get_tokens(user_email: str, username: str, password: str) -> dict:
    now = time.monotonic()
    cached = _token_cache.get(user_email)
    if cached and cached[1] > now:
        return cached[0]

    tokens = await authenticate(username, password)
    _token_cache[user_email] = (tokens, now + 3300)
    return tokens


def _riot_headers(tokens: dict) -> dict:
    return {
        "Authorization": f"Bearer {tokens['access_token']}",
        "X-Riot-Entitlements-JWT": tokens["entitlements_token"],
        "X-Riot-ClientVersion": tokens["client_version"],
        "X-Riot-ClientPlatform": CLIENT_PLATFORM,
        "Content-Type": "application/json",
    }


async def get_store(tokens: dict, puuid: str, region: str = REGION) -> dict:
    async with httpx.AsyncClient(timeout=15.0) as client:
        r = await client.get(
            f"https://pd.{region}.a.pvp.net/store/v3/storefront/{puuid}",
            headers=_riot_headers(tokens),
        )
        if r.status_code == 403:
            raise HTTPException(status_code=403, detail="Riot token rejected — re-enter credentials")
        r.raise_for_status()
        return r.json()


async def get_inventory(tokens: dict, puuid: str, region: str = REGION) -> list:
    async with httpx.AsyncClient(timeout=15.0) as client:
        r = await client.get(
            f"https://pd.{region}.a.pvp.net/store/v1/entitlements/{puuid}/{WEAPON_SKIN_ITEM_TYPE}",
            headers=_riot_headers(tokens),
        )
        if r.status_code == 403:
            raise HTTPException(status_code=403, detail="Riot token rejected — re-enter credentials")
        r.raise_for_status()
        return r.json().get("Entitlements", [])


async def get_skin_info(skin_uuid: str) -> dict:
    try:
        async with httpx.AsyncClient(timeout=8.0) as c:
            r = await c.get(f"https://valorant-api.com/v1/weapons/skinlevels/{skin_uuid}")
            if r.status_code == 200:
                return r.json().get("data", {})
    except Exception:
        pass
    return {}


def invalidate_tokens(user_email: str):
    _token_cache.pop(user_email, None)
