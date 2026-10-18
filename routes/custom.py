from urllib.parse import urlparse

from fastapi import APIRouter, Depends, HTTPException, Response, status
from sqlalchemy import func, select
from sqlalchemy.orm import Session

from database.connection import get_db
from models.custom import CustomChannel, CustomSource
from schemas.iptv import CustomSourceIn, CustomSourceOut
from services.images import get_image
from services.m3u import DIRECT_EXTENSIONS, M3UParseError, prepare_source
from services.palette import accent_for, make_initials
from services.webfetch import WebFetchError

router = APIRouter(prefix="/custom", tags=["custom"])


def _is_direct(url: str) -> bool:
    path = urlparse(url).path.lower()
    return any(path.endswith(ext) for ext in DIRECT_EXTENSIONS)


def _to_out(source: CustomSource, already: bool = False) -> CustomSourceOut:
    return CustomSourceOut(
        id=source.id,
        url=source.url,
        type=source.type,
        status=source.status,
        error=source.error,
        channel_count=source.channel_count,
        created_at=source.created_at,
        already=already,
    )


@router.get("/sources", response_model=list[CustomSourceOut])
def read_sources(db: Session = Depends(get_db)) -> list[CustomSourceOut]:
    rows = db.scalars(
        select(CustomSource).order_by(CustomSource.created_at.desc(), CustomSource.id)
    ).all()
    return [_to_out(row) for row in rows]


@router.post("/sources", response_model=CustomSourceOut)
def add_source(
    payload: CustomSourceIn, db: Session = Depends(get_db)
) -> CustomSourceOut:
    url = payload.url.strip()
    existing = db.scalar(select(CustomSource).where(CustomSource.url == url))

    if existing and existing.status == "active":
        return _to_out(existing, already=True)

    try:
        kind, entries = prepare_source(url)
    except (M3UParseError, WebFetchError) as exc:
        if existing:
            existing.status = "error"
            existing.error = str(exc)
            existing.channel_count = 0
            db.commit()
            db.refresh(existing)
            return _to_out(existing)
        source = CustomSource(
            url=url,
            type="playlist",
            status="error",
            error=str(exc),
            channel_count=0,
        )
        db.add(source)
        db.commit()
        db.refresh(source)
        return _to_out(source)

    if existing:
        source = existing
        db.execute(
            CustomChannel.__table__.delete().where(
                CustomChannel.source_id == source.id
            )
        )
    else:
        source = CustomSource(url=url, type=kind, status="active")
        db.add(source)
        db.flush()

    next_number = (
        db.scalar(select(func.max(CustomChannel.number)).where(CustomChannel.number > 0))
        or 99
    ) + 1

    rows: list[CustomChannel] = []
    for entry in entries:
        rows.append(
            CustomChannel(
                source_id=source.id,
                name=entry["name"],
                url=entry["url"],
                logo_url=entry["logo"],
                group_name=entry["group"],
                category=entry["group"] or "Personalizado",
                number=next_number,
                initials=make_initials(entry["name"]),
                accent=accent_for(entry["url"]),
                is_direct=_is_direct(entry["url"]),
                user_agent=entry["user_agent"],
                referrer=entry["referrer"],
            )
        )
        next_number += 1

    db.add_all(rows)
    source.type = kind
    source.status = "active"
    source.error = None
    source.channel_count = len(rows)
    db.commit()
    db.refresh(source)
    return _to_out(source)


@router.delete("/sources/{source_id}", status_code=status.HTTP_204_NO_CONTENT)
def remove_source(source_id: str, db: Session = Depends(get_db)) -> Response:
    source = db.get(CustomSource, source_id)
    if not source:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Enlace no encontrado",
        )
    db.delete(source)
    db.commit()
    return Response(status_code=status.HTTP_204_NO_CONTENT)


@router.get("/channels/{channel_id}/logo")
def read_custom_logo(channel_id: str, db: Session = Depends(get_db)) -> Response:
    channel = db.get(CustomChannel, channel_id)
    if not channel:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Canal '{channel_id}' no encontrado",
        )
    data, image_type = get_image(channel.logo_url)
    return Response(
        content=data,
        media_type=image_type,
        headers={
            "Cache-Control": "public, max-age=86400",
            "Access-Control-Allow-Origin": "*",
        },
    )
