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
    # Add columns introduced after initial schema
    with engine.connect() as conn:
        cols = {row[1] for row in conn.execute(text("PRAGMA table_info(share_links)")).fetchall()}
        if "public" not in cols:
            conn.execute(text("ALTER TABLE share_links ADD COLUMN public INTEGER NOT NULL DEFAULT 0"))
            conn.commit()


def get_session():
    with Session(engine) as session:
        yield session
