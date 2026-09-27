import os

from cryptography.fernet import Fernet

_key = os.getenv("RIOT_CRED_KEY", "").encode()
try:
    _fernet: Fernet | None = Fernet(_key) if _key else None
except Exception:
    _fernet = None


def encrypt(plaintext: str) -> str:
    if not _fernet:
        raise RuntimeError("RIOT_CRED_KEY not configured")
    return _fernet.encrypt(plaintext.encode()).decode()


def decrypt(ciphertext: str) -> str:
    if not _fernet:
        raise RuntimeError("RIOT_CRED_KEY not configured")
    return _fernet.decrypt(ciphertext.encode()).decode()


def is_configured() -> bool:
    return _fernet is not None
