from datetime import datetime, timezone
from typing import Optional

from sqlmodel import SQLModel, Field


class LinkedAccount(SQLModel, table=True):
    __tablename__ = "linked_accounts"

    id: Optional[int] = Field(default=None, primary_key=True)
    user_email: str = Field(unique=True, index=True)
    riot_name: str
    riot_tag: str
    region: str
    puuid: Optional[str] = Field(default=None)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class RiotCredentials(SQLModel, table=True):
    __tablename__ = "riot_credentials"

    id: Optional[int] = Field(default=None, primary_key=True)
    user_email: str = Field(unique=True, index=True)
    encrypted_username: Optional[str] = Field(default=None)
    encrypted_password: Optional[str] = Field(default=None)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class ShopCache(SQLModel, table=True):
    __tablename__ = "shop_cache"

    id: Optional[int] = Field(default=None, primary_key=True)
    user_email: str = Field(unique=True, index=True)
    offers_json: str
    bundle_json: Optional[str] = Field(default=None)
    cached_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    expires_at: datetime


class RankHistory(SQLModel, table=True):
    __tablename__ = "rank_history"

    id: Optional[int] = Field(default=None, primary_key=True)
    user_email: str = Field(index=True)
    match_id: str = Field(index=True)
    tier: str
    tier_image_url: Optional[str] = Field(default=None)
    mmr: int
    mmr_change: int
    recorded_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
