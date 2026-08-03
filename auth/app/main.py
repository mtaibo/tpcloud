from fastapi import FastAPI
from fastapi.responses import FileResponse
from database import init_db
from routes.passkey import router as passkey_router

app = FastAPI()
app.include_router(passkey_router)

def on_startup():
    init_db()

@app.get("/health")
def health():
    return {"status": "ok"}