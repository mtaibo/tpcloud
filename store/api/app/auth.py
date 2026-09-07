import os
import httpx
from fastapi import HTTPException, Request

AUTH_SERVICE_URL = os.getenv("AUTH_SERVICE_URL", "http://auth:8000")


async def get_current_user(request: Request) -> dict:
    cookie = request.headers.get("cookie", "")
    async with httpx.AsyncClient() as client:
        resp = await client.get(
            f"{AUTH_SERVICE_URL}/auth/passkey/me",
            headers={"cookie": cookie},
        )
    if resp.status_code != 200:
        raise HTTPException(status_code=401, detail="Not authenticated")
    return resp.json()
