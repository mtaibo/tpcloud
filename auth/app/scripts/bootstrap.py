import os
from sqlmodel import Session as DBSession, select
from database import engine, init_db
from models import AllowedEmail

OWNER_EMAIL = os.environ["OWNER_EMAIL"]

def bootstrap():
    init_db()
    with DBSession(engine) as session:
        existing = session.exec(
            select(AllowedEmail).where(AllowedEmail.email == OWNER_EMAIL)
        ).first()

        if existing:
            print(f"{OWNER_EMAIL} is on allowed_emails.")
            return

        session.add(AllowedEmail(email=OWNER_EMAIL, invited_by=None))
        session.commit()
        print(f"{OWNER_EMAIL} added to allowed_emails")

if __name__ == "__main__":
    bootstrap()
