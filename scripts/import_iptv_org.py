"""Importa el catalogo completo de iptv-org a PostgreSQL (Neon).

Uso (desde la raiz del backend, con el .env configurado):

    python -m scripts.import_iptv_org            # importa solo si la tabla esta vacia
    python -m scripts.import_iptv_org --refresh  # recarga limpia del catalogo iptv-org

Los JSON de iptv-org se descargan en memoria, se insertan y se descartan:
el backend en produccion solo lee desde la base de datos.
Las tablas `channels` (canales peruanos) y `custom_*` no se tocan.
"""
import argparse
import json
import logging
import sys

import httpx
from sqlalchemy import delete, func, select

from database.connection import Base, SessionLocal, engine
from models.iptv import IptvCategory, IptvChannel, IptvLogo, IptvStream
from services.palette import accent_for, make_initials

API = "https://iptv-org.github.io/api"
TIMEOUT = httpx.Timeout(60.0)

CATEGORIES_ES: dict[str, str] = {
    "auto": "Automoviles",
    "animation": "Animacion",
    "business": "Negocios",
    "classic": "Clasicos",
    "comedy": "Comedia",
    "cooking": "Cocina",
    "culture": "Cultura",
    "documentary": "Documentales",
    "education": "Educacion",
    "entertainment": "Entretenimiento",
    "family": "Familia",
    "general": "Generalista",
    "interactive": "Interactivo",
    "kids": "Infantil",
    "legislative": "Legislativo",
    "lifestyle": "Estilo de vida",
    "movies": "Cine",
    "music": "Musica",
    "news": "Noticias",
    "outdoor": "Aire libre",
    "public": "Publico",
    "relax": "Relax",
    "religious": "Religioso",
    "series": "Series",
    "science": "Ciencia",
    "shop": "Compras",
    "sports": "Deportes",
    "travel": "Viajes",
    "weather": "Clima",
    "xxx": "Adultos",
}

log = logging.getLogger("import_iptv_org")


def download(name: str):
    url = f"{API}/{name}"
    log.info("Descargando %s ...", url)
    with httpx.Client(timeout=TIMEOUT, follow_redirects=True) as client:
        response = client.get(url)
        response.raise_for_status()
        return response.json()


def _csv(values, limit: int | None = None) -> str:
    text = ",".join(str(value) for value in values or [])
    return text[:limit] if limit else text


def _truncate(value: str | None, limit: int) -> str | None:
    if not value:
        return None
    return value[:limit]


