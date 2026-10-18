from sqlalchemy import func, or_, select
from sqlalchemy.orm import Session

from models.channel import Channel
from schemas.channel import CategoryOut, ChannelOut


def to_out(channel: Channel) -> ChannelOut:
    return ChannelOut(
        id=channel.id,
        uuid=channel.uuid,
        number=channel.number,
        name=channel.name,
        category=channel.category,
        initials=channel.initials,
        accent=channel.accent,
        logo_url=f"/static/logos/{channel.logo_file}",
        description=channel.description,
        stream_url=channel.stream_url,
        website=channel.website,
    )


def list_channels(
    db: Session, category: str | None = None, q: str | None = None
) -> list[ChannelOut]:
    stmt = select(Channel).order_by(Channel.number)
    if category:
        stmt = stmt.where(func.lower(Channel.category) == category.strip().lower())
    if q:
        pattern = f"%{q.strip()}%"
        stmt = stmt.where(
            or_(
                Channel.name.ilike(pattern),
                Channel.category.ilike(pattern),
                Channel.description.ilike(pattern),
            )
        )
    return [to_out(channel) for channel in db.scalars(stmt).all()]


def get_channel(db: Session, channel_id: str) -> ChannelOut | None:
    channel = db.get(Channel, channel_id)
    return to_out(channel) if channel else None


def get_channel_by_uuid(db: Session, channel_uuid: str) -> ChannelOut | None:
    channel = db.scalar(select(Channel).where(Channel.uuid == channel_uuid))
    return to_out(channel) if channel else None


def update_stream(db: Session, channel_id: str, stream_url: str | None) -> ChannelOut | None:
    channel = db.get(Channel, channel_id)
    if not channel:
        return None
    channel.stream_url = stream_url or None
    db.commit()
    db.refresh(channel)
    return to_out(channel)


def list_categories(db: Session) -> list[CategoryOut]:
    stmt = (
        select(Channel.category, func.count(Channel.id))
        .group_by(Channel.category)
        .order_by(func.count(Channel.id).desc())
    )
    return [CategoryOut(name=name, count=count) for name, count in db.execute(stmt)]
