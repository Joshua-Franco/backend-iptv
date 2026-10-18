import time
from collections.abc import Iterator
from urllib.parse import quote, urljoin, urlparse

import httpx
from fastapi import HTTPException, status

BROWSER_HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (SMART-TV; Linux; Tizen 6.5) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36"
    ),
    "Accept": "*/*",
}

_cache: dict[str, tuple[float, str, str]] = {}
_CACHE_TTL = 3.0

_allowed_hosts: dict[str, set[str]] = {}


def _register_host(channel_id: str, url: str) -> None:
    host = urlparse(url).hostname
    if host:
        _allowed_hosts.setdefault(channel_id, set()).add(host)


def _is_allowed(channel_id: str, url: str) -> bool:
    host = urlparse(url).hostname
    return bool(host) and host in _allowed_hosts.get(channel_id, set())


def _rewrite(playlist: str, base_url: str, channel_id: str, prefix: str) -> str:
    lines = []
    for raw in playlist.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            lines.append(raw)
            continue
        absolute = urljoin(base_url, line)
        _register_host(channel_id, absolute)
        if ".m3u8" in absolute.lower():
            target = f"{prefix}/stream/{channel_id}/variant?u={quote(absolute, safe='')}"
        else:
            target = f"{prefix}/stream/{channel_id}/asset?u={quote(absolute, safe='')}"
        lines.append(target)
    return "\n".join(lines)


def _fetch_text(url: str) -> tuple[str, str]:
    cached = _cache.get(url)
    if cached and time.monotonic() - cached[0] < _CACHE_TTL:
        return cached[1], cached[2]
    try:
        with httpx.Client(headers=BROWSER_HEADERS, timeout=10.0, follow_redirects=True) as client:
            response = client.get(url)
    except httpx.HTTPError as exc:
        raise HTTPException(
            status_code=status.HTTP_502_BAD_GATEWAY,
            detail=f"No se pudo obtener la senal: {exc.__class__.__name__}",
        ) from exc
    if response.status_code >= 400:
        raise HTTPException(
            status_code=status.HTTP_502_BAD_GATEWAY,
            detail=f"El origen respondio {response.status_code}",
        )
    text = response.text
    content_type = response.headers.get("content-type", "application/vnd.apple.mpegurl")
    _cache[url] = (time.monotonic(), text, content_type)
    return text, content_type


def get_playlist(channel_id: str, url: str, prefix: str) -> str:
    _register_host(channel_id, url)
    text, _ = _fetch_text(url)
    return _rewrite(text, url, channel_id, prefix)


def proxy_asset(channel_id: str, url: str) -> tuple[str, Iterator[bytes]]:
    if not _is_allowed(channel_id, url):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Host del recurso no permitido",
        )
    client = httpx.Client(headers=BROWSER_HEADERS, timeout=15.0, follow_redirects=True)
    try:
        request = client.build_request("GET", url)
        response = client.send(request, stream=True)
    except httpx.HTTPError as exc:
        client.close()
        raise HTTPException(
            status_code=status.HTTP_502_BAD_GATEWAY,
            detail="No se pudo obtener el recurso",
        ) from exc
    if response.status_code >= 400:
        response.close()
        client.close()
        raise HTTPException(
            status_code=status.HTTP_502_BAD_GATEWAY,
            detail=f"El origen respondio {response.status_code}",
        )

    content_type = response.headers.get("content-type", "application/octet-stream")

    def stream() -> Iterator[bytes]:
        try:
            yield from response.iter_bytes()
        finally:
            response.close()
            client.close()

    return content_type, stream()


def validate_proxy_url(channel_id: str, url: str) -> None:
    if not _is_allowed(channel_id, url):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Host del recurso no permitido",
        )