def pick_logo(entries: list[dict]) -> tuple[str | None, list[dict]]:
    if not entries:
        return None, []
    usable = [entry for entry in entries if entry.get("in_use", True)] or entries
    no_feed = [entry for entry in usable if not entry.get("feed")] or usable
    png = [
        entry
        for entry in no_feed
        if str(entry.get("format") or "").upper() == "PNG"
    ]
    chosen = (png or no_feed)[0]
    return chosen.get("url"), entries


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--refresh",
        action="store_true",
        help="Recarga limpia del catalogo iptv-org",
    )
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(message)s")

    Base.metadata.create_all(bind=engine)
    db = SessionLocal()
    try:
        existing = db.scalar(select(func.count()).select_from(IptvChannel)) or 0
        if existing and not args.refresh:
            log.info(
                "iptv_channels ya tiene %s filas; usa --refresh para recargar.",
                existing,
            )
            return 0

        channels_raw = download("channels.json")
        streams_raw = download("streams.json")
        logos_raw = download("logos.json")
        categories_raw = download("categories.json")

        streams_by_channel: dict[str, list[dict]] = {}
        unlinked_streams = 0
        for stream in streams_raw:
            channel_id = stream.get("channel")
            if not channel_id:
                unlinked_streams += 1
                continue
            streams_by_channel.setdefault(channel_id, []).append(stream)

        channel_by_id = {channel.get("id"): channel for channel in channels_raw}
        importable = [
            channel_by_id[channel_id]
            for channel_id in streams_by_channel
            if channel_id in channel_by_id
        ]
        orphan_stream_channels = len(streams_by_channel) - len(importable)

        logos_by_channel: dict[str, list[dict]] = {}
        for logo in logos_raw:
            channel_id = logo.get("channel")
            if channel_id in streams_by_channel:
                logos_by_channel.setdefault(channel_id, []).append(logo)

        importable.sort(key=lambda c: (c.get("name") or "").lower())

        if args.refresh:
            for model in (IptvStream, IptvLogo, IptvChannel, IptvCategory):
                db.execute(delete(model))

        db.add_all(
            IptvCategory(
                code=item.get("id") or "",
                name_en=item.get("name") or item.get("id") or "",
                name_es=CATEGORIES_ES.get(item.get("id") or "", item.get("name") or ""),
            )
            for item in categories_raw
            if item.get("id")
        )
        # Sin relationship() SQLAlchemy no ordena por FK: forzar el orden.
        db.flush()

        channel_rows: list[IptvChannel] = []
        stream_rows: list[IptvStream] = []
        logo_rows: list[IptvLogo] = []
        too_long_ids = 0

        for index, channel in enumerate(importable, start=1):
            channel_id = channel.get("id") or ""
            if len(channel_id) > 255:
                too_long_ids += 1
                continue
            name = (channel.get("name") or channel_id)[:255]
            number = index

            streams = streams_by_channel.get(channel_id) or []
            primary = streams[0] if streams else {}
            categories = channel.get("categories") or []
            if categories:
                primary_category = CATEGORIES_ES.get(categories[0], categories[0])
            else:
                primary_category = "Generalista"
            primary_logo, entries = pick_logo(logos_by_channel.get(channel_id) or [])

            channel_rows.append(
                IptvChannel(
                    id=channel_id,
                    name=name,
                    category=primary_category[:60],
                    categories=_csv(categories),
                    website=_truncate(channel.get("website"), 500),
                    is_nsfw=bool(channel.get("is_nsfw")),
                    accent=accent_for(channel_id),
                    initials=make_initials(name),
                    number=number,
                    stream_url=primary.get("url"),
                    user_agent=_truncate(primary.get("user_agent"), 500),
                    referrer=_truncate(primary.get("referrer"), 500),
                    logo_url=primary_logo,
                )
            )
            for stream in streams:
                stream_rows.append(
                    IptvStream(
                        channel_id=channel_id,
                        feed=_truncate(stream.get("feed"), 120),
                        title=(stream.get("title") or name)[:255],
                        url=stream.get("url") or "",
                        quality=_truncate(stream.get("quality"), 20),
                        labels=_csv(stream.get("labels")),
                        user_agent=_truncate(stream.get("user_agent"), 500),
                        referrer=_truncate(stream.get("referrer"), 500),
                    )
                )
            for entry in entries:
                if not entry.get("url"):
                    continue
                logo_rows.append(
                    IptvLogo(
                        channel_id=channel_id,
                        feed=_truncate(entry.get("feed"), 120),
                        url=entry["url"],
                        format=_truncate(entry.get("format"), 16),
                        width=entry.get("width"),
                        height=entry.get("height"),
                        in_use=bool(entry.get("in_use", True)),
                        is_primary=entry.get("url") == primary_logo,
                    )
                )

        db.add_all(channel_rows)
        db.flush()
        db.add_all(stream_rows)
        db.add_all(logo_rows)
        db.commit()

        log.info("--------------------------------------------------")
        log.info("Categorias importadas:    %s", len(categories_raw))
        log.info("Canales con enlace:       %s", len(channel_rows))
        log.info("Streams (enlaces m3u8):   %s", len(stream_rows))
        log.info("Logos registrados:        %s", len(logo_rows))
        log.info("Canales sin enlace:       %s (no se importan)", len(channels_raw) - len(importable))
        log.info("Streams sin canal:        %s", unlinked_streams)
        log.info("Streams de canal huerfano:%s", orphan_stream_channels)
        if too_long_ids:
            log.info("Ids demasiados largos:     %s (omitidos)", too_long_ids)
        log.info("--------------------------------------------------")
        return 0
    finally:
        db.close()


if __name__ == "__main__":
    sys.exit(main())
