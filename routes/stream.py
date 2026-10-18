from fastapi import APIRouter, Depends, HTTPException, Query, Response, status
from fastapi.responses import StreamingResponse
from sqlalchemy.orm import Session

from config.settings import settings
from database.connection import get_db
from models.channel import Channel
from models.custom import CustomChannel
from models.iptv import IptvChannel
from services.hls import (
    build_direct_playlist,
    get_playlist,
    proxy_asset,
    set_channel_headers,
    validate_proxy_url,
)

router = APIRouter(prefix="/stream", tags=["stream"])

M3U8_CONTENT_TYPE = "application/vnd.apple.mpegurl"


def _headers_from(user_agent: str | None, referrer: str | None) -> dict[str, str]:
    headers: dict[str, str] = {}
    if user_agent:
        headers["User-Agent"] = user_agent
    if referrer:
        headers["Referer"] = referrer
    return headers


def _resolve_stream(
    db: Session, channel_id: str
) -> tuple[str, bool, dict[str, str]]:
    channel = db.get(Channel, channel_id)
    if channel is not None:
        if not channel.stream_url:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"El canal '{channel_id}' no tiene senal asignada",
            )
        return channel.stream_url, False, {}

    iptv = db.get(IptvChannel, channel_id)
    if iptv is not None:
        if not iptv.stream_url:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"El canal '{channel_id}' no tiene senal asignada",
            )
        return iptv.stream_url, False, _headers_from(iptv.user_agent, iptv.referrer)

    custom = db.get(CustomChannel, channel_id)
    if custom is not None:
        return custom.url, custom.is_direct, _headers_from(
            custom.user_agent, custom.referrer
        )

    raise HTTPException(
        status_code=status.HTTP_404_NOT_FOUND,
        detail=f"Canal '{channel_id}' no encontrado",
    )


@router.get("/{channel_id}/playlist.m3u8")
def read_playlist(channel_id: str, db: Session = Depends(get_db)) -> Response:
    stream_url, is_direct, headers = _resolve_stream(db, channel_id)
    set_channel_headers(channel_id, headers)
    if is_direct:
        playlist = build_direct_playlist(channel_id, stream_url, settings.API_PREFIX)
    else:
        playlist = get_playlist(channel_id, stream_url, settings.API_PREFIX)
    return Response(content=playlist, media_type=M3U8_CONTENT_TYPE)


@router.get("/{channel_id}/variant")
def read_variant(
    channel_id: str,
    u: str = Query(..., description="URL absoluta de la variante"),
    db: Session = Depends(get_db),
) -> Response:
    _, _, headers = _resolve_stream(db, channel_id)
    set_channel_headers(channel_id, headers)
    validate_proxy_url(channel_id, u)
    playlist = get_playlist(channel_id, u, settings.API_PREFIX)
    return Response(content=playlist, media_type=M3U8_CONTENT_TYPE)


@router.get("/{channel_id}/asset")
def read_asset(
    channel_id: str,
    u: str = Query(..., description="URL absoluta del segmento"),
    db: Session = Depends(get_db),
) -> StreamingResponse:
    _, _, headers = _resolve_stream(db, channel_id)
    set_channel_headers(channel_id, headers)
    content_type, body = proxy_asset(channel_id, u)
    return StreamingResponse(
        body,
        media_type=content_type,
        headers={"Access-Control-Allow-Origin": "*"},
    )
