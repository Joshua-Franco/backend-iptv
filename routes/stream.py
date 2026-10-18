from fastapi import APIRouter, Depends, HTTPException, Query, Response, status
from fastapi.responses import StreamingResponse
from sqlalchemy.orm import Session

from config.settings import settings
from database.connection import get_db
from models.channel import Channel
from services.hls import get_playlist, proxy_asset, validate_proxy_url

router = APIRouter(prefix="/stream", tags=["stream"])

M3U8_CONTENT_TYPE = "application/vnd.apple.mpegurl"


def _get_channel_stream(db: Session, channel_id: str) -> tuple[Channel, str]:
    channel = db.get(Channel, channel_id)
    if not channel:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Canal '{channel_id}' no encontrado",
        )
    if not channel.stream_url:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"El canal '{channel_id}' no tiene senal asignada",
        )
    return channel, channel.stream_url


@router.get("/{channel_id}/playlist.m3u8")
def read_playlist(channel_id: str, db: Session = Depends(get_db)) -> Response:
    _, stream_url = _get_channel_stream(db, channel_id)
    playlist = get_playlist(channel_id, stream_url, settings.API_PREFIX)
    return Response(content=playlist, media_type=M3U8_CONTENT_TYPE)


@router.get("/{channel_id}/variant")
def read_variant(
    channel_id: str,
    u: str = Query(..., description="URL absoluta de la variante"),
    db: Session = Depends(get_db),
) -> Response:
    _get_channel_stream(db, channel_id)
    validate_proxy_url(channel_id, u)
    playlist = get_playlist(channel_id, u, settings.API_PREFIX)
    return Response(content=playlist, media_type=M3U8_CONTENT_TYPE)


@router.get("/{channel_id}/asset")
def read_asset(
    channel_id: str,
    u: str = Query(..., description="URL absoluta del segmento"),
    db: Session = Depends(get_db),
) -> StreamingResponse:
    _get_channel_stream(db, channel_id)
    content_type, body = proxy_asset(channel_id, u)
    return StreamingResponse(
        body,
        media_type=content_type,
        headers={"Access-Control-Allow-Origin": "*"},
    )
