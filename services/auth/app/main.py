import asyncio
from contextlib import asynccontextmanager
from fastapi import FastAPI
from sqlmodel import Session as DBSession
from database import engine, init_db
from sessions import delete_expired_sessions
from routes.passkey import router as passkey_router
from routes.account import router as account_router
from routes.admin import router as admin_router
from routes.password_auth import router as password_router
from routes.totp import router as totp_router
from routes.flow import router as flow_router
from routes.qr import router as qr_router

# Expired sessions are deleted at startup and then every hour.
EXPIRED_SESSIONS_SWEEP_SECONDS = 3600


def _sweep_expired_sessions() -> None:
    with DBSession(engine) as db:
        deleted = delete_expired_sessions(db)
    if deleted:
        print(f"Deleted {deleted} expired sessions", flush=True)


async def _sweep_forever() -> None:
    while True:
        await asyncio.sleep(EXPIRED_SESSIONS_SWEEP_SECONDS)
        try:
            await asyncio.to_thread(_sweep_expired_sessions)
        except Exception as e:  # a failed sweep waits for the next one, it never takes auth down
            print(f"Expired sessions sweep failed: {e}", flush=True)


@asynccontextmanager
async def lifespan(app: FastAPI):
    init_db()
    _sweep_expired_sessions()
    sweeper = asyncio.create_task(_sweep_forever())
    yield
    sweeper.cancel()

app = FastAPI(lifespan=lifespan, docs_url=None, redoc_url=None, openapi_url=None)
app.include_router(passkey_router)
app.include_router(account_router)
app.include_router(admin_router)
app.include_router(password_router)
app.include_router(totp_router)
app.include_router(flow_router)
app.include_router(qr_router)

@app.get("/health")
def health():
    return {"status": "ok"}
