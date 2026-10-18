from sqlalchemy import text

from database.connection import engine

with engine.connect() as conn:
    version = conn.execute(text("select version()")).scalar()
    print("CONECTADO:", version[:70])
    tables = conn.execute(
        text(
            "select table_name from information_schema.tables "
            "where table_schema='public' order by table_name"
        )
    ).fetchall()
    print("tablas:", [t[0] for t in tables])
