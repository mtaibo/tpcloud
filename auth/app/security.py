import hmac
import time
from collections import deque
from datetime import datetime

import pyotp

from fastapi import HTTPException, Response

from config import COOKIE_SECURE, SESSION_COOKIE


def set_session_cookie(response: Response, session_id: str, max_age: int | None) -> None:
    """`max_age=None` makes it a browser-session cookie, gone when the browser closes."""
    response.set_cookie(
        key=SESSION_COOKIE,
        value=session_id,
        path="/",
        httponly=True,
        secure=COOKIE_SECURE,
        samesite="lax",
        max_age=max_age,
    )


def clear_session_cookie(response: Response) -> None:
    response.delete_cookie(key=SESSION_COOKIE, path="/", secure=COOKIE_SECURE, httponly=True, samesite="lax")


def set_short_cookie(response: Response, key: str, value: str, max_age: int) -> None:
    response.set_cookie(
        key=key, value=value, path="/", httponly=True, secure=COOKIE_SECURE, samesite="lax", max_age=max_age
    )


def clear_cookie(response: Response, key: str) -> None:
    response.delete_cookie(key=key, path="/", secure=COOKIE_SECURE, httponly=True, samesite="lax")


def safe_path(value: str | None) -> str:
    """Only ever a path on this host, so nothing can send a person elsewhere after signing in."""
    if not value or not value.startswith("/") or value.startswith("//") or "\\" in value:
        return "/"
    if value == "/login" or value.startswith("/login/") or value.startswith("/login?"):
        return "/"
    return value[:2000]


class FailureLimiter:
    """Counts failed attempts per key (an email, an IP, a user) in a sliding window and turns the
    key away once it reaches the limit, until the oldest failure falls out of the window. In memory:
    the service runs as a single process, and a restart only gives an attacker a fresh window."""

    def __init__(self, limit: int, window_seconds: float, max_keys: int = 10_000):
        self.limit = limit
        self.window = window_seconds
        self.max_keys = max_keys
        self._failures: dict[str, deque[float]] = {}

    def _recent(self, key: str, now: float) -> deque[float]:
        failures = self._failures.get(key)
        if failures is None:
            return deque()
        while failures and failures[0] <= now - self.window:
            failures.popleft()
        if not failures:
            del self._failures[key]
        return failures

    def check(self, *keys: str) -> None:
        now = time.monotonic()
        if any(len(self._recent(k, now)) >= self.limit for k in keys):
            raise HTTPException(status_code=429, detail="Demasiados intentos. Espera unos minutos.")

    def fail(self, *keys: str) -> None:
        now = time.monotonic()
        if len(self._failures) >= self.max_keys:
            for k in list(self._failures):
                self._recent(k, now)
        for k in keys:
            self._failures.setdefault(k, deque()).append(now)

    def reset(self, *keys: str) -> None:
        for k in keys:
            self._failures.pop(k, None)


class ExpiringDict:
    """A dict whose entries vanish after `ttl` seconds and that never holds more than `max_size`,
    for state anyone can create without signing in (passkey challenges), so it can't grow forever."""

    def __init__(self, ttl: float, max_size: int = 10_000):
        self.ttl = ttl
        self.max_size = max_size
        self._items: dict = {}

    def _prune(self, now: float) -> None:
        for k in [k for k, (_, t) in self._items.items() if t <= now]:
            del self._items[k]

    def set(self, key, value) -> None:
        now = time.monotonic()
        if len(self._items) >= self.max_size:
            self._prune(now)
            while len(self._items) >= self.max_size:
                del self._items[next(iter(self._items))]
        self._items[key] = (value, now + self.ttl)

    def pop(self, key):
        item = self._items.pop(key, None)
        if item is None or item[1] <= time.monotonic():
            return None
        return item[0]


# 5 wrong passwords for one account, or 20 from one address, in 15 minutes.
login_failures = FailureLimiter(limit=5, window_seconds=15 * 60)
ip_failures = FailureLimiter(limit=20, window_seconds=15 * 60)
# Wrong TOTP codes (sign-in, setup, disabling) and wrong current passwords, per user.
user_failures = FailureLimiter(limit=5, window_seconds=15 * 60)


_last_totp_step: dict[int, int] = {}


def verify_totp(secret: str, code: str, user_id: int) -> bool:
    """A code from the current 30 s step or the one either side of it, each usable only once per
    user, so a code seen over someone's shoulder can't be replayed while it's still valid."""
    code = code.strip()
    if not (len(code) == 6 and code.isdigit()):
        return False
    totp = pyotp.TOTP(secret)
    now_step = totp.timecode(datetime.now())
    for step in (now_step - 1, now_step, now_step + 1):
        if hmac.compare_digest(totp.generate_otp(step), code):
            if step <= _last_totp_step.get(user_id, -1):
                return False
            _last_totp_step[user_id] = step
            return True
    return False
