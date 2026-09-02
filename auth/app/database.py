import os
from dotenv import load_dotenv, find_dotenv
from sqlmodel import create_engine, SQLModel, Session as DBSession
from sqlalchemy import text

load_dotenv(find_dotenv())

DATABASE_URL = os.getenv("DATABASE_URL", "sqlite:///tpauth.db")

engine = create_engine(DATABASE_URL, echo=False)


def init_db():
    SQLModel.metadata.create_all(engine)
    with engine.connect() as conn:
        for sql in [
            "ALTER TABLE users ADD COLUMN is_admin BOOLEAN NOT NULL DEFAULT 0",
            "ALTER TABLE users ADD COLUMN password_hash TEXT",
            "ALTER TABLE users ADD COLUMN totp_secret TEXT",
            "ALTER TABLE sessions ADD COLUMN totp_verified BOOLEAN NOT NULL DEFAULT 1",
            "ALTER TABLE sessions ADD COLUMN auth_method TEXT NOT NULL DEFAULT 'passkey'",
            "ALTER TABLE sessions ADD COLUMN device_info TEXT",
            "ALTER TABLE sessions ADD COLUMN location TEXT",
        ]:
            try:
                conn.execute(text(sql))
                conn.commit()
            except Exception:
                pass


def get_session():
    with DBSession(engine) as session:
        yield session
