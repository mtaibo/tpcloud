import threading
from datetime import datetime, timedelta, timezone

import httpx
import user_agents as ua_lib

from config import GEOIP

# A browser that ticked "keep me signed in" stays signed in for a month after it was last used;
# one that didn't, for half a day (and its cookie goes when the browser closes). The app, which
# keeps its own credentials in the Keychain, for three months.
WEB_REMEMBER_DURATION = timedelta(days=30)
WEB_SESSION_DURATION = timedelta(hours=12)
NATIVE_SESSION_DURATION = timedelta(days=90)
# A password sign-in waiting for its TOTP code.
PENDING_TOTP_DURATION = timedelta(minutes=10)

# How often a session's "last seen" is written, so every request through the gateway isn't a write.
LAST_SEEN_INTERVAL = timedelta(minutes=1)


LANGUAGES = ("en", "es")


def say(user, en: str, es: str) -> str:
    """A message in the account's language. Spanish until one is picked, as the web and the
    app always were; the sign-in endpoints answer in English only."""
    return en if user is not None and getattr(user, "language", None) == "en" else es


def detect_client_type(request) -> str:
    """The native app (iOS/macOS) tags its requests with X-TP-Client; browsers never send it."""
    return "native" if request.headers.get("X-TP-Client") else "web"


def session_duration_for(client_type: str, remember: bool = False) -> timedelta:
    if client_type == "native":
        return NATIVE_SESSION_DURATION
    return WEB_REMEMBER_DURATION if remember else WEB_SESSION_DURATION


def cookie_lifetime(db_session) -> int | None:
    """Max-Age for a session's cookie: None (gone when the browser closes) unless it's remembered."""
    if db_session.client_type == "web" and not db_session.remember:
        return None
    return int(session_duration_for(db_session.client_type, db_session.remember).total_seconds())


def as_utc(value: datetime | None) -> datetime | None:
    return value.replace(tzinfo=timezone.utc) if value and value.tzinfo is None else value


def iso(value: datetime | None) -> str | None:
    return as_utc(value).isoformat() if value else None


_NATIVE_PLATFORMS = {"ios": ("iPhone", "phone"), "macos": ("Mac", "desktop")}


def describe_device(ua_string: str | None, client_type: str = "web", platform: str | None = None) -> dict:
    """What a user agent is, in words a person recognises: browser, system, and the kind of
    device (phone, tablet, desktop) for its icon."""
    if client_type == "native":
        name, kind = _NATIVE_PLATFORMS.get((platform or "").lower(), (None, "phone"))
        if not name:
            ua = ua_lib.parse(ua_string or "")
            name = "Mac" if "Mac" in ua.os.family else "iPhone"
            kind = "desktop" if name == "Mac" else "phone"
        return {"browser": "App TPCloud", "os": name, "kind": kind, "label": f"App TPCloud en {name}"}

    if not ua_string:
        return {"browser": None, "os": None, "kind": "unknown", "label": "Dispositivo desconocido"}
    ua = ua_lib.parse(ua_string)
    browser = ua.browser.family if ua.browser.family != "Other" else None
    os_ = ua.os.family if ua.os.family != "Other" else None
    if os_ == "Mac OS X":
        os_ = "macOS"
    if browser and browser.startswith("Mobile Safari"):
        browser = "Safari"
    if ua.is_tablet:
        kind = "tablet"
    elif ua.is_mobile:
        kind = "phone"
    elif ua.is_pc:
        kind = "desktop"
    else:
        kind = "unknown"
    if browser and os_:
        label = f"{browser} en {os_}"
    else:
        label = browser or os_ or ua_string[:80]
    return {"browser": browser, "os": os_, "kind": kind, "label": label}


def device_for_request(request) -> dict:
    client_type = detect_client_type(request)
    return describe_device(
        request.headers.get("User-Agent", ""), client_type, request.headers.get("X-TP-Client")
    )


def lookup_location(ip: str) -> str:
    if not GEOIP:
        return ip
    if not ip or ip in ("127.0.0.1", "::1", ""):
        return "Local"
    try:
        r = httpx.get(
            f"http://ip-api.com/json/{ip}?fields=city,regionName,country",
            timeout=2.0,
        )
        if r.status_code == 200:
            data = r.json()
            parts = []
            for key in ("city", "regionName", "country"):
                value = data.get(key, "")
                if value and value not in parts:
                    parts.append(value)
            return ", ".join(parts) if parts else ip
    except Exception:
        pass
    return ip


def get_client_ip(request) -> str:
    """The gateway sets X-Real-IP to the client's address, taken from Cloudflare's or the proxy's
    header only when the hop it came through is a trusted one, so a client can't pick its own."""
    real_ip = request.headers.get("X-Real-IP", "")
    if real_ip:
        return real_ip.strip()
    return request.client.host if request.client else ""


def update_session_location(session_id: str, ip: str) -> None:
    """Best-effort location backfill, run as a background task so a slow/unresponsive
    geolocation lookup never blocks the login response itself."""
    from database import engine
    from models import Session as SessionModel
    from sqlmodel import Session as DBSession, select

    location = lookup_location(ip)
    with DBSession(engine) as db:
        db_session = db.exec(
            select(SessionModel).where(SessionModel.session_id == session_id)
        ).first()
        if db_session:
            db_session.location = location
            db.add(db_session)
            db.commit()


# Activity older than this is dropped.
EVENT_RETENTION = timedelta(days=180)


def record_event(
    request,
    user_id: int,
    kind: str,
    method: str | None = None,
    detail: str | None = None,
) -> None:
    """Adds a line to the account's activity (a sign-in, a failed attempt, a revoked session).
    On its own thread: its location lookup may take a moment and must never hold up the answer,
    and a failed attempt is answered with an error, which skips FastAPI's background tasks."""
    ip = get_client_ip(request)
    args = (
        user_id, kind, method, detail, ip,
        request.headers.get("User-Agent", ""), detect_client_type(request), device_for_request(request)["label"],
    )
    threading.Thread(target=_store_event, args=args, daemon=True).start()


def _store_event(user_id, kind, method, detail, ip, ua_string, client_type, device) -> None:
    from database import engine
    from models import LoginEvent
    from sqlmodel import Session as DBSession, delete

    location = lookup_location(ip) if ip else None
    with DBSession(engine) as db:
        db.add(LoginEvent(
            user_id=user_id,
            kind=kind,
            method=method,
            detail=detail,
            ip_address=ip,
            location=location,
            device_info=device,
            user_agent=ua_string,
            client_type=client_type,
        ))
        db.exec(delete(LoginEvent).where(LoginEvent.at < datetime.now(timezone.utc) - EVENT_RETENTION))
        db.commit()
