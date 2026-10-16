import os
from pathlib import Path

from sqlalchemy import text
from sqlmodel import SQLModel, create_engine, Session

DATA_DIR = Path(os.getenv("DATA_DIR", "/app/data"))
DATABASE_URL = f"sqlite:///{DATA_DIR}/shares.db"

engine = create_engine(DATABASE_URL, connect_args={"check_same_thread": False})


def init_db():
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    SQLModel.metadata.create_all(engine)
    with engine.connect() as conn:
        conn.execute(text("PRAGMA journal_mode=WAL"))
        conn.execute(text("PRAGMA synchronous=NORMAL"))
        conn.commit()
        cols = {row[1] for row in conn.execute(text("PRAGMA table_info(share_links)")).fetchall()}
        if "public" not in cols:
            conn.execute(text("ALTER TABLE share_links ADD COLUMN public INTEGER NOT NULL DEFAULT 0"))
            conn.commit()

        upload_cols = {row[1] for row in conn.execute(text("PRAGMA table_info(pending_uploads)")).fetchall()}
        if "chunk_size" not in upload_cols:
            conn.execute(text("ALTER TABLE pending_uploads ADD COLUMN chunk_size INTEGER NOT NULL DEFAULT 0"))
            conn.commit()
        if "received_chunks" not in upload_cols:
            conn.execute(text("ALTER TABLE pending_uploads ADD COLUMN received_chunks TEXT NOT NULL DEFAULT ''"))
            conn.commit()


def get_session():
    with Session(engine) as session:
        yield session
