"""Accounts from the server itself, for the `tpcloud` command: whoever can run it here already owns
the server, so no session is involved.

    python scripts/manage.py list                       one JSON object per line
    NEW_PASSWORD=... python scripts/manage.py add <email> [--admin]
"""
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

import bcrypt
from sqlmodel import Session as DBSession, select
from database import engine, init_db
from models import User

MIN_PASSWORD = 12


def list_users() -> None:
    with DBSession(engine) as session:
        for user in session.exec(select(User).order_by(User.id)).all():
            print(json.dumps({
                "email": user.email,
                "name": user.display_name,
                "admin": bool(user.is_admin),
                "totp": user.totp_secret is not None,
            }))


def add_user(email: str, admin: bool) -> None:
    email = email.strip().lower()
    password = os.environ.get("NEW_PASSWORD", "")
    if "@" not in email:
        sys.exit("That doesn't look like an email.")
    if len(password) < MIN_PASSWORD:
        sys.exit(f"The password needs {MIN_PASSWORD} characters or more.")
    with DBSession(engine) as session:
        if session.exec(select(User).where(User.email == email)).first():
            sys.exit(f"{email} already has an account.")
        session.add(User(
            email=email,
            display_name=email.split("@")[0],
            password_hash=bcrypt.hashpw(password.encode(), bcrypt.gensalt()).decode(),
            is_admin=admin,
        ))
        session.commit()
    print(f"{email} can sign in now{' (admin)' if admin else ''}.")


if __name__ == "__main__":
    init_db()
    args = sys.argv[1:]
    if args[:1] == ["list"]:
        list_users()
    elif args[:1] == ["add"] and len(args) >= 2:
        add_user(args[1], "--admin" in args[2:])
    else:
        sys.exit(__doc__)
