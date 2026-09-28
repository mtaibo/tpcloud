import json
import logging
from datetime import datetime, timezone, timedelta

from apscheduler.schedulers.asyncio import AsyncIOScheduler
from sqlmodel import Session, select

from app import crypto, riot_client
from app.database import engine
from app.models import RiotCredentials

logger = logging.getLogger(__name__)

REFRESH_INTERVAL_HOURS = 12
SESSION_DURATION_DAYS = 21
RESYNC_THRESHOLD_HOURS = 24

_scheduler: AsyncIOScheduler | None = None


def _now() -> datetime:
    return datetime.now(timezone.utc)


async def refresh_one(creds: RiotCredentials) -> bool:
    """Refresh tokens for a single user. Returns True on success."""
    if not creds.encrypted_cookies:
        return False
    try:
        cookies = json.loads(crypto.decrypt(creds.encrypted_cookies))
    except Exception:
        logger.warning("Cannot decrypt cookies for %s", creds.user_email)
        return False

    try:
        tokens = await riot_client.auth_with_cookies(creds.user_email, cookies)
    except Exception as e:
        logger.info("Refresh failed for %s: %s", creds.user_email, e)
        return False

    riot_client.cache_tokens(creds.user_email, tokens)
    refreshed = tokens.get("_cookies") or {}
    merged = {**cookies, **refreshed}
    creds.encrypted_cookies = crypto.encrypt(json.dumps(merged))
    creds.session_expires_at = _now() + timedelta(days=SESSION_DURATION_DAYS)
    creds.needs_resync = False
    creds.updated_at = _now()
    return True


async def refresh_all_credentials():
    """Called by APScheduler. Refreshes every set of stored cookies and flags stale ones."""
    with Session(engine) as db:
        rows = db.exec(select(RiotCredentials).where(RiotCredentials.encrypted_cookies.is_not(None))).all()
        for creds in rows:
            ok = await refresh_one(creds)
            if not ok:
                expires = creds.session_expires_at
                if expires and expires.tzinfo is None:
                    expires = expires.replace(tzinfo=timezone.utc)
                if not expires or expires < _now() + timedelta(hours=RESYNC_THRESHOLD_HOURS):
                    creds.needs_resync = True
                    creds.updated_at = _now()
            db.add(creds)
        db.commit()
    logger.info("Refresh sweep complete: %s credentials processed", len(rows))


def start():
    global _scheduler
    if _scheduler is not None:
        return
    _scheduler = AsyncIOScheduler(timezone="UTC")
    _scheduler.add_job(
        refresh_all_credentials,
        "interval",
        hours=REFRESH_INTERVAL_HOURS,
        next_run_time=_now() + timedelta(minutes=1),
        id="refresh_all_credentials",
        max_instances=1,
        coalesce=True,
    )
    _scheduler.start()
    logger.info("Scheduler started — refresh_all_credentials every %sh", REFRESH_INTERVAL_HOURS)


def stop():
    global _scheduler
    if _scheduler is not None:
        _scheduler.shutdown(wait=False)
        _scheduler = None
