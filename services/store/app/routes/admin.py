import os

import httpx
from fastapi import APIRouter, HTTPException, Request

from app.auth import get_current_user

router = APIRouter(prefix="/api/admin", tags=["admin"])

AUTH_SERVICE_URL = os.getenv("AUTH_SERVICE_URL", "http://auth:8000")


@router.get("/users")
async def list_users(request: Request):
    user = await get_current_user(request)
    if not user["is_admin"]:
        raise HTTPException(403, "Admin access required")
    cookie = request.headers.get("cookie", "")
    async with httpx.AsyncClient() as client:
        resp = await client.get(
            f"{AUTH_SERVICE_URL}/auth/admin/users",
            headers={"cookie": cookie},
        )
    if resp.status_code != 200:
        raise HTTPException(502, "Failed to fetch users from auth service")
    return resp.json()
