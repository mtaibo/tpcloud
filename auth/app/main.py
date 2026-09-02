from contextlib import asynccontextmanager
from fastapi import FastAPI
from database import init_db
from routes.passkey import router as passkey_router
from routes.account import router as account_router
from routes.admin import router as admin_router
from routes.password_auth import router as password_router
from routes.totp import router as totp_router

@asynccontextmanager
async def lifespan(app: FastAPI):
    init_db()
    yield

app = FastAPI(lifespan=lifespan)
app.include_router(passkey_router)
app.include_router(account_router)
app.include_router(admin_router)
app.include_router(password_router)
app.include_router(totp_router)

@app.get("/health")
def health():
    return {"status": "ok"}
