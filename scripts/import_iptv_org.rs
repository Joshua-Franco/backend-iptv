use std::collections::HashMap;
use std::time::Duration;

use clap::Parser;
use serde::Deserialize;

use iptv_peru_api::config;
use iptv_peru_api::database;
use iptv_peru_api::services::palette::{accent_for, make_initials};

const API: &str = "https://iptv-org.github.io/api";

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Importa el catalogo completo de iptv-org a PostgreSQL (Neon)"
)]
struct Args {
    #[arg(long, help = "Recarga limpia del catalogo iptv-org")]
    refresh: bool,
}

#[derive(Debug, Deserialize)]
struct ChannelItem {
    id: Option<String>,
    name: Option<String>,
    categories: Option<Vec<String>>,
    website: Option<String>,
    is_nsfw: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
struct StreamItem {
    channel: Option<String>,
    feed: Option<String>,
    title: Option<String>,
    url: Option<String>,
    quality: Option<String>,
    labels: Option<Vec<String>>,
    user_agent: Option<String>,
    referrer: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct LogoItem {
    channel: Option<String>,
    feed: Option<String>,
    url: Option<String>,
    format: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    in_use: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CategoryItem {
    id: Option<String>,
    name: Option<String>,
}

fn category_es(code: &str, default_name: &str) -> String {
    match code {
        "auto" => "Automoviles",
        "animation" => "Animacion",
        "business" => "Negocios",
        "classic" => "Clasicos",
        "comedy" => "Comedia",
        "cooking" => "Cocina",
        "culture" => "Cultura",
        "documentary" => "Documentales",
        "education" => "Educacion",
        "entertainment" => "Entretenimiento",
        "family" => "Familia",
        "general" => "Generalista",
        "interactive" => "Interactivo",
        "kids" => "Infantil",
        "legislative" => "Legislativo",
        "lifestyle" => "Estilo de vida",
        "movies" => "Cine",
        "music" => "Musica",
        "news" => "Noticias",
        "outdoor" => "Aire libre",
        "public" => "Publico",
        "relax" => "Relax",
        "religious" => "Religioso",
        "series" => "Series",
        "science" => "Ciencia",
        "shop" => "Compras",
        "sports" => "Deportes",
        "travel" => "Viajes",
        "weather" => "Clima",
        "xxx" => "Adultos",
        _ => default_name,
    }
    .to_string()
}

fn pick_logo(entries: &[LogoItem]) -> (Option<String>, Vec<LogoItem>) {
    if entries.is_empty() {
        return (None, Vec::new());
    }
    let usable: Vec<LogoItem> = entries
        .iter()
        .filter(|e| e.in_use.unwrap_or(true))
        .cloned()
        .collect();
    let usable = if usable.is_empty() {
        entries.to_vec()
    } else {
        usable
    };

    let no_feed: Vec<LogoItem> = usable
        .iter()
        .filter(|e| e.feed.is_none() || e.feed.as_deref() == Some(""))
        .cloned()
        .collect();
    let no_feed = if no_feed.is_empty() { usable } else { no_feed };

    let png: Vec<LogoItem> = no_feed
        .iter()
        .filter(|e| {
            e.format
                .as_deref()
                .map(|f| f.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    let chosen = if !png.is_empty() {
        &png[0]
    } else {
        &no_feed[0]
    };
    (chosen.url.clone(), entries.to_vec())
}

async fn download<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    file_name: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    let url = format!("{API}/{file_name}");
    println!("Descargando {url} ...");
    let resp = client.get(&url).send().await?.error_for_status()?;
    let bytes = resp.bytes().await?;
    let data = serde_json::from_slice::<T>(&bytes)?;
    Ok(data)
}

fn csv_str(values: Option<&[String]>, limit: Option<usize>) -> String {
    let s = values
        .unwrap_or(&[])
        .iter()
        .map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(",");
    if let Some(lim) = limit {
        if s.len() > lim {
            s[..lim].to_string()
        } else {
            s
        }
    } else {
        s
    }
}

fn truncate_str(val: Option<&str>, limit: usize) -> Option<String> {
    val.map(|s| {
        if s.len() > limit {
            s[..limit].to_string()
        } else {
            s.to_string()
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    config::load_dotenv();
    let url = config::database_url();
    let pool = database::connect(&url).await;
    database::run_migrations(&pool).await?;

    let existing: i64 = sqlx::query_scalar("SELECT count(*) FROM iptv_channels")
        .fetch_one(&pool)
        .await?;

    if existing > 0 && !args.refresh {
        println!("iptv_channels ya tiene {existing} filas; usa --refresh para recargar.");
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    let channels_raw: Vec<ChannelItem> = download(&client, "channels.json").await?;
    let streams_raw: Vec<StreamItem> = download(&client, "streams.json").await?;
    let logos_raw: Vec<LogoItem> = download(&client, "logos.json").await?;
    let categories_raw: Vec<CategoryItem> = download(&client, "categories.json").await?;

    let mut streams_by_channel: HashMap<String, Vec<StreamItem>> = HashMap::new();
    let mut unlinked_streams = 0;
    for stream in streams_raw {
        if let Some(ref ch) = stream.channel {
            if !ch.is_empty() {
                streams_by_channel
                    .entry(ch.clone())
                    .or_default()
                    .push(stream);
                continue;
            }
        }
        unlinked_streams += 1;
    }

    let mut channel_by_id: HashMap<String, ChannelItem> = HashMap::new();
    for ch in channels_raw {
        if let Some(ref id) = ch.id {
            channel_by_id.insert(id.clone(), ch);
        }
    }

    let mut importable: Vec<ChannelItem> = Vec::new();
    for ch_id in streams_by_channel.keys() {
        if let Some(ch) = channel_by_id.remove(ch_id) {
            importable.push(ch);
        }
    }
    let orphan_stream_channels = streams_by_channel.len() - importable.len();

    let mut logos_by_channel: HashMap<String, Vec<LogoItem>> = HashMap::new();
    for logo in logos_raw {
        if let Some(ref ch) = logo.channel {
            if streams_by_channel.contains_key(ch) {
                logos_by_channel.entry(ch.clone()).or_default().push(logo);
            }
        }
    }

    importable.sort_by(|a, b| {
        a.name
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .cmp(&b.name.as_deref().unwrap_or("").to_lowercase())
    });

    if args.refresh {
        sqlx::query("DELETE FROM iptv_streams")
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM iptv_logos").execute(&pool).await?;
        sqlx::query("DELETE FROM iptv_channels")
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM iptv_categories")
            .execute(&pool)
            .await?;
    }

    for cat in &categories_raw {
        if let Some(ref id) = cat.id {
            let name_en = cat.name.as_deref().unwrap_or(id);
            let name_es = category_es(id, name_en);
            sqlx::query(
                "INSERT INTO iptv_categories (code, name_en, name_es) VALUES ($1, $2, $3)
                 ON CONFLICT (code) DO UPDATE SET name_en = EXCLUDED.name_en, name_es = EXCLUDED.name_es",
            )
            .bind(id)
            .bind(name_en)
            .bind(&name_es)
            .execute(&pool)
            .await?;
        }
    }

    let mut channel_count = 0;
    let mut stream_count = 0;
    let mut logo_count = 0;
    let mut too_long_ids = 0;

    for (index, channel) in importable.iter().enumerate() {
        let channel_id = channel.id.as_deref().unwrap_or("");
        if channel_id.len() > 255 || channel_id.is_empty() {
            too_long_ids += 1;
            continue;
        }
        let raw_name = channel.name.as_deref().unwrap_or(channel_id);
        let name: String = raw_name.chars().take(255).collect();
        let number = (index + 1) as i32;

        let streams = streams_by_channel.get(channel_id);
        let primary = streams.and_then(|s| s.first());
        let categories = channel.categories.as_deref();
        let primary_category = if let Some(cats) = categories {
            if let Some(first) = cats.first() {
                category_es(first, first)
            } else {
                "Generalista".to_string()
            }
        } else {
            "Generalista".to_string()
        };
        let primary_category: String = primary_category.chars().take(60).collect();

        let empty_logos = Vec::new();
        let (primary_logo, entries) =
            pick_logo(logos_by_channel.get(channel_id).unwrap_or(&empty_logos));

        let accent = accent_for(channel_id);
        let initials = make_initials(&name);
        let uuid_str = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO iptv_channels (id, uuid, name, category, categories, website, is_nsfw, accent, initials, number, stream_url, user_agent, referrer, logo_url)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (id) DO UPDATE SET
                uuid = EXCLUDED.uuid,
                name = EXCLUDED.name,
                category = EXCLUDED.category,
                categories = EXCLUDED.categories,
                website = EXCLUDED.website,
                is_nsfw = EXCLUDED.is_nsfw,
                accent = EXCLUDED.accent,
                initials = EXCLUDED.initials,
                number = EXCLUDED.number,
                stream_url = EXCLUDED.stream_url,
                user_agent = EXCLUDED.user_agent,
                referrer = EXCLUDED.referrer,
                logo_url = EXCLUDED.logo_url",
        )
        .bind(channel_id)
        .bind(&uuid_str)
        .bind(&name)
        .bind(&primary_category)
        .bind(csv_str(categories, None))
        .bind(truncate_str(channel.website.as_deref(), 500))
        .bind(channel.is_nsfw.unwrap_or(false))
        .bind(&accent)
        .bind(&initials)
        .bind(number)
        .bind(primary.and_then(|s| s.url.clone()))
        .bind(truncate_str(primary.and_then(|s| s.user_agent.as_deref()), 500))
        .bind(truncate_str(primary.and_then(|s| s.referrer.as_deref()), 500))
        .bind(primary_logo.as_deref())
        .execute(&pool)
        .await?;
        channel_count += 1;

        if let Some(stream_list) = streams {
            for stream in stream_list {
                let st_title: String = stream
                    .title
                    .as_deref()
                    .unwrap_or(&name)
                    .chars()
                    .take(255)
                    .collect();
                sqlx::query(
                    "INSERT INTO iptv_streams (channel_id, feed, title, url, quality, labels, user_agent, referrer)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                )
                .bind(channel_id)
                .bind(truncate_str(stream.feed.as_deref(), 120))
                .bind(&st_title)
                .bind(stream.url.as_deref().unwrap_or(""))
                .bind(truncate_str(stream.quality.as_deref(), 20))
                .bind(csv_str(stream.labels.as_deref(), None))
                .bind(truncate_str(stream.user_agent.as_deref(), 500))
                .bind(truncate_str(stream.referrer.as_deref(), 500))
                .execute(&pool)
                .await?;
                stream_count += 1;
            }
        }

        for entry in entries {
            let Some(ref logo_url) = entry.url else {
                continue;
            };
            if logo_url.is_empty() {
                continue;
            }
            let is_prim = primary_logo.as_ref() == Some(logo_url);
            sqlx::query(
                "INSERT INTO iptv_logos (channel_id, feed, url, format, width, height, in_use, is_primary)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(channel_id)
            .bind(truncate_str(entry.feed.as_deref(), 120))
            .bind(logo_url)
            .bind(truncate_str(entry.format.as_deref(), 16))
            .bind(entry.width)
            .bind(entry.height)
            .bind(entry.in_use.unwrap_or(true))
            .bind(is_prim)
            .execute(&pool)
            .await?;
            logo_count += 1;
        }
    }

    println!("--------------------------------------------------");
    println!("Categorias importadas:    {}", categories_raw.len());
    println!("Canales con enlace:       {channel_count}");
    println!("Streams (enlaces m3u8):   {stream_count}");
    println!("Logos registrados:        {logo_count}");
    println!("Streams sin canal:        {unlinked_streams}");
    println!("Streams de canal huerfano:{orphan_stream_channels}");
    if too_long_ids > 0 {
        println!("Ids demasiados largos:    {too_long_ids} (omitidos)");
    }
    println!("--------------------------------------------------");

    Ok(())
}
