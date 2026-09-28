import os
from pathlib import Path

from sqlalchemy import text
from sqlmodel import SQLModel, create_engine, Session

DATA_DIR = Path(os.getenv("DATA_DIR", "/app/data"))
DATABASE_URL = f"sqlite:///{DATA_DIR}/val.db"

engine = create_engine(DATABASE_URL, connect_args={"check_same_thread": False})


def init_db():
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    SQLModel.metadata.create_all(engine)
    with engine.connect() as conn:
        conn.execute(text("PRAGMA journal_mode=WAL"))
        conn.execute(text("PRAGMA synchronous=NORMAL"))
        _migrate(conn)
        conn.commit()


def _migrate(conn):
    """Additive-only migrations: ALTER TABLE ADD COLUMN when missing."""
    additions = {
        "riot_credentials": [
            ("encrypted_cookies", "TEXT"),
            ("pair_token", "TEXT"),
            ("last_sync_at", "DATETIME"),
            ("session_expires_at", "DATETIME"),
            ("needs_resync", "INTEGER DEFAULT 0"),
            ("extension_version", "TEXT"),
        ],
    }
    conn.execute(text(
        "CREATE UNIQUE INDEX IF NOT EXISTS ix_riot_credentials_pair_token "
        "ON riot_credentials(pair_token) WHERE pair_token IS NOT NULL"
    ))
    for table, cols in additions.items():
        existing = {row[1] for row in conn.execute(text(f"PRAGMA table_info({table})"))}
        for name, ddl in cols:
            if name not in existing:
                conn.execute(text(f"ALTER TABLE {table} ADD COLUMN {name} {ddl}"))


def get_session():
    with Session(engine) as session:
        yield session
