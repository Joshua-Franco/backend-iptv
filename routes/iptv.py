from collections import Counter

from fastapi import APIRouter, Depends, HTTPException, Query, Response, status
from sqlalchemy import func, or_, select
from sqlalchemy.orm import Session

from database.connection import get_db
from models.iptv import IptvCategory, IptvChannel
from schemas.channel import ChannelOut
from schemas.iptv import ChannelPageOut, IptvCategoryOut
from services.channels import to_iptv_out
from services.images import get_image

router = APIRouter(tags=["iptv"])


@router.get("/iptv/categories", response_model=list[IptvCategoryOut])
def read_iptv_categories(db: Session = Depends(get_db)) -> list[IptvCategoryOut]:
    tag_counts: Counter[str] = Counter()
    for csv in db.scalars(select(IptvChannel.categories)):
        if not csv:
            continue
        for tag in {part.strip() for part in csv.split(",")}:
            if tag:
                tag_counts[tag] += 1
    items = [
        IptvCategoryOut(
            code=row.code,
            name=row.name_es,
            name_en=row.name_en,
            count=tag_counts.get(row.code, 0),
        )
        for row in db.scalars(select(IptvCategory)).all()
    ]
    items.sort(key=lambda item: (-item.count, item.name))
    return items


@router.get("/iptv/categories/{code}/channels", response_model=ChannelPageOut)
def read_iptv_category_channels(
    code: str,
    limit: int = Query(default=60, ge=1, le=200),
    offset: int = Query(default=0, ge=0),
    db: Session = Depends(get_db),
) -> ChannelPageOut:
    category = db.get(IptvCategory, code)
    if not category:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Categoria '{code}' no encontrada",
        )
    where = IptvChannel.categories.ilike(f"%{code}%")
    total = db.scalar(select(func.count(IptvChannel.id)).where(where)) or 0
    rows = db.scalars(
        select(IptvChannel)
        .where(where)
        .order_by(IptvChannel.name, IptvChannel.id)
        .limit(limit)
        .offset(offset)
    ).all()
    return ChannelPageOut(
        items=[to_iptv_out(row) for row in rows],
        total=total,
        limit=limit,
        offset=offset,
    )


@router.get("/iptv/catalog", response_model=ChannelPageOut)
def read_iptv_catalog(
    limit: int = Query(default=60, ge=1, le=200),
    offset: int = Query(default=0, ge=0),
    q: str | None = Query(default=None, max_length=100),
    db: Session = Depends(get_db),
) -> ChannelPageOut:
    stmt = select(IptvChannel)
    total_stmt = select(func.count(IptvChannel.id))
    if q and q.strip():
        pattern = f"%{q.strip()}%"
        condition = or_(
            IptvChannel.name.ilike(pattern),
            IptvChannel.category.ilike(pattern),
        )
        stmt = stmt.where(condition)
        total_stmt = total_stmt.where(condition)
    total = db.scalar(total_stmt) or 0
    rows = db.scalars(
        stmt.order_by(IptvChannel.name, IptvChannel.id).limit(limit).offset(offset)
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
