import os
from urllib.parse import urlsplit

# The one address every module is served from, e.g. https://cloud.example.com. Each module is a
# path under it (/login, /store, /home…), so the session cookie never has to leave this host.
PUBLIC_URL = os.environ["TPCLOUD_URL"].rstrip("/")

_url = urlsplit(PUBLIC_URL)
ORIGIN = f"{_url.scheme}://{_url.netloc}"

# Passkeys are bound to this. It may be a parent domain of the host (passkeys made back when
# every module had its own subdomain were registered for the bare domain), so it can be overridden.
RP_ID = os.environ.get("RP_ID") or _url.hostname
RP_NAME = os.environ.get("RP_NAME") or "TPCloud"

COOKIE_SECURE = _url.scheme == "https"
# `__Host-` makes the browser refuse the cookie unless it's Secure, host-only and on Path=/,
# so no other site or subdomain can set or overwrite it.
SESSION_COOKIE = "__Host-tpcloud_session" if COOKIE_SECURE else "tpcloud_session"

# Modules this server runs (store, home, admin…), so clients only show what's there.
MODULES = [m.strip() for m in os.environ.get("TPCLOUD_MODULES", "").split(",") if m.strip()]

# Looking up where a session signs in from sends its IP to ip-api.com, so it's opt-in.
GEOIP = os.environ.get("TPCLOUD_GEOIP") == "1"
