import logging

from sqlalchemy import select
from sqlalchemy.orm import Session

from database.connection import SessionLocal
from models.channel import Channel

CHANNELS: list[dict] = [
    {
        "id": "willax",
        "number": 1,
        "name": "Willax",
        "category": "Entretenimiento",
        "initials": "WX",
        "accent": "#e30613",
        "logo_file": "willax.png",
        "description": "Canal 1 de la TDT de Lima: noticias, opinion y programas propios.",
        "website": "https://www.willax.tv",
    },
    {
        "id": "latina",
        "number": 2,
        "name": "Latina Televisión",
        "category": "Generalista",
        "initials": "LT",
        "accent": "#d1112b",
        "logo_file": "latina.png",
        "description": "Canal 2 de la TDT de Lima: entretenimiento, realities y noticias.",
        "stream_url": "https://redirector.rudo.video/hls-video/567ffde3fa319fadf3419efda25619456231dfea/latina/latina.smil/playlist.m3u8",
        "website": "https://www.latina.pe",
    },
    {
        "id": "rpp-tv",
        "number": 3,
        "name": "RPP TV",
        "category": "Noticias",
        "initials": "RPP",
        "accent": "#003a78",
        "logo_file": "rpp-tv.png",
        "description": "Canal 3 de la TDT de Lima: informacion continua del Grupo RPP.",
        "stream_url": "https://redirector.rudo.video/hls-video/567ffde3fa319fadf3419efda25619456231dfea/rpptv/rpptv.smil/playlist.m3u8",
        "website": "https://rpp.pe",
    },
    {
        "id": "america-tv",
        "number": 4,
        "name": "América Televisión",
        "category": "Generalista",
        "initials": "AM",
        "accent": "#ef4900",
        "logo_file": "america-tv.svg",
        "description": "Canal 4 de la TDT de Lima: el canal con mas anos de television en el Peru.",
        "stream_url": "http://bantel-cdn1.iptvperu.tv:1935/btnscrtn/america.stream/playlist.m3u8",
        "website": "https://www.americatv.com.pe",
    },
    {
        "id": "panamericana",
        "number": 5,
        "name": "Panamericana Televisión",
        "category": "Generalista",
        "initials": "PT",
        "accent": "#c8102e",
        "logo_file": "panamericana.png",
        "description": "Canal 5 de la TDT de Lima: series, novelas, deportes y noticieros.",
        "website": "https://panamericana.pe",
    },
    {
        "id": "exitosa",
        "number": 6,
        "name": "Exitosa TV",
        "category": "Noticias",
        "initials": "EX",
        "accent": "#f58220",
        "logo_file": "exitosa.png",
        "description": "Canal 6 de la TDT de Lima: noticias y programas de opinion.",
        "website": "https://www.exitosanoticias.pe",
    },
    {
        "id": "tv-peru",
        "number": 7,
        "name": "TV Perú",
        "category": "Cultural",
        "initials": "TVP",
        "accent": "#1e5aa8",
        "logo_file": "tv-peru.png",
        "description": "Canal 7 de la TDT de Lima: television publica del IRTP.",
        "website": "https://www.tvperu.gob.pe",
    },
    {
        "id": "canal-n",
        "number": 8,
        "name": "Canal N",
        "category": "Noticias",
        "initials": "CN",
        "accent": "#00a3e0",
        "logo_file": "canal-n.png",
        "description": "Noticias las 24 horas de América Multimedia.",
        "website": "https://canaln.pe",
    },
    {
        "id": "atv",
        "number": 9,
        "name": "ATV",
        "category": "Generalista",
        "initials": "ATV",
        "accent": "#e4002b",
        "logo_file": "atv.png",
        "description": "Canal 9 de la TDT de Lima: entretenimiento y magazine del Grupo ATV.",
        "website": "https://www.atv.pe",
    },
    {
        "id": "la-tele",
        "number": 10,
        "name": "La Tele",
        "category": "Entretenimiento",
        "initials": "LTE",
        "accent": "#e8552e",
        "logo_file": "la-tele.png",
        "description": "Canal 10 de la TDT de Lima: variedades y programacion de Empresa Alliance.",
    },
    {
        "id": "viva-tv",
        "number": 11,
        "name": "Viva TV",
        "category": "Generalista",
        "initials": "VT",
        "accent": "#d62828",
        "logo_file": "viva-tv.png",
        "description": "Canal 11 de la TDT de Lima: television generalista de la Red Bicolor.",
    },
    {
        "id": "global",
        "number": 13,
        "name": "Global Televisión",
        "category": "Generalista",
        "initials": "GT",
        "accent": "#f7941e",
        "logo_file": "global.png",
        "description": "Canal 13 de la TDT de Lima: series, novelas y entretenimiento.",
    },
]


def seed_channels(db: Session) -> int:
    created = 0
    for data in CHANNELS:
        exists = db.get(Channel, data["id"])
        if exists:
            continue
        db.add(Channel(**data))
        created += 1
    if created:
        db.commit()
    return created


def seed_if_empty(db: Session) -> int:
    count = db.scalar(select(Channel.id).limit(1))
    if count is None:
        return seed_channels(db)
    return 0


def update_streams(db: Session) -> int:
    updated = 0
    for data in CHANNELS:
        stream_url = data.get("stream_url")
        if not stream_url:
            continue
        channel = db.get(Channel, data["id"])
        if channel and channel.stream_url != stream_url:
            channel.stream_url = stream_url
            updated += 1
    if updated:
        db.commit()
    return updated


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    session = SessionLocal()
    try:
        inserted = seed_channels(session)
        logging.getLogger(__name__).info("Canales insertados: %s", inserted)
    finally:
        session.close()
