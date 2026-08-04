from contextlib import asynccontextmanager
from fastapi import FastAPI
from database import init_db
from routes.passkey import router as passkey_router

@asynccontextmanager
async def lifespan(app: FastAPI):
    init_db()
    yield

app = FastAPI(lifespan=lifespan)
app.include_router(passkey_router)

@app.get("/health")
def health():
    return {"status": "ok"}