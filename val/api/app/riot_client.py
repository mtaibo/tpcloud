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
IMPERSONATIONS = ["chrome120", "chrome116", "chrome110", "safari17_2_ios"]

CLIENT_PLATFORM = base64.b64encode(json.dumps({
    "platformType": "PC",
    "platformOS": "Windows",
    "platformOSVersion": "10.0.19042.1.256.64bit",
    "platformChipset": "Unknown",
}).encode()).decode()

_token_cache: dict[str, tuple[dict, float, dict]] = {}  # email -> (tokens, expires_at, cookies)
_version_cache: tuple[str, float] = ("", 0.0)
_pending_mfa: dict[str, tuple[dict, str, float]] = {}  # user_email -> (cookies, client_version, expires)
_auth_failures: dict[str, float] = {}  # email -> monotonic timestamp of last failure


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


async def _attempt_auth(username: str, password: str, client_version: str, impersonate: str) -> dict:
    """
    Single full auth attempt with one impersonation target.
    Returns dict with:
      - "_done": True, "tokens": {...}  on success
      - raw Riot response fields otherwise (type, error, _session_cookies, etc.)
    """
    headers = _auth_headers(client_version)
    async with AsyncSession(impersonate=impersonate, timeout=15) as session:
        init_resp = await session.post(
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
        logger.info("[%s] Init: status=%s cookies=%s body=%s",
                    impersonate, init_resp.status_code,
                    list(session.cookies.keys()), init_resp.text[:300])

        resp = await session.put(
            "https://auth.riotgames.com/api/v1/authorization",
            json={
                "type": "auth",
                "username": username,
                "password": password,
                "remember": True,
                "language": "en_US",
            },
            headers=headers,
        )
        logger.warning("[%s] Auth: status=%s resp_headers=%s body=%s",
                       impersonate, resp.status_code,
                       dict(resp.headers), resp.text[:600])

        data = resp.json()

        # Capture session cookies before context closes
        data["_session_cookies"] = dict(session.cookies)

        if data.get("type") == "response":
            access_token = _extract_access_token(data)
            tokens = await _finish_auth_cffi(session, access_token, client_version)
            return {"_done": True, "tokens": tokens}

        return data


async def start_auth(user_email: str, username: str, password: str) -> dict:
    """
    Returns tokens dict on success, or {"requires_mfa": True} if MFA is needed.
    Tries all impersonation strategies before raising.
    """
    client_version = await _get_client_version()
    logger.info("start_auth user=%s client_version=%s", user_email, client_version)

    for impersonate in IMPERSONATIONS:
        try:
            data = await _attempt_auth(username, password, client_version, impersonate)
        except Exception as e:
            logger.exception("[%s] Exception: %s", impersonate, e)
            continue

        if data.get("_done"):
            logger.info("[%s] Auth SUCCESS puuid=%s", impersonate, data["tokens"].get("puuid"))
            return data["tokens"]

        rtype = data.get("type")

        if rtype == "multifactor" or "multifactor" in data:
            cookies = data.get("_session_cookies", {})
            _pending_mfa[user_email] = (cookies, client_version, time.monotonic() + 300)
            return {"requires_mfa": True}

        if rtype == "captcha":
            logger.warning("[%s] CAPTCHA — Riot blocking automation, trying next", impersonate)
            continue

        if data.get("error") == "rate_limited":
            logger.warning("[%s] rate_limited, trying next", impersonate)
            continue

        if rtype == "error" or data.get("error") == "auth_failure":
            logger.warning("[%s] auth_failure, trying next", impersonate)
            continue

        logger.warning("[%s] Unexpected type=%r, trying next", impersonate, rtype)

    logger.error("All auth strategies failed for %s", user_email)
    _auth_failures[user_email] = time.monotonic()
    raise HTTPException(
        status_code=401,
        detail="All auth strategies failed — Riot may require captcha or credentials are wrong",
    )


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
        logger.warning("MFA response: %s", data)

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

    # Backoff: don't hammer Riot if we failed recently
    last_fail = _auth_failures.get(user_email, 0)
    if now - last_fail < 600:
        raise HTTPException(status_code=401, detail="Riot auth failed recently — re-enter credentials to retry")

    # Full auth with username/password
    tokens = await start_auth(user_email, username, password)
    if tokens.get("requires_mfa"):
        raise HTTPException(status_code=428, detail="MFA required to refresh tokens")
    cache_tokens(user_email, tokens)
    return _token_cache[user_email][0]


async def run_auth_diagnosis(username: str, password: str) -> list[dict]:
    """Try each impersonation and return diagnostic info. Does NOT modify any cache or state."""
    client_version = await _get_client_version()
    results = []
    for impersonate in IMPERSONATIONS:
        t0 = time.monotonic()
        try:
            async with AsyncSession(impersonate=impersonate, timeout=15) as session:
                init_resp = await session.post(
                    "https://auth.riotgames.com/api/v1/authorization",
                    json={
                        "client_id": "play-valorant-web-prod",
                        "nonce": "1",
                        "redirect_uri": "https://playvalorant.com/opt_in",
                        "response_type": "token id_token",
                        "scope": "openid",
                    },
                    headers=_auth_headers(client_version),
                )
                init_cookies = list(session.cookies.keys())

                resp = await session.put(
                    "https://auth.riotgames.com/api/v1/authorization",
                    json={
                        "type": "auth",
                        "username": username,
                        "password": password,
                        "remember": True,
                        "language": "en_US",
                    },
                    headers=_auth_headers(client_version),
                )
                data = resp.json()
                results.append({
                    "strategy": impersonate,
                    "ms": round((time.monotonic() - t0) * 1000),
                    "init_status": init_resp.status_code,
                    "init_cookies": init_cookies,
                    "auth_status": resp.status_code,
                    "type": data.get("type"),
                    "error": data.get("error"),
                    "country": data.get("country"),
                    "success": data.get("type") == "response",
                    "mfa": data.get("type") == "multifactor",
                    "captcha": data.get("type") == "captcha",
                    "raw": str(data)[:400],
                })
        except Exception as e:
            results.append({
                "strategy": impersonate,
                "ms": round((time.monotonic() - t0) * 1000),
                "error": str(e),
                "success": False,
            })
        logger.info("[diagnose/%s] %s", impersonate, results[-1])
    return results


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
    _auth_failures.pop(user_email, None)
