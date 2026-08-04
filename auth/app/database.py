import os
from dotenv import load_dotenv, find_dotenv
from sqlmodel import create_engine, SQLModel, Session as DBSession

load_dotenv(find_dotenv())

DATABASE_URL = os.getenv("DATABASE_URL", "sqlite:///tpauth.db")

engine = create_engine(DATABASE_URL, echo=False)


def init_db():
    SQLModel.metadata.create_all(engine)


def get_session():
    with DBSession(engine) as session:
        yield session
