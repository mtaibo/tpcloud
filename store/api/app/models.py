from datetime import datetime, timezone
from typing import Optional

from sqlmodel import SQLModel, Field


class ShareLink(SQLModel, table=True):
    __tablename__ = "share_links"

    id: Optional[int] = Field(default=None, primary_key=True)
    token: str = Field(unique=True, index=True)
    location: str
    path: str
    owner_email: str
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    hidden: bool = Field(default=False)
    password_hash: Optional[str] = Field(default=None)
    editable: bool = Field(default=False)
    public: bool = Field(default=False)


class ShareSession(SQLModel, table=True):
    __tablename__ = "share_sessions"

    id: Optional[int] = Field(default=None, primary_key=True)
    session_token: str = Field(unique=True, index=True)
    share_token: str = Field(index=True)
    expires_at: datetime
