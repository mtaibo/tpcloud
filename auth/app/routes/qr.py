"""Signing in a browser by scanning a QR code with a device that's already signed in.

The browser asks for a request (`/start`) and shows its address as a QR code; only that browser
holds the secret to collect the session (a cookie). Whoever scans it sees which device is asking,
from where, and approves or denies it from their own session; the browser polls until then. A
request lives two minutes and can be approved once.
"""

import hashlib
import hmac
import secrets
import time
from dataclasses import dataclass, field

from fastapi import APIRouter, BackgroundTasks, Depends, HTTPException, Request
from fastapi.responses import JSONResponse
from sqlmodel import Session as DBSession, select

from config import PUBLIC_URL, QR_COOKIE, SESSION_COOKIE
from database import get_session
from dependencies import require_login
from models import Session as SessionModel, User
from security import ExpiringDict, FailureLimiter, clear_cookie, set_short_cookie
from sessions import start_session
from utils import device_for_request, get_client_ip, lookup_location, record_event

router = APIRouter(prefix="/auth/qr")

QR_LIFETIME = 120

# Starting a request needs no account, so it's capped per address.
_starts = FailureLimiter(limit=40, window_seconds=15 * 60)


@dataclass
class QRRequest:
    secret_hash: str
    ip: str
    device: dict
    created: float = field(default_factory=time.time)
    status: str = "pending"  # pending | scanned | approved | denied
    location: str | None = None
    user_id: int | None = None
    approved_by: str | None = None


_requests = ExpiringDict(ttl=QR_LIFETIME + 10, max_size=2000)


def _get(request_id: str) -> QRRequest | None:
    item = _requests.pop(request_id)
    if item is None:
        return None
    if time.time() - item.created > QR_LIFETIME and item.status != "approved":
        return None
    _requests.set(request_id, item)
    return item


def _hash(secret: str) -> str:
    return hashlib.sha256(secret.encode()).hexdigest()


@router.post("/start")
def start(request: Request):
    ip = get_client_ip(request)
    _starts.check(f"ip:{ip}")
    _starts.fail(f"ip:{ip}")

    request_id = secrets.token_urlsafe(12)
    secret = secrets.token_urlsafe(32)
    _requests.set(request_id, QRRequest(secret_hash=_hash(secret), ip=ip, device=device_for_request(request)))

    response = JSONResponse({
        "id": request_id,
        "url": f"{PUBLIC_URL}/accounts/approve/{request_id}",
        "expires_in": QR_LIFETIME,
    })
    set_short_cookie(response, QR_COOKIE, f"{request_id}.{secret}", QR_LIFETIME + 10)
    return response


@router.get("/poll")
def poll(request: Request, background_tasks: BackgroundTasks, session: DBSession = Depends(get_session)):
    """The browser that showed the code asks how it's going; once approved, it gets its session."""
    raw = request.cookies.get(QR_COOKIE, "")
    request_id, _, secret = raw.partition(".")
    item = _get(request_id) if request_id else None
    if not item or not hmac.compare_digest(item.secret_hash, _hash(secret)):
        return {"status": "expired"}

    if item.status != "approved":
        return {"status": item.status}

    _requests.pop(request_id)
    user = session.get(User, item.user_id)
    if not user:
        return {"status": "expired"}
    response = JSONResponse({"status": "approved"})
    start_session(session, background_tasks, request, response, user, "qr", True, approved_by=item.approved_by)
    clear_cookie(response, QR_COOKIE)
    return response


def _approver(request: Request, db: DBSession) -> SessionModel:
    return db.exec(select(SessionModel).where(SessionModel.session_id == request.cookies.get(SESSION_COOKIE))).first()


@router.get("/{request_id}")
def details(request_id: str, user: User = Depends(require_login)):
    """What the person scanning sees before approving: which device asks, from where."""
    item = _get(request_id)
    if not item or item.status in ("approved", "denied"):
        raise HTTPException(status_code=410, detail="Este código ya no es válido. Genera otro en la pantalla de inicio de sesión.")
    if item.location is None:
        item.location = lookup_location(item.ip)
    if item.status == "pending":
        item.status = "scanned"
    return {
        "device": item.device,
        "ip_address": item.ip,
        "location": item.location,
        "created_at": item.created,
        "expires_in": max(0, int(QR_LIFETIME - (time.time() - item.created))),
        "account": {"email": user.email, "display_name": user.display_name},
    }


@router.post("/{request_id}/approve")
def approve(
    request_id: str,
    request: Request,
    user: User = Depends(require_login),
    db: DBSession = Depends(get_session),
):
    item = _get(request_id)
    if not item or item.status in ("approved", "denied"):
        raise HTTPException(status_code=410, detail="Este código ya no es válido. Genera otro en la pantalla de inicio de sesión.")
    approver = _approver(request, db)
    item.status = "approved"
    item.user_id = user.id
    item.approved_by = approver.device_info if approver else device_for_request(request)["label"]
    return {"status": "approved"}


@router.post("/{request_id}/deny")
def deny(
    request_id: str,
    request: Request,
    user: User = Depends(require_login),
):
    item = _get(request_id)
    if item and item.status != "approved":
        item.status = "denied"
        record_event(
            request, user.id, "failed", "qr",
            f"Rechazaste un inicio de sesión desde {item.device['label']} ({item.location or item.ip})",
        )
    return {"status": "denied"}
