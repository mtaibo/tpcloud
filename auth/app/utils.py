import httpx
import user_agents as ua_lib


def parse_device_info(ua_string: str) -> str:
    if not ua_string:
        return "Dispositivo desconocido"
    ua = ua_lib.parse(ua_string)
    browser = ua.browser.family
    os_ = ua.os.family
    if browser and os_ and browser != "Other" and os_ != "Other":
        return f"{browser} en {os_}"
    if browser and browser != "Other":
        return browser
    return ua_string[:80]


def lookup_location(ip: str) -> str:
    if not ip or ip in ("127.0.0.1", "::1", ""):
        return "Local"
    try:
        r = httpx.get(
            f"http://ip-api.com/json/{ip}?fields=city,country",
            timeout=2.0,
        )
        if r.status_code == 200:
            data = r.json()
            city = data.get("city", "")
            country = data.get("country", "")
            parts = [p for p in [city, country] if p]
            return ", ".join(parts) if parts else ip
    except Exception:
        pass
    return ip


def get_client_ip(request) -> str:
    forwarded = request.headers.get("X-Forwarded-For", "")
    if forwarded:
        return forwarded.split(",")[0].strip()
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
