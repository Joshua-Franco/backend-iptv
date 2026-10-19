use sqlx::PgPool;

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS channels (
            id VARCHAR(64) PRIMARY KEY,
            uuid VARCHAR(36) NOT NULL UNIQUE,
            number INTEGER NOT NULL UNIQUE,
            name VARCHAR(120) NOT NULL,
            category VARCHAR(60) NOT NULL,
            initials VARCHAR(8) NOT NULL,
            accent VARCHAR(9) NOT NULL,
            logo_file VARCHAR(255) NOT NULL,
            description TEXT,
            stream_url VARCHAR(500),
            website VARCHAR(255),
            created_at TIMESTAMPTZ DEFAULT now()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS custom_sources (
            id VARCHAR(36) PRIMARY KEY,
            url TEXT NOT NULL UNIQUE,
            type VARCHAR(16) NOT NULL,
            status VARCHAR(16) NOT NULL,
            error TEXT,
            channel_count INTEGER NOT NULL,
            created_at TIMESTAMPTZ DEFAULT now()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS custom_channels (
            id VARCHAR(64) PRIMARY KEY,
            uuid VARCHAR(36) NOT NULL UNIQUE,
            source_id VARCHAR(36) NOT NULL REFERENCES custom_sources(id) ON DELETE CASCADE,
            name VARCHAR(255) NOT NULL,
            url TEXT NOT NULL,
            logo_url TEXT,
            group_name VARCHAR(160),
            category VARCHAR(160) NOT NULL,
            number INTEGER NOT NULL,
            initials VARCHAR(8) NOT NULL,
            accent VARCHAR(9) NOT NULL,
            is_direct BOOLEAN NOT NULL,
            user_agent VARCHAR(500),
            referrer VARCHAR(500),
            created_at TIMESTAMPTZ DEFAULT now()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS iptv_categories (
            code VARCHAR(40) PRIMARY KEY,
            name_en VARCHAR(60) NOT NULL,
            name_es VARCHAR(60) NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS iptv_channels (
            id VARCHAR(255) PRIMARY KEY,
            uuid VARCHAR(36) NOT NULL UNIQUE,
            name VARCHAR(255) NOT NULL,
            category VARCHAR(60) NOT NULL,
            categories TEXT NOT NULL,
            website VARCHAR(500),
            is_nsfw BOOLEAN NOT NULL,
            accent VARCHAR(9) NOT NULL,
            initials VARCHAR(8) NOT NULL,
            number INTEGER NOT NULL,
            stream_url TEXT,
            user_agent VARCHAR(500),
            referrer VARCHAR(500),
            logo_url TEXT,
            created_at TIMESTAMPTZ DEFAULT now()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS iptv_streams (
            id SERIAL PRIMARY KEY,
            channel_id VARCHAR(255) NOT NULL REFERENCES iptv_channels(id) ON DELETE CASCADE,
            feed VARCHAR(120),
            title VARCHAR(255) NOT NULL,
            url TEXT NOT NULL,
            quality VARCHAR(20),
            labels TEXT NOT NULL,
            user_agent VARCHAR(500),
            referrer VARCHAR(500)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS iptv_logos (
            id SERIAL PRIMARY KEY,
            channel_id VARCHAR(255) NOT NULL REFERENCES iptv_channels(id) ON DELETE CASCADE,
            feed VARCHAR(120),
            url TEXT NOT NULL,
            format VARCHAR(16),
            width INTEGER,
            height INTEGER,
            in_use BOOLEAN NOT NULL,
            is_primary BOOLEAN NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    for stmt in [
        "CREATE INDEX IF NOT EXISTS ix_channels_uuid ON channels (uuid)",
        "CREATE INDEX IF NOT EXISTS ix_channels_number ON channels (number)",
        "CREATE INDEX IF NOT EXISTS ix_channels_category ON channels (category)",
        "CREATE INDEX IF NOT EXISTS ix_custom_channels_source_id ON custom_channels (source_id)",
        "CREATE INDEX IF NOT EXISTS ix_custom_channels_category ON custom_channels (category)",
        "CREATE INDEX IF NOT EXISTS ix_iptv_channels_uuid ON iptv_channels (uuid)",
        "CREATE INDEX IF NOT EXISTS ix_iptv_channels_name ON iptv_channels (name)",
        "CREATE INDEX IF NOT EXISTS ix_iptv_channels_category ON iptv_channels (category)",
        "CREATE INDEX IF NOT EXISTS ix_iptv_streams_channel_id ON iptv_streams (channel_id)",
        "CREATE INDEX IF NOT EXISTS ix_iptv_logos_channel_id ON iptv_logos (channel_id)",
    ] {
        sqlx::query(stmt).execute(pool).await?;
    }

    // run_migrations: anade uuid si faltase (bases antiguas) y rellena siempre.
    let has_uuid: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns
         WHERE table_name = 'channels' AND column_name = 'uuid'",
    )
    .fetch_one(pool)
    .await?;
    if has_uuid == 0 {
        sqlx::query("ALTER TABLE channels ADD COLUMN uuid VARCHAR(36)")
            .execute(pool)
            .await?;
        sqlx::query("ALTER TABLE channels ADD CONSTRAINT channels_uuid_key UNIQUE (uuid)")
            .execute(pool)
            .await?;
        sqlx::query("CREATE INDEX ix_channels_uuid_mig ON channels (uuid)")
            .execute(pool)
            .await?;
    }
    sqlx::query(
        "UPDATE channels SET uuid = gen_random_uuid()::text WHERE uuid IS NULL OR uuid = ''",
    )
    .execute(pool)
    .await?;

    Ok(())
}
