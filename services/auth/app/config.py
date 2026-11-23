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

# Where to go back to after signing in, kept in a short-lived cookie so the login page's address
# stays a plain /login.
NEXT_COOKIE = "__Host-tpcloud_next" if COOKIE_SECURE else "tpcloud_next"
# Ties a browser to the QR sign-in it's showing, so only that browser can collect the session.
QR_COOKIE = "__Host-tpcloud_qr" if COOKIE_SECURE else "tpcloud_qr"

# The network this server is the host of, as /.well-known/tpcloud tells the apps. The id never
# changes once made (the `tpcloud` command writes it); the name is what people see.
NETWORK_ID = os.environ.get("TPCLOUD_NETWORK_ID", "")
NETWORK_NAME = os.environ.get("TPCLOUD_NETWORK_NAME") or _url.hostname or "TPCloud"
VERSION = os.environ.get("TPCLOUD_VERSION", "")

# Every address the network answers on (TPCLOUD_URL first), comma-separated: an app keeps them all
# and uses the fastest it can reach, e.g. the local one at home and the public one elsewhere.
ADDRESSES = list(dict.fromkeys(
    [PUBLIC_URL] + [a.strip().rstrip("/") for a in os.environ.get("TPCLOUD_ADDRESSES", "").split(",") if a.strip()]
))

# The host's own certificate authority (Caddy's, for local addresses), so apps can pin it.
CA_CERT = os.environ.get("TPCLOUD_CA_CERT", "")

# Passkeys work wherever the browser allows them: on the addresses under RP_ID (a .local or
# *.ts.net address can't use a passkey made for the public domain, and the browser says so).
ORIGINS = [ORIGIN] + [
    o for o in (f"{urlsplit(a).scheme}://{urlsplit(a).netloc}" for a in ADDRESSES[1:])
    if (h := urlsplit(o).hostname or "") and (h == RP_ID or h.endswith("." + RP_ID)) and o != ORIGIN
]

# Which of them are only reachable at home (the local site's names, whatever they look like:
# cloud.casa is as local as tphome.local), as the `tpcloud` command knows.
LOCAL_ADDRESSES = {a.strip().rstrip("/") for a in os.environ.get("TPCLOUD_LOCAL_ADDRESSES", "").split(",") if a.strip()}
