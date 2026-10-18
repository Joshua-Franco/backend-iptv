"""Elimina paises del catalogo iptv en la base de datos.

Ejecutar SOLO despues de desplegar en Render el codigo que ya no
referencia `country` (el codigo viejo haria SELECT sobre la columna
y fallaria).

    python -m scripts.drop_countries
"""
from sqlalchemy import text

from database.connection import engine


def main() -> int:
    statements = (
        "ALTER TABLE iptv_channels DROP CONSTRAINT IF EXISTS iptv_channels_country_fkey",
        "ALTER TABLE iptv_channels DROP COLUMN IF EXISTS country",
        "DROP TABLE IF EXISTS iptv_countries",
    )
    with engine.begin() as conn:
        for statement in statements:
            conn.execute(text(statement))
    print("Listo: tabla iptv_countries, columna country y su FK eliminadas.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
