import re
from urllib.parse import urljoin, urlparse

from services.webfetch import BlockedUrlError, WebFetchError, fetch_text

MAX_PLAYLIST_ENTRIES = 10_000
DIRECT_EXTENSIONS = (".mp4", ".ts", ".mkv", ".webm", ".mov", ".avi", ".flv")
HLS_EXTENSIONS = (".m3u8", ".m3u")

_EXTINF_RE = re.compile(r"^#EXTINF:\s*-?[\d.]*\s*(?P<attrs>.*?),\s*(?P<name>.*)$", re.I)
_ATTR_RE = re.compile(r'([A-Za-z0-9_-]+)="([^"]*)"')
_VLC_USER_AGENT_RE = re.compile(r"^#EXTVLCOPT:\s*http-user-agent\s*=\s*(.+)$", re.I)
_VLC_REFERRER_RE = re.compile(r"^#EXTVLCOPT:\s*http-referrer\s*=\s*(.+)$", re.I)


class M3UParseError(Exception):
    """El enlace no es una lista M3U ni un canal reproducible."""


def _path_ext(url: str) -> str:
    path = urlparse(url).path.lower()
    for ext in (*DIRECT_EXTENSIONS, *HLS_EXTENSIONS):
        if path.endswith(ext):
            return ext
    return ""


def _entry_name(url: str) -> str:
    path = urlparse(url).path.rstrip("/")
    name = path.rsplit("/", 1)[-1] if path else ""
    for ext in (".m3u8", ".m3u", ".mp4", ".ts"):
        if name.lower().endswith(ext):
            name = name[: -len(ext)]
    return name or urlparse(url).hostname or "Canal"


def parse_entries(text: str, base_url: str) -> list[dict]:
    entries: list[dict] = []
    attrs: dict[str, str] = {}
    display_name = ""
    group_name: str | None = None
    user_agent: str | None = None
    referrer: str | None = None

    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        if line.startswith("#EXTINF:"):
            match = _EXTINF_RE.match(line)
            attrs = {}
            display_name = ""
            if match:
                attrs = dict(_ATTR_RE.findall(match.group("attrs")))
                display_name = (match.group("name") or "").strip()
            group_name = attrs.get("group-title") or None
            user_agent = None
            referrer = None
            continue
        if line.upper().startswith("#EXTVLCOPT:"):
            agent_match = _VLC_USER_AGENT_RE.match(line)
            if agent_match:
                user_agent = agent_match.group(1).strip() or None
            ref_match = _VLC_REFERRER_RE.match(line)
            if ref_match:
                referrer = ref_match.group(1).strip() or None
            continue
        if line.upper().startswith("#EXTGRP:"):
            group_name = line.split(":", 1)[1].strip() or group_name
            continue
        if line.startswith("#"):
            continue

        url = urljoin(base_url, line)
        name = display_name or attrs.get("tvg-name") or _entry_name(url)
        entries.append(
            {
                "name": name[:255],
                "url": url,
                "logo": attrs.get("logo") or attrs.get("tvg-logo") or None,
                "group": (group_name or None) and group_name[:160],
                "user_agent": user_agent,
                "referrer": referrer,
            }
        )
        display_name = ""
        attrs = {}
        group_name = None
        if len(entries) >= MAX_PLAYLIST_ENTRIES:
            break
    return entries


def prepare_source(url: str) -> tuple[str, list[dict]]:
    """Descarga el enlace y detecta si es lista M3U o un canal suelto.

    Retorna (type, entries) con type en {"playlist", "stream"}.
    Lanza M3UParseError/WebFetchError con el mensaje para el usuario.
    """
    url = url.strip()
    if not url:
        raise M3UParseError("Pega un enlace M3U o .m3u8")
    if not url.lower().startswith(("http://", "https://")):
        raise M3UParseError("El enlace debe empezar con http:// o https://")

    ext = _path_ext(url)
    try:
        text, content_type, final_url = fetch_text(url)
    except BlockedUrlError as exc:
        raise M3UParseError(str(exc)) from exc
    except WebFetchError as exc:
        if ext == ".m3u8" or ext in DIRECT_EXTENSIONS:
            return "stream", [
                {
                    "name": _entry_name(url),
                    "url": url,
                    "logo": None,
                    "group": None,
                    "user_agent": None,
                    "referrer": None,
                }
            ]
        raise M3UParseError(str(exc)) from exc

    is_hls_tags = "#EXT-X-" in text[:200_000]
    starts_m3u = text.lstrip("\ufeff \t\r\n").upper().startswith("#EXTM3U")

    if is_hls_tags:
        return "stream", [
            {
                "name": _entry_name(final_url),
                "url": final_url,
                "logo": None,
                "group": None,
                "user_agent": None,
                "referrer": None,
            }
        ]

    if starts_m3u or "#EXTINF" in text[:200_000]:
        entries = parse_entries(text, final_url)
        if entries:
            return "playlist", entries
        if ext in HLS_EXTENSIONS:
            return "stream", [
                {
                    "name": _entry_name(final_url),
                    "url": final_url,
                    "logo": None,
                    "group": None,
                    "user_agent": None,
                    "referrer": None,
                }
            ]
        raise M3UParseError("La lista M3U esta vacia")

    if ext in HLS_EXTENSIONS:
        return "stream", [
            {
                "name": _entry_name(final_url),
                "url": final_url,
                "logo": None,
                "group": None,
                "user_agent": None,
                "referrer": None,
            }
        ]
    if ext in DIRECT_EXTENSIONS:
        return "stream", [
            {
                "name": _entry_name(final_url),
                "url": final_url,
                "logo": None,
                "group": None,
                "user_agent": None,
                "referrer": None,
            }
        ]

    ctype = (content_type or "").lower()
    if "mpegurl" in ctype:
        return "stream", [
            {
                "name": _entry_name(final_url),
                "url": final_url,
                "logo": None,
                "group": None,
                "user_agent": None,
                "referrer": None,
            }
        ]
    raise M3UParseError(
        "Formato no reconocido: solo se aceptan listas M3U y enlaces .m3u8"
    )
