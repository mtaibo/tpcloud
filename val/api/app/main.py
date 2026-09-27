from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.database import init_db
from app.henrik_client import startup as henrik_startup, shutdown as henrik_shutdown
from app.routes.account import router as account_router
from app.routes.player import router as player_router
from app.routes.matches import router as matches_router
from app.routes.shop import router as shop_router
from app.routes.inventory import router as inventory_router
from app.routes.esports import router as esports_router


@asynccontextmanager
async def lifespan(app: FastAPI):
    await henrik_startup()
    init_db()
    yield
    await henrik_shutdown()


app = FastAPI(title="TPVal API", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(account_router)
app.include_router(player_router)
app.include_router(matches_router)
app.include_router(shop_router)
app.include_router(inventory_router)
app.include_router(esports_router)
