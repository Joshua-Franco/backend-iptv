from datetime import datetime

from pydantic import BaseModel, ConfigDict, Field

from schemas.channel import ChannelOut


class CountryOut(BaseModel):
    code: str
    name: str
    flag: str
    channel_count: int = 0


class ChannelPageOut(BaseModel):
    items: list[ChannelOut]
    total: int
    limit: int
    offset: int


class CustomSourceIn(BaseModel):
    url: str = Field(
        ...,
        min_length=8,
        max_length=2000,
        description="Enlace de lista M3U (.m3u) o de canal (.m3u8)",
    )


class CustomSourceOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    url: str
    type: str
    status: str
    error: str | None = None
    channel_count: int
    created_at: datetime | None = None
    already: bool = False
