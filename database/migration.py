from sqlalchemy import inspect, text
from sqlalchemy.engine import Engine


def run_migrations(engine: Engine) -> None:
    inspector = inspect(engine)
    columns = {col["name"] for col in inspector.get_columns("channels")}

    with engine.begin() as conn:
        if "uuid" not in columns:
            conn.execute(text("ALTER TABLE channels ADD COLUMN uuid VARCHAR(36)"))
            conn.execute(
                text(
                    "ALTER TABLE channels ADD CONSTRAINT channels_uuid_key "
                    "UNIQUE (uuid)"
                )
            )
            conn.execute(text("CREATE INDEX ix_channels_uuid ON channels (uuid)"))
        conn.execute(
            text(
                "UPDATE channels SET uuid = gen_random_uuid()::text "
                "WHERE uuid IS NULL OR uuid = ''"
            )
        )
