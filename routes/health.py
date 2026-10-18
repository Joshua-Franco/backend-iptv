from fastapi import APIRouter, Depends
from sqlalchemy import func, select
from sqlalchemy.orm import Session

from database.connection import get_db
from models.channel import Channel
from models.custom import CustomChannel
from models.iptv import IptvChannel
from schemas.channel import HealthOut

router = APIRouter(tags=["health"])


@router.get("/health", response_model=HealthOut)
def health(db: Session = Depends(get_db)) -> HealthOut:
    channels = db.scalar(select(func.count()).select_from(Channel)) or 0
    iptv = db.scalar(select(func.count()).select_from(IptvChannel)) or 0
    custom = db.scalar(select(func.count()).select_from(CustomChannel)) or 0
    return HealthOut(
        status="ok", database="connected", channels=channels, iptv=iptv, custom=custom
    )
