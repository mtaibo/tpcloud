import base64
import json
import logging
import re
import secrets
import time

import httpx
from curl_cffi.requests import AsyncSession
from fastapi import HTTPException

logger = logging.getLogger(__name__)

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
_pending_mfa: dict[str, tuple[dict, str, float]] = {}  # user_email -> (cookies, client_version, expires)


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


def _ua(client_version: str) -> str:
    return f"RiotClient/{client_version} rso-auth (Windows;10;;Professional, x64)"


def _auth_headers(client_version: str) -> dict:
    return {"User-Agent": _ua(client_version), "Content-Type": "application/json"}


def _extract_access_token(data: dict) -> str:
    uri = data.get("response", {}).get("parameters", {}).get("uri", "")
    match = re.search(r"access_token=([^&]+)", uri)
    if not match:
        raise HTTPException(status_code=500, detail=f"Failed to extract Riot access token from URI: {uri[:200]}")
    return match.group(1)


async def _finish_auth(client: httpx.AsyncClient, access_token: str, client_version: str) -> dict:
    ua = _ua(client_version)
    ent_resp = await client.post(
        "https://entitlements.auth.riotgames.com/api/token/v1",
        json={},
        headers={"Authorization": f"Bearer {access_token}", "User-Agent": ua, "Content-Type": "application/json"},
    )
    entitlements_token = ent_resp.json().get("entitlements_token", "")

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


async def _finish_auth_cffi(session: AsyncSession, access_token: str, client_version: str) -> dict:
    ua = _ua(client_version)
    ent_resp = await session.post(
        "https://entitlements.auth.riotgames.com/api/token/v1",
        json={},
        headers={"Authorization": f"Bearer {access_token}", "User-Agent": ua, "Content-Type": "application/json"},
    )
    entitlements_token = ent_resp.json().get("entitlements_token", "")

    user_resp = await session.get(
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


async def start_auth(user_email: str, username: str, password: str) -> dict:
    """
    Returns tokens dict on success, or {"requires_mfa": True} if MFA is needed.
    Raises HTTPException on bad credentials.
    """
    client_version = await _get_client_version()
    headers = {**_auth_headers(client_version), "Accept": "application/json", "Accept-Language": "en-US,en;q=0.9"}

    async with AsyncSession(impersonate="chrome120", timeout=15) as session:
        await session.post(
            "https://auth.riotgames.com/api/v1/authorization",
            json={
                "client_id": "riot-client",
                "nonce": secrets.token_hex(16),
                "redirect_uri": "http://localhost/redirect",
                "response_type": "token id_token",
                "scope": "openid link ban lol_region account",
            },
            headers=headers,
        )

        resp = await session.put(
            "https://auth.riotgames.com/api/v1/authorization",
            json={
                "type": "auth",
                "username": username,
                "password": password,
                "remember": False,
                "language": "en_US",
            },
            headers=headers,
        )
        data = resp.json()

        logger.warning("Riot auth response type=%r keys=%s", data.get("type"), list(data.keys()))

        if data.get("type") == "error" or data.get("error") == "auth_failure":
            raise HTTPException(status_code=401, detail="Invalid Riot credentials")

        if data.get("type") == "multifactor" or "multifactor" in data:
            cookies = {k: v for k, v in session.cookies.items()}
            _pending_mfa[user_email] = (cookies, client_version, time.monotonic() + 300)
            return {"requires_mfa": True}

        if data.get("type") != "response":
            raise HTTPException(
                status_code=500,
                detail=f"Unexpected Riot auth response: type={data.get('type')!r} — {str(data)[:300]}",
            )

        access_token = _extract_access_token(data)
        return await _finish_auth_cffi(session, access_token, client_version)


async def complete_mfa(user_email: str, code: str) -> dict:
    """Complete MFA step using stored cookies from start_auth."""
    pending = _pending_mfa.get(user_email)
    if not pending:
        raise HTTPException(status_code=400, detail="No pending MFA session — re-enter credentials")

    cookies, client_version, expires = pending
    if time.monotonic() > expires:
        _pending_mfa.pop(user_email, None)
        raise HTTPException(status_code=400, detail="MFA session expired — re-enter credentials")

    headers = {**_auth_headers(client_version), "Accept": "application/json", "Accept-Language": "en-US,en;q=0.9"}

    async with AsyncSession(impersonate="chrome120", timeout=15, cookies=cookies) as session:
        resp = await session.put(
            "https://auth.riotgames.com/api/v1/authorization",
            json={"type": "multifactor", "code": code.strip(), "rememberDevice": False},
            headers=headers,
        )
        data = resp.json()

        if data.get("type") == "error":
            raise HTTPException(status_code=401, detail="Invalid MFA code")

        _pending_mfa.pop(user_email, None)
        access_token = _extract_access_token(data)
        return await _finish_auth_cffi(session, access_token, client_version)


async def auth_with_ssid(user_email: str, ssid: str) -> dict:
    """Re-authenticate using a Riot SSID session cookie. Works from datacenter IPs."""
    client_version = await _get_client_version()
    headers = {**_auth_headers(client_version), "Accept": "application/json", "Accept-Language": "en-US,en;q=0.9"}

    async with AsyncSession(impersonate="chrome120", timeout=15) as session:
        session.cookies.set("ssid", ssid, domain=".auth.riotgames.com")

        resp = await session.get(
            "https://auth.riotgames.com/authorize",
            params={
                "redirect_uri": "http://localhost/redirect",
                "client_id": "riot-client",
                "response_type": "token id_token",
                "nonce": secrets.token_hex(16),
                "scope": "openid link ban lol_region account",
            },
            headers=headers,
            allow_redirects=False,
        )

        location = resp.headers.get("location", "")
        logger.warning("SSID re-auth location prefix=%r", location[:80])
        if not location or "access_token" not in location:
            raise HTTPException(status_code=401, detail="Invalid or expired SSID cookie")

        match = re.search(r"access_token=([^&]+)", location)
        if not match:
            raise HTTPException(status_code=401, detail="Could not extract token from re-auth response")

        tokens = await _finish_auth_cffi(session, match.group(1), client_version)
        _token_cache[user_email] = (tokens, time.monotonic() + 3300)
        return tokens


async def get_tokens(user_email: str, username: str, password: str) -> dict:
    now = time.monotonic()
    cached = _token_cache.get(user_email)
    if cached and cached[1] > now:
        return cached[0]

    tokens = await start_auth(user_email, username, password)
    if tokens.get("requires_mfa"):
        raise HTTPException(status_code=428, detail="MFA required to refresh tokens")

    _token_cache[user_email] = (tokens, now + 3300)
    return tokens


def cache_tokens(user_email: str, tokens: dict):
    _token_cache[user_email] = (tokens, time.monotonic() + 3300)


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
    _pending_mfa.pop(user_email, None)
