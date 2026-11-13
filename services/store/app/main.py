import asyncio
import os
from contextlib import asynccontextmanager
from pathlib import Path

from fastapi import FastAPI

from app import media
from app.database import init_db
from app.routes.files import keep_pending_uploads_clean, router as files_router
from app.routes.admin import router as admin_router
from app.routes.shares import router as shares_router
from app.routes.share_access import router as share_access_router
from app.routes.open import router as open_router
from app.routes.compression import router as compression_router
from app.routes.backup import router as backup_router
from app.routes.photos import router as photos_router
from app.routes.albums import router as albums_router


@asynccontextmanager
async def lifespan(app: FastAPI):
    try:
        from pillow_heif import register_heif_opener
        register_heif_opener()
    except Exception:
        pass

    external_base = Path(os.getenv("EXTERNAL_DISK_PATH", "/mnt/external"))
    try:
        (external_base / "shared").mkdir(parents=True, exist_ok=True)
        (external_base / "users").mkdir(parents=True, exist_ok=True)
    except Exception:
        pass
    init_db()
    # The gallery's thumbnails are made ahead of time in the background (see media.py).
    thumbnails = asyncio.create_task(media.keep_photo_thumbnails())
    # Uploads that will never finish (late duplicate chunks, abandoned ones) are cleared out.
    pending_cleanup = asyncio.create_task(keep_pending_uploads_clean())
    yield
    thumbnails.cancel()
    pending_cleanup.cancel()


app = FastAPI(title="TPStore API", lifespan=lifespan, docs_url=None, redoc_url=None, openapi_url=None)

app.include_router(files_router)
app.include_router(admin_router)
app.include_router(shares_router)
app.include_router(share_access_router)
app.include_router(open_router)
app.include_router(compression_router)
app.include_router(backup_router)
app.include_router(photos_router)
app.include_router(albums_router)
