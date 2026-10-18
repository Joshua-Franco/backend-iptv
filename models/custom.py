from datetime import datetime

from sqlalchemy import Boolean, DateTime, ForeignKey, Integer, String, Text, func
from sqlalchemy.orm import Mapped, mapped_column

from database.connection import Base
from models.channel import new_uuid


class CustomSource(Base):
    __tablename__ = "custom_sources"

    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=new_uuid)
    url: Mapped[str] = mapped_column(Text, nullable=False, unique=True)
    type: Mapped[str] = mapped_column(String(16), nullable=False, default="playlist")
    status: Mapped[str] = mapped_column(String(16), nullable=False, default="active")
    error: Mapped[str | None] = mapped_column(Text, nullable=True)
    channel_count: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )


class CustomChannel(Base):
    __tablename__ = "custom_channels"

    id: Mapped[str] = mapped_column(
        String(64), primary_key=True, default=new_uuid
    )
    uuid: Mapped[str] = mapped_column(
        String(36), unique=True, nullable=False, index=True, default=new_uuid
    )
    source_id: Mapped[str] = mapped_column(
        String(36),
        ForeignKey("custom_sources.id", ondelete="CASCADE"),
        nullable=False,
        index=True,
    )
    name: Mapped[str] = mapped_column(String(255), nullable=False)
    url: Mapped[str] = mapped_column(Text, nullable=False)
    logo_url: Mapped[str | None] = mapped_column(Text, nullable=True)
    group_name: Mapped[str | None] = mapped_column(String(160), nullable=True)
    category: Mapped[str] = mapped_column(
        String(160), nullable=False, default="Personalizado", index=True
    )
    number: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
    initials: Mapped[str] = mapped_column(String(8), nullable=False, default="TV")
    accent: Mapped[str] = mapped_column(String(9), nullable=False, default="#7c5cff")
    is_direct: Mapped[bool] = mapped_column(Boolean, nullable=False, default=False)
    user_agent: Mapped[str | None] = mapped_column(String(500), nullable=True)
    referrer: Mapped[str | None] = mapped_column(String(500), nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now()
    )
