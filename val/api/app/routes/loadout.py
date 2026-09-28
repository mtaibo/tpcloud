import json
from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import BaseModel
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount, LoadoutPreset, RiotCredentials
from app import riot_client

router = APIRouter(prefix="/api/val/loadout", tags=["loadout"])


class SavePresetBody(BaseModel):
    name: str
    payload: dict


class UpdatePresetBody(BaseModel):
    name: str | None = None
    payload: dict | None = None


async def _tokens_for(user_email: str, db: Session):
    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user_email)).first()
    tokens = await riot_client.resolve_tokens_from_credentials(user_email, creds)
    return tokens


@router.get("/current")
async def get_current_loadout(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        raise HTTPException(status_code=404, detail="No linked Riot account")
    tokens = await _tokens_for(user["email"], db)
    if tokens is None:
        return {"requires_credentials": True}
    puuid = tokens.get("puuid") or acc.puuid
    return await riot_client.get_loadout(tokens, puuid, region=acc.region)


@router.get("/presets")
async def list_presets(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    rows = db.exec(
        select(LoadoutPreset)
        .where(LoadoutPreset.user_email == user["email"])
        .order_by(LoadoutPreset.updated_at.desc())
    ).all()
    return {
        "presets": [
            {
                "id": r.id,
                "name": r.name,
                "created_at": r.created_at.isoformat(),
                "updated_at": r.updated_at.isoformat(),
            }
            for r in rows
        ]
    }


@router.post("/presets")
async def save_preset(body: SavePresetBody, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    preset = LoadoutPreset(
        user_email=user["email"],
        name=body.name,
        payload_json=json.dumps(body.payload),
    )
    db.add(preset)
    db.commit()
    db.refresh(preset)
    return {"id": preset.id, "name": preset.name}


@router.get("/presets/{preset_id}")
async def get_preset(preset_id: int, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    preset = db.exec(
        select(LoadoutPreset).where(
            LoadoutPreset.id == preset_id,
            LoadoutPreset.user_email == user["email"],
        )
    ).first()
    if not preset:
        raise HTTPException(status_code=404, detail="Preset not found")
    return {
        "id": preset.id,
        "name": preset.name,
        "payload": json.loads(preset.payload_json),
    }


@router.patch("/presets/{preset_id}")
async def update_preset(preset_id: int, body: UpdatePresetBody, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    preset = db.exec(
        select(LoadoutPreset).where(
            LoadoutPreset.id == preset_id,
            LoadoutPreset.user_email == user["email"],
        )
    ).first()
    if not preset:
        raise HTTPException(status_code=404, detail="Preset not found")
    if body.name is not None:
        preset.name = body.name
    if body.payload is not None:
        preset.payload_json = json.dumps(body.payload)
    preset.updated_at = datetime.now(timezone.utc)
    db.add(preset)
    db.commit()
    return {"ok": True}


@router.delete("/presets/{preset_id}")
async def delete_preset(preset_id: int, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    preset = db.exec(
        select(LoadoutPreset).where(
            LoadoutPreset.id == preset_id,
            LoadoutPreset.user_email == user["email"],
        )
    ).first()
    if preset:
        db.delete(preset)
        db.commit()
    return {"ok": True}


@router.post("/presets/{preset_id}/apply")
async def apply_preset(preset_id: int, request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    preset = db.exec(
        select(LoadoutPreset).where(
            LoadoutPreset.id == preset_id,
            LoadoutPreset.user_email == user["email"],
        )
    ).first()
    if not preset:
        raise HTTPException(status_code=404, detail="Preset not found")

    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        raise HTTPException(status_code=404, detail="No linked Riot account")

    tokens = await _tokens_for(user["email"], db)
    if tokens is None:
        raise HTTPException(status_code=401, detail="Credentials required")

    puuid = tokens.get("puuid") or acc.puuid
    payload = json.loads(preset.payload_json)
    result = await riot_client.set_loadout(tokens, puuid, payload, region=acc.region)
    return {"ok": True, "loadout": result}
