import os
from contextlib import asynccontextmanager
from pathlib import Path

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.routes.files import router as files_router
from app.routes.admin import router as admin_router


@asynccontextmanager
async def lifespan(app: FastAPI):
    external_base = Path(os.getenv("EXTERNAL_DISK_PATH", "/mnt/external"))
    try:
        (external_base / "shared").mkdir(parents=True, exist_ok=True)
        (external_base / "users").mkdir(parents=True, exist_ok=True)
    except Exception:
        pass
    yield


app = FastAPI(title="TPStore API", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(files_router)
app.include_router(admin_router)
