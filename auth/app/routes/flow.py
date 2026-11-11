from fastapi import APIRouter, Request
from fastapi.responses import JSONResponse, RedirectResponse

from config import NEXT_COOKIE, PUBLIC_URL
from security import clear_cookie, safe_path, set_short_cookie

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

