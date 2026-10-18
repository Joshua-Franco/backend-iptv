import time

from fastapi import HTTPException, status

from services.webfetch import BlockedUrlError, WebFetchError, fetch_bytes

_TTL = 6 * 3600.0
_MAX_ENTRIES = 2000
MAX_IMAGE_BYTES = 5 * 1024 * 1024

_cache: dict[str, tuple[float, bytes, str]] = {}


def _sniff_image_type(content_type: str, data: bytes) -> str:
    ctype = (content_type or "").split(";")[0].strip().lower()
    if ctype.startswith("image/"):
        return ctype
    if data[:4] == b"\x89PNG":
        return "image/png"
    if data[:3] == b"\xff\xd8\xff":
        return "image/jpeg"
    if data[:6] in (b"GIF87a", b"GIF89a"):
        return "image/gif"
    if b"<svg" in data[:1000].lower():
        return "image/svg+xml"
    if data[:4] == b"RIFF" and data[8:12] == b"WEBP":
        return "image/webp"
    return "image/png"


def get_image(url: str | None) -> tuple[bytes, str]:
    if not url:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Este canal no tiene imagen",
        )
    cached = _cache.get(url)
    if cached and time.monotonic() - cached[0] < _TTL:
        return cached[1], cached[2]
    try:
        data, content_type, _ = fetch_bytes(
            url, max_bytes=MAX_IMAGE_BYTES, timeout=10.0
        )
    except (BlockedUrlError, WebFetchError) as exc:
        raise HTTPException(
            status_code=status.HTTP_502_BAD_GATEWAY,
            detail=f"No se pudo obtener la imagen: {exc}",
        ) from exc
    image_type = _sniff_image_type(content_type, data)
    if len(_cache) >= _MAX_ENTRIES:
        oldest = min(_cache, key=lambda key: _cache[key][0])
        _cache.pop(oldest, None)
    _cache[url] = (time.monotonic(), data, image_type)
    return data, image_type
