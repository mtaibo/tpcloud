from datetime import datetime, timezone

from fastapi import BackgroundTasks, Request, Response
from sqlmodel import Session as DBSession, delete

from models import Session as SessionModel, User
from security import set_session_cookie
from utils import (
    PENDING_TOTP_DURATION,
    cookie_lifetime,
    describe_device,
    detect_client_type,
    device_for_request,
    get_client_ip,
    iso,
    record_event,
    session_duration_for,
    update_session_location,
)


def delete_expired_sessions(db: DBSession) -> int:
    """Deletes every session past its expiry. An expired one is already refused everywhere and
    shown nowhere; its row only kept an old IP, device and place around. What happened stays in
    the account's activity (login_events), which keeps six months."""
    result = db.exec(delete(SessionModel).where(SessionModel.expires_at < datetime.now(timezone.utc)))
    db.commit()
    return result.rowcount or 0


def start_session(
    db: DBSession,
    background_tasks: BackgroundTasks,
    request: Request,
    response: Response,
    user: User,
    method: str,
    remember: bool,
    totp_pending: bool = False,
    approved_by: str | None = None,
) -> SessionModel:
    """Creates a session for whoever made this request and gives them its cookie. A password
    sign-in still waiting for its TOTP code gets ten minutes and only becomes a real session
    once the code is right (`complete_totp`)."""
    ip = get_client_ip(request)
    ua_string = request.headers.get("User-Agent", "")
    client_type = detect_client_type(request)
    now = datetime.now(timezone.utc)
    duration = PENDING_TOTP_DURATION if totp_pending else session_duration_for(client_type, remember)

    db_session = SessionModel(
        user_id=user.id,
        expires_at=now + duration,
        ip_address=ip,
        user_agent=ua_string,
        totp_verified=not totp_pending,
        auth_method=method,
        device_info=device_for_request(request)["label"],
        location=ip,
        client_type=client_type,
        remember=remember,
        last_seen_at=now,
        last_ip=ip,
        approved_by=approved_by,
    )
    db.add(db_session)
    db.commit()
    background_tasks.add_task(update_session_location, db_session.session_id, ip)
    if not totp_pending:
        record_event(request, user.id, "signin", method, approved_by)

    set_session_cookie(
        response,
        db_session.session_id,
        int(duration.total_seconds()) if totp_pending else cookie_lifetime(db_session),
    )
    return db_session


def complete_totp(
    db: DBSession, request: Request, response: Response, db_session: SessionModel
) -> None:
    db_session.totp_verified = True
    db_session.expires_at = datetime.now(timezone.utc) + session_duration_for(db_session.client_type, db_session.remember)
    db.add(db_session)
    db.commit()
    record_event(request, db_session.user_id, "signin", "password", "con código de verificación")
    set_session_cookie(response, db_session.session_id, cookie_lifetime(db_session))


def session_json(s: SessionModel, current_session_id: str | None = None) -> dict:
    """Everything worth knowing about a session, for the person it belongs to (never its token)."""
    device = describe_device(s.user_agent, s.client_type, _platform_of(s))
    return {
        "id": s.id,
        "created_at": iso(s.created_at),
        "expires_at": iso(s.expires_at),
        "last_seen_at": iso(s.last_seen_at or s.created_at),
        "ip_address": s.ip_address,
        "last_ip": s.last_ip or s.ip_address,
        "device_info": s.device_info,
        "device": device,
        "user_agent": s.user_agent,
        "location": s.location,
        "auth_method": s.auth_method,
        "client_type": s.client_type,
        "remember": s.remember or s.client_type == "native",
        "approved_by": s.approved_by,
        "current": current_session_id is not None and s.session_id == current_session_id,
    }


def _platform_of(s: SessionModel) -> str | None:
    if s.client_type != "native":
        return None
    label = s.device_info or ""
    return "macos" if label.endswith("Mac") else "ios" if label.endswith("iPhone") else None
