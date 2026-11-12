from sqlmodel import SQLModel, Field
from datetime import datetime, timezone
from typing import Optional
import uuid


class User(SQLModel, table=True):
    __tablename__ = "users"

    id: Optional[int] = Field(default=None, primary_key=True)
    email: str = Field(unique=True, index=True)
    display_name: str
    is_admin: bool = Field(default=False)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    password_hash: Optional[str] = Field(default=None)
    totp_secret: Optional[str] = Field(default=None)
    # Failed sign-ins up to here were confirmed as the owner's own, so they no longer warn.
    failures_reviewed_at: Optional[datetime] = Field(default=None)
    # "en" or "es", picked in Cuenta → Idioma; unset until then (the web follows the browser).
    language: Optional[str] = Field(default=None)


class PasskeyCredential(SQLModel, table=True):
    __tablename__ = "passkey_credentials"

    id: Optional[int] = Field(default=None, primary_key=True)
    user_id: int = Field(foreign_key="users.id", index=True)
    credential_id: str = Field(unique=True, index=True)
    public_key: str
    device_name: str
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    last_used_at: Optional[datetime] = Field(default=None)


class Session(SQLModel, table=True):
    __tablename__ = "sessions"

    id: Optional[int] = Field(default=None, primary_key=True)
    session_id: str = Field(
        unique=True, index=True, default_factory=lambda: uuid.uuid4().hex
    )
    user_id: int = Field(foreign_key="users.id", index=True)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    expires_at: datetime
    ip_address: Optional[str] = Field(default=None)
    user_agent: Optional[str] = Field(default=None)
    totp_verified: bool = Field(default=True)
    auth_method: str = Field(default="passkey")
    device_info: Optional[str] = Field(default=None)
    location: Optional[str] = Field(default=None)
    client_type: str = Field(default="web")
    # "Keep me signed in": a month that renews itself while it's used, instead of half a day.
    remember: bool = Field(default=False)
    last_seen_at: Optional[datetime] = Field(default=None)
    last_ip: Optional[str] = Field(default=None)
    # For a session approved by scanning a QR code: the device that approved it.
    approved_by: Optional[str] = Field(default=None)


class LoginEvent(SQLModel, table=True):
    """Every sign-in, failed attempt and revoked session on an account, so its owner can see who
    got in, from where and how."""

    __tablename__ = "login_events"

    id: Optional[int] = Field(default=None, primary_key=True)
    user_id: int = Field(foreign_key="users.id", index=True)
    at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc), index=True)
    # signin | failed | revoked | signout
    kind: str
    method: Optional[str] = Field(default=None)
    detail: Optional[str] = Field(default=None)
    ip_address: Optional[str] = Field(default=None)
    location: Optional[str] = Field(default=None)
    device_info: Optional[str] = Field(default=None)
    user_agent: Optional[str] = Field(default=None)
    client_type: str = Field(default="web")
