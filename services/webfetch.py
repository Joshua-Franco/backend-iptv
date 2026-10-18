import ipaddress
import socket
from urllib.parse import urljoin, urlparse

import httpx

from services.hls import BROWSER_HEADERS

MAX_REDIRECTS = 5
DEFAULT_MAX_BYTES = 10 * 1024 * 1024


class WebFetchError(Exception):
    """Error al obtener un recurso remoto de forma segura."""


class BlockedUrlError(WebFetchError):
    """El enlace apunta a una direccion no permitida (SSRF)."""


def assert_public_url(url: str) -> None:
    parsed = urlparse(url)
    if parsed.scheme not in {"http", "https"}:
        raise BlockedUrlError("Solo se aceptan enlaces http/https")
    host = parsed.hostname
    if not host:
        raise BlockedUrlError("El enlace no tiene un host valido")
    port = parsed.port or (443 if parsed.scheme == "https" else 80)
    try:
        infos = socket.getaddrinfo(host, port, proto=socket.IPPROTO_TCP)
    except socket.gaierror as exc:
        raise WebFetchError("No se pudo resolver el dominio del enlace") from exc
    for info in infos:
        ip = ipaddress.ip_address(info[4][0])
        if not ip.is_global:
            raise BlockedUrlError(
                "El enlace apunta a una red interna y fue bloqueada"
            )


def fetch_bytes(
    url: str, max_bytes: int = DEFAULT_MAX_BYTES, timeout: float = 15.0
) -> tuple[bytes, str, str]:
    """Descarga un recurso validando cada salto contra SSRF.

    Retorna (contenido, content_type, url_final).
    """
    current = url
    with httpx.Client(
        headers=BROWSER_HEADERS, timeout=timeout, follow_redirects=False
    ) as client:
        response: httpx.Response | None = None
        for _ in range(MAX_REDIRECTS + 1):
            assert_public_url(current)
            try:
                response = client.get(current)
            except httpx.HTTPError as exc:
                raise WebFetchError(
                    f"No se pudo conectar con el enlace ({exc.__class__.__name__})"
                ) from exc
            if response.status_code in {301, 302, 303, 307, 308}:
                location = response.headers.get("location")
                if not location:
                    raise WebFetchError("El enlace redirige sin destino")
                current = urljoin(current, location)
                continue
            break
        if response is None or response.status_code >= 400:
            code = response.status_code if response is not None else "?"
            raise WebFetchError(f"El enlace respondio {code}")
        content_type = response.headers.get("content-type", "")
        chunks: list[bytes] = []
        total = 0
        try:
            for chunk in response.iter_bytes():
                total += len(chunk)
                if total > max_bytes:
                    raise WebFetchError("El enlace trae demasiados datos")
                chunks.append(chunk)
        except httpx.HTTPError as exc:
            raise WebFetchError(
                f"Error leyendo la respuesta ({exc.__class__.__name__})"
            ) from exc
        return b"".join(chunks), content_type, str(response.url)


def decode_text(raw: bytes) -> str:
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError:
        return raw.decode("latin-1", errors="replace")


def fetch_text(
    url: str, max_bytes: int = DEFAULT_MAX_BYTES, timeout: float = 15.0
) -> tuple[str, str, str]:
    raw, content_type, final_url = fetch_bytes(url, max_bytes=max_bytes, timeout=timeout)
    return decode_text(raw), content_type, final_url
