import httpx
from fastapi import APIRouter, Depends, HTTPException, Request
from sqlmodel import Session, select

from app.auth import get_current_user
from app.database import get_session
from app.models import LinkedAccount, RiotCredentials
from app import riot_client

router = APIRouter(prefix="/api/val/battle-pass", tags=["battle_pass"])


async def _fetch_contract_meta(contract_uuid: str) -> dict:
    try:
        async with httpx.AsyncClient(timeout=8.0) as c:
            r = await c.get(f"https://valorant-api.com/v1/contracts/{contract_uuid}")
            if r.status_code == 200:
                return r.json().get("data", {})
    except Exception:
        pass
    return {}


@router.get("")
async def get_battle_pass(request: Request, db: Session = Depends(get_session)):
    user = await get_current_user(request)
    acc = db.exec(select(LinkedAccount).where(LinkedAccount.user_email == user["email"])).first()
    if not acc:
        raise HTTPException(status_code=404, detail="No linked Riot account")

    creds = db.exec(select(RiotCredentials).where(RiotCredentials.user_email == user["email"])).first()
    tokens = await riot_client.resolve_tokens_from_credentials(user["email"], creds)
    if tokens is None:
        return {"requires_credentials": True}

    puuid = tokens.get("puuid") or acc.puuid
    data = await riot_client.get_contracts(tokens, puuid, region=acc.region)

    active_uuid = data.get("ActiveSpecialContract")
    contracts = data.get("Contracts", [])
    result = []
    for c in contracts:
        contract_uuid = c.get("ContractDefinitionID", "")
        progression = c.get("ProgressionTowardsNextLevel", 0)
        level = c.get("ProgressionLevelReached", 0)
        total_xp = c.get("TotalProgressionEarned", 0)
        meta = await _fetch_contract_meta(contract_uuid)
        result.append({
            "uuid": contract_uuid,
            "name": meta.get("displayName", contract_uuid),
            "is_active_battle_pass": contract_uuid == active_uuid,
            "level": level,
            "progression": progression,
            "total_xp": total_xp,
            "chapters": len(meta.get("content", {}).get("chapters", [])),
        })

    return {
        "active_battle_pass_uuid": active_uuid,
        "processed_xp": data.get("ProcessedMatches", []),
        "contracts": result,
    }
