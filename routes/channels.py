from fastapi import APIRouter, Depends, HTTPException, Query, status
from sqlalchemy.orm import Session

from database.connection import get_db
from schemas.channel import CategoryOut, ChannelOut, StreamUpdate
from services.channels import (
    get_channel,
    get_channel_by_uuid,
    list_categories,
    list_channels,
    update_stream,
)

router = APIRouter(prefix="/channels", tags=["channels"])


@router.get("", response_model=list[ChannelOut])
def read_channels(
    category: str | None = Query(default=None, description="Filtrar por categoria"),
    q: str | None = Query(default=None, description="Buscar por nombre o categoria"),
    db: Session = Depends(get_db),
) -> list[ChannelOut]:
    return list_channels(db, category=category, q=q)


@router.get("/categories", response_model=list[CategoryOut])
def read_categories(db: Session = Depends(get_db)) -> list[CategoryOut]:
    return list_categories(db)


@router.get("/by-uuid/{channel_uuid}", response_model=ChannelOut)
def read_channel_by_uuid(
    channel_uuid: str, db: Session = Depends(get_db)
) -> ChannelOut:
    channel = get_channel_by_uuid(db, channel_uuid)
    if not channel:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Canal con uuid '{channel_uuid}' no encontrado",
        )
    return channel


@router.get("/{channel_id}", response_model=ChannelOut)
def read_channel(channel_id: str, db: Session = Depends(get_db)) -> ChannelOut:
    channel = get_channel(db, channel_id)
    if not channel:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Canal '{channel_id}' no encontrado",
        )
    return channel


@router.patch("/{channel_id}/stream", response_model=ChannelOut)
def set_channel_stream(
    channel_id: str, payload: StreamUpdate, db: Session = Depends(get_db)
) -> ChannelOut:
    channel = update_stream(db, channel_id, payload.stream_url)
    if not channel:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Canal '{channel_id}' no encontrado",
        )
    return channel
