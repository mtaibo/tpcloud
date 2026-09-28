from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.database import init_db
from app.henrik_client import startup as henrik_startup, shutdown as henrik_shutdown
from app.scheduler import start as scheduler_start, stop as scheduler_stop
from app.routes.account import router as account_router
from app.routes.extension import router as extension_router
from app.routes.player import router as player_router
from app.routes.matches import router as matches_router
from app.routes.shop import router as shop_router
from app.routes.shop_history import router as shop_history_router
from app.routes.night_market import router as night_market_router
from app.routes.wallet import router as wallet_router
from app.routes.wishlist import router as wishlist_router
from app.routes.loadout import router as loadout_router
from app.routes.battle_pass import router as battle_pass_router
from app.routes.inventory import router as inventory_router
from app.routes.esports import router as esports_router


@asynccontextmanager
async def lifespan(app: FastAPI):
    await henrik_startup()
    init_db()
    scheduler_start()
    yield
    scheduler_stop()
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
app.include_router(extension_router)
app.include_router(player_router)
app.include_router(matches_router)
app.include_router(shop_router)
app.include_router(shop_history_router)
app.include_router(night_market_router)
app.include_router(wallet_router)
app.include_router(wishlist_router)
app.include_router(loadout_router)
app.include_router(battle_pass_router)
app.include_router(inventory_router)
app.include_router(esports_router)
