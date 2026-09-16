import os
from pathlib import Path

from sqlmodel import SQLModel, create_engine, Session

DATA_DIR = Path(os.getenv("DATA_DIR", "/app/data"))
DATABASE_URL = f"sqlite:///{DATA_DIR}/shares.db"

engine = create_engine(DATABASE_URL, connect_args={"check_same_thread": False})


def init_db():
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    SQLModel.metadata.create_all(engine)


def get_session():
    with Session(engine) as session:
        yield session
