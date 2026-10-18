from fastapi import APIRouter, Depends, HTTPException, Query, Response, status
from sqlalchemy import func, or_, select
from sqlalchemy.orm import Session

from database.connection import get_db
from models.iptv import IptvChannel, IptvCountry
from schemas.channel import ChannelOut
from schemas.iptv import ChannelPageOut, CountryOut
from services.channels import to_iptv_out
from services.images import get_image

router = APIRouter(tags=["iptv"])


@router.get("/countries", response_model=list[CountryOut])
def read_countries(db: Session = Depends(get_db)) -> list[CountryOut]:
    stmt = (
        select(IptvCountry, func.count(IptvChannel.id))
        .outerjoin(IptvChannel, IptvChannel.country == IptvCountry.code)
        .group_by(IptvCountry.code)
        .order_by(IptvCountry.name)
    )
    return [
        CountryOut(
            code=country.code,
            name=country.name,
            flag=country.flag,
            channel_count=count,
        )
        for country, count in db.execute(stmt)
    ]


@router.get("/countries/{code}/channels", response_model=ChannelPageOut)
def read_country_channels(
    code: str,
    limit: int = Query(default=60, ge=1, le=200),
    offset: int = Query(default=0, ge=0),
    db: Session = Depends(get_db),
) -> ChannelPageOut:
    code = code.upper()
    if not db.get(IptvCountry, code):
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Pais '{code}' no encontrado",
        )
    total = (
        db.scalar(
            select(func.count(IptvChannel.id)).where(IptvChannel.country == code)
        )
        or 0
    )
    rows = db.scalars(
        select(IptvChannel)
        .where(IptvChannel.country == code)
        .order_by(IptvChannel.name)
        .limit(limit)
        .offset(offset)
    ).all()
    return ChannelPageOut(
        items=[to_iptv_out(row) for row in rows],
        total=total,
        limit=limit,
        offset=offset,
    )


@router.get("/iptv/channels", response_model=list[ChannelOut])
def read_iptv_channels(
    ids: str = Query(..., description="CSV de ids o uuids de canales iptv"),
    db: Session = Depends(get_db),
) -> list[ChannelOut]:
    keys = [key.strip() for key in ids.split(",") if key.strip()][:200]
    if not keys:
        return []
    stmt = select(IptvChannel).where(
        or_(IptvChannel.id.in_(keys), IptvChannel.uuid.in_(keys))
    )
    return [to_iptv_out(row) for row in db.scalars(stmt).all()]


@router.get("/iptv/channels/{channel_id}/logo")
def read_iptv_logo(channel_id: str, db: Session = Depends(get_db)) -> Response:
    channel = db.get(IptvChannel, channel_id)
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
