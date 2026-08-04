from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException
from sqlmodel import Session as DBSession, select, func

from database import get_session
from dependencies import require_admin
from models import AllowedEmail, PasskeyCredential, Session as SessionModel, User

router = APIRouter(prefix="/auth/admin")


@router.get("/users")
def list_users(
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    users = session.exec(select(User)).all()
    now = datetime.now(timezone.utc).replace(tzinfo=None)
    result = []
    for u in users:
        passkey_count = session.exec(
            select(func.count()).where(PasskeyCredential.user_id == u.id)
        ).one()
        session_count = session.exec(
            select(func.count()).where(
                SessionModel.user_id == u.id,
                SessionModel.expires_at > now,
            )
        ).one()
        result.append({
            "id": u.id,
            "email": u.email,
            "display_name": u.display_name,
            "is_admin": u.is_admin,
            "created_at": u.created_at.isoformat(),
            "passkey_count": passkey_count,
            "active_session_count": session_count,
        })
    return result


@router.patch("/users/{user_id}")
def toggle_admin(
    user_id: int,
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    if user_id == admin.id:
        raise HTTPException(status_code=400, detail="No puedes cambiar tu propio rol")
    user = session.get(User, user_id)
    if not user:
        raise HTTPException(status_code=404, detail="Usuario no encontrado")
    user.is_admin = not user.is_admin
    session.add(user)
    session.commit()
    return {"id": user.id, "is_admin": user.is_admin}


@router.delete("/users/{user_id}")
def delete_user(
    user_id: int,
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    if user_id == admin.id:
        raise HTTPException(status_code=400, detail="No puedes eliminarte a ti mismo")
    user = session.get(User, user_id)
    if not user:
        raise HTTPException(status_code=404, detail="Usuario no encontrado")

    for p in session.exec(select(PasskeyCredential).where(PasskeyCredential.user_id == user_id)).all():
        session.delete(p)
    for s in session.exec(select(SessionModel).where(SessionModel.user_id == user_id)).all():
        session.delete(s)
    session.delete(user)
    session.commit()
    return {"status": "eliminado"}


@router.get("/sessions")
def list_sessions(
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    now = datetime.now(timezone.utc).replace(tzinfo=None)
    sessions = session.exec(
        select(SessionModel).where(SessionModel.expires_at > now)
    ).all()
    result = []
    for s in sessions:
        user = session.get(User, s.user_id)
        result.append({
            "session_id": s.session_id,
            "user_email": user.email if user else "—",
            "created_at": s.created_at.isoformat(),
            "expires_at": s.expires_at.isoformat(),
            "ip_address": s.ip_address,
        })
    return result


@router.delete("/sessions/{session_id}")
def revoke_session(
    session_id: str,
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    db_session = session.exec(
        select(SessionModel).where(SessionModel.session_id == session_id)
    ).first()
    if not db_session:
        raise HTTPException(status_code=404, detail="Sesión no encontrada")
    session.delete(db_session)
    session.commit()
    return {"status": "revocada"}


@router.get("/invites")
def list_invites(
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    invites = session.exec(select(AllowedEmail)).all()
    return [
        {
            "email": i.email,
            "used": i.used,
            "invited_by": i.invited_by,
            "created_at": i.created_at.isoformat(),
        }
        for i in invites
    ]


@router.post("/invites")
def create_invite(
    body: dict,
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    email = body.get("email", "").strip().lower()
    if not email:
        raise HTTPException(status_code=400, detail="Email requerido")
    existing = session.exec(select(AllowedEmail).where(AllowedEmail.email == email)).first()
    if existing:
        raise HTTPException(status_code=409, detail="Email ya invitado")
    invite = AllowedEmail(email=email, invited_by=admin.id)
    session.add(invite)
    session.commit()
    return {"email": email, "status": "invitado"}


@router.delete("/invites/{email}")
def revoke_invite(
    email: str,
    admin: User = Depends(require_admin),
    session: DBSession = Depends(get_session),
):
    invite = session.exec(select(AllowedEmail).where(AllowedEmail.email == email)).first()
    if not invite:
        raise HTTPException(status_code=404, detail="Invitación no encontrada")
    if invite.used:
        raise HTTPException(status_code=400, detail="La invitación ya fue usada")
    session.delete(invite)
    session.commit()
    return {"status": "revocada"}
