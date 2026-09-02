import os
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

import bcrypt
from sqlmodel import Session as DBSession, select
from database import engine, init_db
from models import User

OWNER_EMAIL = os.environ["OWNER_EMAIL"]
OWNER_PASSWORD = os.environ.get("OWNER_PASSWORD", "")

def bootstrap():
    init_db()
    with DBSession(engine) as session:
        owner = session.exec(select(User).where(User.email == OWNER_EMAIL)).first()

        if not owner:
            if not OWNER_PASSWORD:
                print("ERROR: OWNER_PASSWORD env var is required to create the owner account.")
                sys.exit(1)
            password_hash = bcrypt.hashpw(OWNER_PASSWORD.encode(), bcrypt.gensalt()).decode()
            owner = User(
                email=OWNER_EMAIL,
                display_name=OWNER_EMAIL.split("@")[0],
                password_hash=password_hash,
                is_admin=True,
            )
            session.add(owner)
            session.commit()
            print(f"Owner account created: {OWNER_EMAIL}")
        else:
            if not owner.is_admin:
                owner.is_admin = True
                session.add(owner)
                session.commit()
                print(f"{OWNER_EMAIL} set as admin")
            else:
                print(f"{OWNER_EMAIL} already exists and is admin")

            if OWNER_PASSWORD and not owner.password_hash:
                password_hash = bcrypt.hashpw(OWNER_PASSWORD.encode(), bcrypt.gensalt()).decode()
                owner.password_hash = password_hash
                session.add(owner)
                session.commit()
                print(f"Password set for {OWNER_EMAIL}")

if __name__ == "__main__":
    bootstrap()
