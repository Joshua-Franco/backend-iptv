from collections.abc import Generator

from sqlalchemy import create_engine
from sqlalchemy.orm import DeclarativeBase, Session, sessionmaker

from config.settings import settings

if not settings.DATABASE_URL:
    raise RuntimeError("DATABASE_URL no esta definida en el archivo .env")


def _normalize_db_url(url: str) -> str:
    if url.startswith("postgres://"):
        url = "postgresql://" + url.removeprefix("postgres://")
    if url.startswith("postgresql://"):
        return "postgresql+psycopg://" + url.removeprefix("postgresql://")
    return url


engine = create_engine(_normalize_db_url(settings.DATABASE_URL), pool_pre_ping=True)
SessionLocal = sessionmaker(bind=engine, autoflush=False, expire_on_commit=False)


class Base(DeclarativeBase):
    pass


def get_db() -> Generator[Session, None, None]:
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()
