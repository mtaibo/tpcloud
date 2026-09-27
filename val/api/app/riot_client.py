import base64
import json
import logging
import re
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

_token_cache: dict[str, tuple[dict, float, dict]] = {}  # email -> (tokens, expires_at, cookies)
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
    return {
        "User-Agent": _ua(client_version),
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Accept-Language": "en-US,en;q=0.9",
    }


def _extract_access_token(data: dict) -> str:
    uri = data.get("response", {}).get("parameters", {}).get("uri", "")
    match = re.search(r"access_token=([^&]+)", uri)
    if not match:
        raise HTTPException(status_code=500, detail=f"Failed to extract Riot access token from URI: {uri[:200]}")
    return match.group(1)


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
        "_cookies": dict(session.cookies),
    }


async def _cookie_reauth(session: AsyncSession, client_version: str) -> dict | None:
    """Try to get fresh tokens using stored session cookies. Returns tokens or None."""
    try:
        resp = await session.get(
            "https://auth.riotgames.com/api/v1/authorization",
            params={
                "client_id": "play-valorant-web-prod",
                "nonce": "1",
                "redirect_uri": "https://playvalorant.com/opt_in",
                "response_type": "token id_token",
                "scope": "openid",
            },
            headers=_auth_headers(client_version),
            allow_redirects=False,
        )
        location = resp.headers.get("location", "")
        m = re.search(r"access_token=([^&]+)", location)
        if not m:
            return None
        return await _finish_auth_cffi(session, m.group(1), client_version)
    except Exception:
        return None


async def start_auth(user_email: str, username: str, password: str) -> dict:
    """
    Returns tokens dict on success, or {"requires_mfa": True} if MFA is needed.
    Raises HTTPException on bad credentials.
    """
    client_version = await _get_client_version()
    headers = _auth_headers(client_version)

    async with AsyncSession(impersonate="chrome120", timeout=15) as session:
        # Init session — establishes cookies
        await session.post(
            "https://auth.riotgames.com/api/v1/authorization",
            json={
                "client_id": "play-valorant-web-prod",
                "nonce": "1",
                "redirect_uri": "https://playvalorant.com/opt_in",
                "response_type": "token id_token",
                "scope": "openid",
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

        logger.warning("Riot auth: type=%r error=%r country=%r", data.get("type"), data.get("error"), data.get("country"))

        if data.get("type") == "error" or data.get("error") == "auth_failure":
            raise HTTPException(status_code=401, detail="Invalid Riot credentials — check username and password")

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

    headers = _auth_headers(client_version)

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


async def get_tokens(user_email: str, username: str, password: str) -> dict:
    now = time.monotonic()
    cached = _token_cache.get(user_email)
    if cached and cached[1] > now:
        return cached[0]

    client_version = await _get_client_version()

    # Try cookie reauth if we have cookies from a previous session
    old_cookies = cached[2] if cached else {}
    if old_cookies:
        try:
            async with AsyncSession(impersonate="chrome120", timeout=15, cookies=old_cookies) as session:
                tokens = await _cookie_reauth(session, client_version)
                if tokens:
                    cache_tokens(user_email, tokens)
                    return _token_cache[user_email][0]
        except Exception:
            pass

    # Full auth with username/password
    tokens = await start_auth(user_email, username, password)
    if tokens.get("requires_mfa"):
        raise HTTPException(status_code=428, detail="MFA required to refresh tokens")
    cache_tokens(user_email, tokens)
    return _token_cache[user_email][0]


def cache_tokens(user_email: str, tokens: dict):
    cookies = tokens.get("_cookies", {})
    clean = {k: v for k, v in tokens.items() if k != "_cookies"}
    _token_cache[user_email] = (clean, time.monotonic() + 3300, cookies)


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
