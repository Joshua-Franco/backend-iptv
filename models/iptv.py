from datetime import datetime

from sqlalchemy import (
    Boolean,
    DateTime,
    ForeignKey,
    Integer,
    String,
    Text,
    func,
)
from sqlalchemy.orm import Mapped, mapped_column

from database.connection import Base
from models.channel import new_uuid


class IptvCountry(Base):
    __tablename__ = "iptv_countries"

    code: Mapped[str] = mapped_column(String(8), primary_key=True)
    name: Mapped[str] = mapped_column(String(120), nullable=False, index=True)
    flag: Mapped[str] = mapped_column(String(16), nullable=False, default="")
    languages: Mapped[str] = mapped_column(Text, nullable=False, default="")


class IptvCategory(Base):
    __tablename__ = "iptv_categories"

    code: Mapped[str] = mapped_column(String(40), primary_key=True)
    name_en: Mapped[str] = mapped_column(String(60), nullable=False)
    name_es: Mapped[str] = mapped_column(String(60), nullable=False)


class IptvChannel(Base):
    __tablename__ = "iptv_channels"

    id: Mapped[str] = mapped_column(String(255), primary_key=True)
    uuid: Mapped[str] = mapped_column(
        String(36), unique=True, nullable=False, index=True, default=new_uuid
    )
    name: Mapped[str] = mapped_column(String(255), nullable=False, index=True)
    country: Mapped[str] = mapped_column(
        String(8), ForeignKey("iptv_countries.code"), nullable=False, index=True
    )
    category: Mapped[str] = mapped_column(
        String(60), nullable=False, default="Generalista", index=True
    )
    categories: Mapped[str] = mapped_column(Text, nullable=False, default="")
    website: Mapped[str | None] = mapped_column(String(500), nullable=True)
    is_nsfw: Mapped[bool] = mapped_column(Boolean, nullable=False, default=False)
    accent: Mapped[str] = mapped_column(String(9), nullable=False, default="#38a3d1")
    initials: Mapped[str] = mapped_column(String(8), nullable=False, default="TV")
    number: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
    stream_url: Mapped[str | None] = mapped_column(Text, nullable=True)
    user_agent: Mapped[str | None] = mapped_column(String(500), nullable=True)
    referrer: Mapped[str | None] = mapped_column(String(500), nullable=True)
    logo_url: Mapped[str | None] = mapped_column(Text, nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )

    def __repr__(self) -> str:
        return f"<IptvChannel {self.country}:{self.name}>"


class IptvStream(Base):
    __tablename__ = "iptv_streams"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    channel_id: Mapped[str] = mapped_column(
        String(255),
        ForeignKey("iptv_channels.id", ondelete="CASCADE"),
        nullable=False,
        index=True,
    )
    feed: Mapped[str | None] = mapped_column(String(120), nullable=True)
    title: Mapped[str] = mapped_column(String(255), nullable=False, default="")
    url: Mapped[str] = mapped_column(Text, nullable=False)
    quality: Mapped[str | None] = mapped_column(String(20), nullable=True)
    labels: Mapped[str] = mapped_column(Text, nullable=False, default="")
    user_agent: Mapped[str | None] = mapped_column(String(500), nullable=True)
    referrer: Mapped[str | None] = mapped_column(String(500), nullable=True)


class IptvLogo(Base):
    __tablename__ = "iptv_logos"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    channel_id: Mapped[str] = mapped_column(
        String(255),
        ForeignKey("iptv_channels.id", ondelete="CASCADE"),
        nullable=False,
        index=True,
    )
    feed: Mapped[str | None] = mapped_column(String(120), nullable=True)
    url: Mapped[str] = mapped_column(Text, nullable=False)
    format: Mapped[str | None] = mapped_column(String(16), nullable=True)
    width: Mapped[int | None] = mapped_column(Integer, nullable=True)
    height: Mapped[int | None] = mapped_column(Integer, nullable=True)
    in_use: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)
    is_primary: Mapped[bool] = mapped_column(Boolean, nullable=False, default=False)
