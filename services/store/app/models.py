from datetime import datetime, timezone
from typing import Optional

from sqlalchemy import UniqueConstraint
from sqlmodel import SQLModel, Field


class FileViewToken(SQLModel, table=True):
    __tablename__ = "file_view_tokens"

    id: Optional[int] = Field(default=None, primary_key=True)
    token: str = Field(unique=True, index=True)
    path: str
    location: str
    owner_email: str
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    first_accessed_at: Optional[datetime] = Field(default=None)


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


class ShareFileToken(SQLModel, table=True):
    __tablename__ = "share_file_tokens"

    id: Optional[int] = Field(default=None, primary_key=True)
    token: str = Field(unique=True, index=True)
    share_token: str = Field(index=True)
    path: str
    session_token: Optional[str] = Field(default=None)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    first_accessed_at: Optional[datetime] = Field(default=None)


class FolderIcon(SQLModel, table=True):
    __tablename__ = "folder_icons"

    id: Optional[int] = Field(default=None, primary_key=True)
    owner_email: str = Field(index=True)
    location: str
    path: str
    icon_name: str


class CompressionJob(SQLModel, table=True):
    __tablename__ = "compression_jobs"

    job_id: str = Field(primary_key=True)
    owner_email: str
    filename: str
    path: str
    location: str
    status: str = Field(default="active")  # active, done, error, cancelled
    total_bytes: int = Field(default=0)
    extracted_bytes: int = Field(default=0)
    error: Optional[str] = Field(default=None)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class PendingUpload(SQLModel, table=True):
    __tablename__ = "pending_uploads"

    upload_id: str = Field(primary_key=True)
    filename: str
    path: str
    location: str
    owner_email: str
    file_size: int
    chunk_size: int = Field(default=0)
    bytes_received: int = Field(default=0)
    total_chunks: int
    # JSON-encoded sorted list of chunk indices already written to disk - lets completion be
    # driven by "every index has arrived" rather than "the chunk tagged as last arrived",
    # since chunks can legitimately arrive out of order or be retried.
    received_chunks: str = Field(default="")
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class Photo(SQLModel, table=True):
    __tablename__ = "photos"
    __table_args__ = (UniqueConstraint("owner_email", "checksum", name="uq_photos_owner_checksum"),)

    id: Optional[int] = Field(default=None, primary_key=True)
    owner_email: str = Field(index=True)
    checksum: str = Field(index=True)
    source_asset_id: Optional[str] = Field(default=None, index=True)
    path: str
    location: str = Field(default="external")
    original_filename: str
    file_size: int
    mime_type: Optional[str] = Field(default=None)
    width: Optional[int] = Field(default=None)
    height: Optional[int] = Field(default=None)
    taken_at: Optional[datetime] = Field(default=None, index=True)
    is_screenshot: bool = Field(default=False)
    is_camera_original: bool = Field(default=False)
    source_type: str = Field(default="unknown")
    camera_make: Optional[str] = Field(default=None)
    camera_model: Optional[str] = Field(default=None)
    gps_lat: Optional[float] = Field(default=None)
    gps_lon: Optional[float] = Field(default=None)
    status: str = Field(default="pending")  # pending, verified, checksum_mismatch, error
    uploaded_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    verified_at: Optional[datetime] = Field(default=None)
