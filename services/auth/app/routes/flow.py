from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, Request
from fastapi.responses import JSONResponse, RedirectResponse
from sqlmodel import Session as DBSession

from config import NEXT_COOKIE, PUBLIC_URL
from database import get_session
from dependencies import _get_db_session_and_user
from security import clear_cookie, safe_path, set_session_cookie, set_short_cookie
from utils import cookie_lifetime, session_duration_for

router = APIRouter(prefix="/auth")

NEXT_COOKIE_AGE = 15 * 60


def redirect_to_login(next_path: str | None) -> RedirectResponse:
    """To the login page at its plain address, remembering where to come back to in a cookie
    instead of in the URL."""
    response = RedirectResponse(url=f"{PUBLIC_URL}/login", status_code=302)
    path = safe_path(next_path)
    if path != "/":
        set_short_cookie(response, NEXT_COOKIE, path, NEXT_COOKIE_AGE)
    return response


@router.get("/signin")
def signin(next: str | None = None):
    """For pages that find out they're signed out on their own (e.g. a 401 from their API):
    /auth/signin?next=/store/ lands on /login and returns to /store/ afterwards."""
    return redirect_to_login(next)


@router.post("/login/continue")
def login_continue(request: Request):
    """Where the login page should go now that it's signed in, and forget it."""
    path = safe_path(request.cookies.get(NEXT_COOKIE))
    response = JSONResponse({"next": path})
    clear_cookie(response, NEXT_COOKIE)
    return response



@router.post("/login/remember")
def login_remember(body: dict, request: Request, session: DBSession = Depends(get_session)):
    """The login page asks "keep me signed in?" once the browser is already in: every web sign-in
    starts as a browser-session one, and this turns it into a month-long one (or leaves it)."""
    db_session, _ = _get_db_session_and_user(request, session)
    if not db_session.totp_verified:
        raise HTTPException(status_code=403, detail="Two-step verification pending")
    response = JSONResponse({"remember": db_session.remember})
    if db_session.client_type != "web":
        return response
    db_session.remember = bool(body.get("remember", False))
    db_session.expires_at = datetime.now(timezone.utc) + session_duration_for(db_session.client_type, db_session.remember)
    session.add(db_session)
    session.commit()
    response = JSONResponse({"remember": db_session.remember})
    set_session_cookie(response, db_session.session_id, cookie_lifetime(db_session))
    return response
