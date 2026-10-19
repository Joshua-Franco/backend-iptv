use sqlx::PgPool;

pub struct SeedChannel {
    pub id: &'static str,
    pub number: i32,
    pub name: &'static str,
    pub category: &'static str,
    pub initials: &'static str,
    pub accent: &'static str,
    pub logo_file: &'static str,
    pub description: &'static str,
    pub stream_url: Option<&'static str>,
    pub website: Option<&'static str>,
}

pub const CHANNELS: &[SeedChannel] = &[
    SeedChannel {
        id: "willax",
        number: 1,
        name: "Willax",
        category: "Entretenimiento",
        initials: "WX",
        accent: "#e30613",
        logo_file: "willax.png",
        description: "Canal 1 de la TDT de Lima: noticias, opinion y programas propios.",
        stream_url: None,
        website: Some("https://www.willax.tv"),
    },
    SeedChannel {
        id: "latina",
        number: 2,
        name: "Latina Televisión",
        category: "Generalista",
        initials: "LT",
        accent: "#d1112b",
        logo_file: "latina.png",
        description: "Canal 2 de la TDT de Lima: entretenimiento, realities y noticias.",
        stream_url: Some(
            "https://redirector.rudo.video/hls-video/567ffde3fa319fadf3419efda25619456231dfea/latina/latina.smil/playlist.m3u8",
        ),
        website: Some("https://www.latina.pe"),
    },
    SeedChannel {
        id: "rpp-tv",
        number: 3,
        name: "RPP TV",
        category: "Noticias",
        initials: "RPP",
        accent: "#003a78",
        logo_file: "rpp-tv.png",
        description: "Canal 3 de la TDT de Lima: informacion continua del Grupo RPP.",
        stream_url: Some(
            "https://redirector.rudo.video/hls-video/567ffde3fa319fadf3419efda25619456231dfea/rpptv/rpptv.smil/playlist.m3u8",
        ),
        website: Some("https://rpp.pe"),
    },
    SeedChannel {
        id: "america-tv",
        number: 4,
        name: "América Televisión",
        category: "Generalista",
        initials: "AM",
        accent: "#ef4900",
        logo_file: "america-tv.svg",
        description: "Canal 4 de la TDT de Lima: el canal con mas anos de television en el Peru.",
        stream_url: Some(
            "http://bantel-cdn1.iptvperu.tv:1935/btnscrtn/america.stream/playlist.m3u8",
        ),
        website: Some("https://www.americatv.com.pe"),
    },
    SeedChannel {
        id: "panamericana",
        number: 5,
        name: "Panamericana Televisión",
        category: "Generalista",
        initials: "PT",
        accent: "#c8102e",
        logo_file: "panamericana.png",
        description: "Canal 5 de la TDT de Lima: series, novelas, deportes y noticieros.",
        stream_url: None,
        website: Some("https://panamericana.pe"),
    },
    SeedChannel {
        id: "exitosa",
        number: 6,
        name: "Exitosa TV",
        category: "Noticias",
        initials: "EX",
        accent: "#f58220",
        logo_file: "exitosa.png",
        description: "Canal 6 de la TDT de Lima: noticias y programas de opinion.",
        stream_url: None,
        website: Some("https://www.exitosanoticias.pe"),
    },
    SeedChannel {
        id: "tv-peru",
        number: 7,
        name: "TV Perú",
        category: "Cultural",
        initials: "TVP",
        accent: "#1e5aa8",
        logo_file: "tv-peru.png",
        description: "Canal 7 de la TDT de Lima: television publica del IRTP.",
        stream_url: None,
        website: Some("https://www.tvperu.gob.pe"),
    },
    SeedChannel {
        id: "canal-n",
        number: 8,
        name: "Canal N",
        category: "Noticias",
        initials: "CN",
        accent: "#00a3e0",
        logo_file: "canal-n.png",
        description: "Noticias las 24 horas de América Multimedia.",
        stream_url: None,
        website: Some("https://canaln.pe"),
    },
    SeedChannel {
        id: "atv",
        number: 9,
        name: "ATV",
        category: "Generalista",
        initials: "ATV",
        accent: "#e4002b",
        logo_file: "atv.png",
        description: "Canal 9 de la TDT de Lima: entretenimiento y magazine del Grupo ATV.",
        stream_url: None,
        website: Some("https://www.atv.pe"),
    },
    SeedChannel {
        id: "la-tele",
        number: 10,
        name: "La Tele",
        category: "Entretenimiento",
        initials: "LTE",
        accent: "#e8552e",
        logo_file: "la-tele.png",
        description: "Canal 10 de la TDT de Lima: variedades y programacion de Empresa Alliance.",
        stream_url: None,
        website: None,
    },
    SeedChannel {
        id: "viva-tv",
        number: 11,
        name: "Viva TV",
        category: "Generalista",
        initials: "VT",
        accent: "#d62828",
        logo_file: "viva-tv.png",
        description: "Canal 11 de la TDT de Lima: television generalista de la Red Bicolor.",
        stream_url: None,
        website: None,
    },
    SeedChannel {
        id: "global",
        number: 13,
        name: "Global Televisión",
        category: "Generalista",
        initials: "GT",
        accent: "#f7941e",
        logo_file: "global.png",
        description: "Canal 13 de la TDT de Lima: series, novelas y entretenimiento.",
        stream_url: None,
        website: None,
    },
];

pub async fn seed_channels(pool: &PgPool) -> Result<usize, sqlx::Error> {
    let mut created = 0;
    for data in CHANNELS {
        let exists: Option<String> = sqlx::query_scalar("SELECT id FROM channels WHERE id = $1")
            .bind(data.id)
            .fetch_optional(pool)
            .await?;
        if exists.is_some() {
            continue;
        }
        sqlx::query(
            "INSERT INTO channels (id, uuid, number, name, category, initials, accent,
                logo_file, description, stream_url, website)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(data.id)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(data.number)
        .bind(data.name)
        .bind(data.category)
        .bind(data.initials)
        .bind(data.accent)
        .bind(data.logo_file)
        .bind(data.description)
        .bind(data.stream_url)
        .bind(data.website)
        .execute(pool)
        .await?;
        created += 1;
    }
    Ok(created)
}

pub async fn seed_if_empty(pool: &PgPool) -> Result<usize, sqlx::Error> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM channels LIMIT 1")
        .fetch_optional(pool)
        .await?;
    if existing.is_none() {
        seed_channels(pool).await
    } else {
        Ok(0)
    }
}

pub async fn update_streams(pool: &PgPool) -> Result<usize, sqlx::Error> {
    let mut updated = 0;
    for data in CHANNELS {
        let Some(want) = data.stream_url else {
            continue;
        };
        let current: Option<Option<String>> =
            sqlx::query_scalar("SELECT stream_url FROM channels WHERE id = $1")
                .bind(data.id)
                .fetch_optional(pool)
                .await?;
        if let Some(Some(cur)) = &current {
            if cur.as_str() == want {
                continue;
            }
        }
        if current.is_some() {
            sqlx::query("UPDATE channels SET stream_url = $1 WHERE id = $2")
                .bind(want)
                .bind(data.id)
                .execute(pool)
                .await?;
            updated += 1;
        }
    }
    Ok(updated)
}
