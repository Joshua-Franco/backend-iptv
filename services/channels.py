from sqlalchemy import func, or_, select
from sqlalchemy.orm import Session

from config.settings import settings
from models.channel import Channel
from models.custom import CustomChannel
from models.iptv import IptvChannel
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
        source=None,
    )


def to_iptv_out(channel: IptvChannel) -> ChannelOut:
    logo = (
        f"{settings.API_PREFIX}/iptv/channels/{channel.id}/logo"
        if channel.logo_url
        else ""
    )
    return ChannelOut(
        id=channel.id,
        uuid=channel.uuid,
        number=channel.number,
        name=channel.name,
        category=channel.category,
        initials=channel.initials,
        accent=channel.accent,
        logo_url=logo,
        description=None,
        stream_url=channel.stream_url,
        website=channel.website,
        source="iptv",
    )


def to_custom_out(channel: CustomChannel) -> ChannelOut:
    logo = (
        f"{settings.API_PREFIX}/custom/channels/{channel.id}/logo"
        if channel.logo_url
        else ""
    )
    return ChannelOut(
        id=channel.id,
        uuid=channel.uuid,
        number=channel.number,
        name=channel.name,
        category=channel.category,
        initials=channel.initials,
        accent=channel.accent,
        logo_url=logo,
        description=channel.group_name,
        stream_url=channel.url,
        website=None,
        source="custom",
    )


def _filtered_custom_stmt(category: str | None, q: str | None):
    stmt = select(CustomChannel).order_by(CustomChannel.number)
    if category:
        stmt = stmt.where(
            func.lower(CustomChannel.category) == category.strip().lower()
        )
    if q:
        pattern = f"%{q.strip()}%"
        stmt = stmt.where(
            or_(CustomChannel.name.ilike(pattern), CustomChannel.category.ilike(pattern))
        )
    return stmt


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
    channels = [to_out(channel) for channel in db.scalars(stmt).all()]
    customs = [
        to_custom_out(channel)
        for channel in db.scalars(_filtered_custom_stmt(category, q)).all()
    ]
    return channels + customs


def get_channel(db: Session, channel_id: str) -> ChannelOut | None:
    channel = db.get(Channel, channel_id)
    if channel:
        return to_out(channel)
    iptv = db.get(IptvChannel, channel_id)
    if iptv:
        return to_iptv_out(iptv)
    custom = db.get(CustomChannel, channel_id)
    if custom:
        return to_custom_out(custom)
    return None


def get_channel_by_uuid(db: Session, channel_uuid: str) -> ChannelOut | None:
    channel = db.scalar(select(Channel).where(Channel.uuid == channel_uuid))
    if channel:
        return to_out(channel)
    iptv = db.scalar(select(IptvChannel).where(IptvChannel.uuid == channel_uuid))
    if iptv:
        return to_iptv_out(iptv)
    custom = db.scalar(select(CustomChannel).where(CustomChannel.uuid == channel_uuid))
    if custom:
        return to_custom_out(custom)
    return None


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
    merged: dict[str, int] = {name: count for name, count in db.execute(stmt)}
    custom_stmt = (
        select(CustomChannel.category, func.count(CustomChannel.id)).group_by(
            CustomChannel.category
        )
    )
    for name, count in db.execute(custom_stmt):
        merged[name] = merged.get(name, 0) + count
    ordered = sorted(merged.items(), key=lambda item: item[1], reverse=True)
    return [CategoryOut(name=name, count=count) for name, count in ordered]
