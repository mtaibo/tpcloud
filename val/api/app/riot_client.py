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


def get_cached_tokens(user_email: str) -> dict | None:
    """Return in-memory cached tokens if still valid, else None."""
    cached = _token_cache.get(user_email)
    if cached and cached[1] > time.monotonic():
        return cached[0]
    return None


async def auth_with_access_token(user_email: str, access_token: str) -> dict:
    """Complete auth using a browser-obtained access_token. Calls entitlements + userinfo."""
    client_version = await _get_client_version()
    ua = _ua(client_version)
    async with AsyncSession(impersonate="chrome120", timeout=15) as session:
        ent_resp = await session.post(
            "https://entitlements.auth.riotgames.com/api/token/v1",
            json={},
            headers={"Authorization": f"Bearer {access_token}", "User-Agent": ua, "Content-Type": "application/json"},
        )
        if ent_resp.status_code != 200:
            logger.warning("Entitlements call failed: %s %s", ent_resp.status_code, ent_resp.text[:200])
            raise HTTPException(status_code=401, detail="Invalid access token — get a fresh one from the Riot login flow")
        entitlements_token = ent_resp.json().get("entitlements_token", "")

        user_resp = await session.get(
            "https://auth.riotgames.com/userinfo",
            headers={"Authorization": f"Bearer {access_token}", "User-Agent": ua},
        )
        puuid = user_resp.json().get("sub", "")

    logger.info("Token auth SUCCESS for %s, puuid=%s", user_email, puuid)
    return {
        "access_token": access_token,
        "entitlements_token": entitlements_token,
        "puuid": puuid,
        "client_version": client_version,
        "_cookies": {},
    }


async def auth_with_ssid(user_email: str, ssid: str) -> dict:
    """Authenticate using browser ssid cookie. Bypasses password auth and captcha entirely."""
    client_version = await _get_client_version()
    async with AsyncSession(impersonate="chrome120", timeout=15, cookies={"ssid": ssid}) as session:
        tokens = await _cookie_reauth(session, client_version)
        if not tokens:
            raise HTTPException(
                status_code=401,
                detail="Invalid or expired ssid cookie — log in again in your browser and copy a fresh ssid",
            )
        logger.info("Cookie auth SUCCESS for %s, puuid=%s", user_email, tokens.get("puuid"))
        return tokens


async def get_tokens(user_email: str, *, ssid: str | None = None) -> dict:
    now = time.monotonic()
    cached = _token_cache.get(user_email)
    if cached and cached[1] > now:
        return cached[0]

    client_version = await _get_client_version()

    old_cookies = cached[2] if cached else {}
    if old_cookies:
        async with AsyncSession(impersonate="chrome120", timeout=15, cookies=old_cookies) as session:
            tokens = await _cookie_reauth(session, client_version)
            if tokens:
                cache_tokens(user_email, tokens)
                return _token_cache[user_email][0]

    if ssid:
        async with AsyncSession(impersonate="chrome120", timeout=15, cookies={"ssid": ssid}) as session:
            tokens = await _cookie_reauth(session, client_version)
            if tokens:
                logger.info("ssid reauth SUCCESS for %s", user_email)
                cache_tokens(user_email, tokens)
                return _token_cache[user_email][0]

    raise HTTPException(status_code=401, detail="Session expired — log in again")


def cache_tokens(user_email: str, tokens: dict):
    cookies = tokens.get("_cookies", {})
    clean = {k: v for k, v in tokens.items() if k != "_cookies"}
    _token_cache[user_email] = (clean, time.monotonic() + 3300, cookies)


def invalidate_tokens(user_email: str):
    _token_cache.pop(user_email, None)


def _riot_headers(tokens: dict) -> dict:
    return {
        "Authorization": f"Bearer {tokens['access_token']}",
        "X-Riot-Entitlements-JWT": tokens["entitlements_token"],
        "X-Riot-ClientVersion": tokens["client_version"],
        "X-Riot-ClientPlatform": CLIENT_PLATFORM,
        "Content-Type": "application/json",
    }


async def get_store(tokens: dict, puuid: str, region: str = REGION) -> dict:
    async with AsyncSession(impersonate="chrome120", timeout=15) as session:
        r = await session.get(
            f"https://pd.{region}.a.pvp.net/store/v3/storefront/{puuid}",
            headers=_riot_headers(tokens),
        )
        if r.status_code == 403:
            raise HTTPException(status_code=403, detail="Riot token rejected — re-enter credentials")
        if r.status_code >= 400:
            logger.error("Riot storefront %s: %s", r.status_code, r.text[:500])
            raise HTTPException(status_code=502, detail=f"Riot storefront error {r.status_code}")
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
