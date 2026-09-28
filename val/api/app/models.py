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
    encrypted_ssid: Optional[str] = Field(default=None)
    encrypted_cookies: Optional[str] = Field(default=None)
    pair_token: Optional[str] = Field(default=None, unique=True, index=True)
    last_sync_at: Optional[datetime] = Field(default=None)
    session_expires_at: Optional[datetime] = Field(default=None)
    needs_resync: bool = Field(default=False)
    extension_version: Optional[str] = Field(default=None)
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


class WishlistItem(SQLModel, table=True):
    __tablename__ = "wishlist_items"

    id: Optional[int] = Field(default=None, primary_key=True)
    user_email: str = Field(index=True)
    skin_uuid: str = Field(index=True)
    priority: int = Field(default=0)
    note: Optional[str] = Field(default=None)
    notified_at: Optional[datetime] = Field(default=None)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class ShopHistoryEntry(SQLModel, table=True):
    __tablename__ = "shop_history"

    id: Optional[int] = Field(default=None, primary_key=True)
    user_email: str = Field(index=True)
    shop_date: str = Field(index=True)  # YYYY-MM-DD (UTC)
    offers_json: str
    bundle_json: Optional[str] = Field(default=None)
    recorded_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class LoadoutPreset(SQLModel, table=True):
    __tablename__ = "loadout_presets"

    id: Optional[int] = Field(default=None, primary_key=True)
    user_email: str = Field(index=True)
    name: str
    payload_json: str  # full Riot loadout payload (Guns, Sprays, Identity, Incognito)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
