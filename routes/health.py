from fastapi import APIRouter, Depends
from sqlalchemy import func, select
from sqlalchemy.orm import Session

from database.connection import get_db
from models.channel import Channel
from schemas.channel import HealthOut

router = APIRouter(tags=["health"])


@router.get("/health", response_model=HealthOut)
def health(db: Session = Depends(get_db)) -> HealthOut:
    count = db.scalar(select(func.count()).select_from(Channel)) or 0
    return HealthOut(status="ok", database="connected", channels=count)
