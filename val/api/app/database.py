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
    with engine.begin() as conn:
        conn.execute(text("PRAGMA journal_mode=WAL"))
        conn.execute(text("PRAGMA synchronous=NORMAL"))
    _migrate()


def _migrate():
    """Additive-only migrations. Each ALTER runs in its own tx so following statements see the new column."""
    deprecated = {
        "riot_credentials": ["encrypted_username", "encrypted_password"],
    }
    for table, cols in deprecated.items():
        with engine.begin() as conn:
            existing = {row[1] for row in conn.execute(text(f"PRAGMA table_info({table})"))}
        for name in cols:
            if name in existing:
                try:
                    with engine.begin() as conn:
                        conn.execute(text(f"ALTER TABLE {table} DROP COLUMN {name}"))
                except Exception:
                    pass

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
    for table, cols in additions.items():
        with engine.begin() as conn:
            existing = {row[1] for row in conn.execute(text(f"PRAGMA table_info({table})"))}
        for name, ddl in cols:
            if name not in existing:
                with engine.begin() as conn:
                    conn.execute(text(f"ALTER TABLE {table} ADD COLUMN {name} {ddl}"))

    with engine.begin() as conn:
        existing = {row[1] for row in conn.execute(text("PRAGMA table_info(riot_credentials)"))}
        if "pair_token" in existing:
            conn.execute(text(
                "CREATE UNIQUE INDEX IF NOT EXISTS ix_riot_credentials_pair_token "
                "ON riot_credentials(pair_token) WHERE pair_token IS NOT NULL"
            ))


def get_session():
    with Session(engine) as session:
        yield session
