import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles

from config.logging_setup import setup_logging
from config.settings import settings
from database.connection import Base, SessionLocal, engine
from database.migration import run_migrations
from database.seed import seed_if_empty, update_streams
from routes import channels, custom, health, iptv, stream

setup_logging()
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    Base.metadata.create_all(bind=engine)
    run_migrations(engine)
    db = SessionLocal()
    try:
        inserted = seed_if_empty(db)
        if inserted:
            logger.info("Base de datos sembrada: %s canales insertados", inserted)
        updated = update_streams(db)
        if updated:
            logger.info("Streams actualizados: %s", updated)
    finally:
        db.close()
    yield


app = FastAPI(
    title=settings.PROJECT_NAME,
    description="API de canales de television peruana",
    version="1.0.0",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.CORS_ORIGINS,
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.mount("/static", StaticFiles(directory=settings.LOGOS_DIR.parent), name="static")
app.include_router(health.router, prefix=settings.API_PREFIX)
app.include_router(channels.router, prefix=settings.API_PREFIX)
app.include_router(iptv.router, prefix=settings.API_PREFIX)
app.include_router(custom.router, prefix=settings.API_PREFIX)
app.include_router(stream.router, prefix=settings.API_PREFIX)


@app.get("/", include_in_schema=False)
def root() -> dict[str, str]:
    return {
        "name": settings.PROJECT_NAME,
        "docs": "/docs",
        "health": f"{settings.API_PREFIX}/health",
        "channels": f"{settings.API_PREFIX}/channels",
    }
