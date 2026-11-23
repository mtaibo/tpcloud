import base64
import hashlib
import ipaddress
import os
from functools import lru_cache
from urllib.parse import urlsplit

from fastapi import APIRouter

from config import ADDRESSES, CA_CERT, MODULES, NETWORK_ID, NETWORK_NAME, VERSION

router = APIRouter(prefix="/auth")

# Bumped only when the card changes in a way older apps can't read.
CARD_VERSION = 1


def address_kind(url: str) -> str:
    """local (only reachable at home), tailscale or public, so an app knows which to try first."""
    host = urlsplit(url).hostname or ""
    if host.endswith(".ts.net"):
        return "tailscale"
    if host.endswith(".local") or host == "localhost" or "." not in host:
        return "local"
    try:
        ip = ipaddress.ip_address(host)
    except ValueError:
        return "public"
    if ip in ipaddress.ip_network("100.64.0.0/10"):
        return "tailscale"
    return "local" if ip.is_private or ip.is_loopback or ip.is_link_local else "public"


def _ca_fingerprint(path: str) -> str | None:
    """SHA-256 of the CA certificate's DER bytes, the same value `openssl x509 -fingerprint`
    gives, as lowercase hex. The gateway makes the certificate after auth starts, so it's looked
    for on every call and read again only when it changes."""
    if not path:
        return None
    try:
        mtime = os.stat(path).st_mtime_ns
    except OSError:
        return None
    return _fingerprint_of(path, mtime)


@lru_cache(maxsize=4)
def _fingerprint_of(path: str, mtime: int) -> str | None:
    try:
        pem = open(path).read()
    except OSError:
        return None
    body = pem.split("-----BEGIN CERTIFICATE-----", 1)[-1].split("-----END CERTIFICATE-----", 1)[0]
    try:
        der = base64.b64decode("".join(body.split()), validate=True)
    except ValueError:
        return None
    return hashlib.sha256(der).hexdigest() if der else None


@router.get("/network")
def network_card():
    """What an app needs to add this network: who it is, where it answers and which certificate
    its local addresses use. Public: it's read before anyone signs in (also served at
    /.well-known/tpcloud)."""
    return {
        "tpcloud": CARD_VERSION,
        "id": NETWORK_ID,
        "name": NETWORK_NAME,
        "version": VERSION,
        "addresses": [{"url": url, "kind": address_kind(url)} for url in ADDRESSES],
        "modules": MODULES,
        "ca_sha256": _ca_fingerprint(CA_CERT),
    }
