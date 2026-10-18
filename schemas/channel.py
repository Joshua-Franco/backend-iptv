from pydantic import BaseModel, ConfigDict, Field


class ChannelOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    uuid: str
    number: int = Field(..., description="Numero logico del canal")
    name: str
    category: str
    initials: str
    accent: str
    logo_url: str
    description: str | None = None
    stream_url: str | None = None
    website: str | None = None


class CategoryOut(BaseModel):
    name: str
    count: int


class StreamUpdate(BaseModel):
    stream_url: str | None = Field(
        default=None, description="URL HLS (.m3u8) del canal; null para sin senal"
    )


class HealthOut(BaseModel):
    status: str
    database: str
    channels: int
