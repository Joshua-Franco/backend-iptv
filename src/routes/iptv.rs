use std::collections::{HashMap, HashSet};

use axum::body::Body;
use axum::extract::{Path, RawQuery, State};
use axum::http::header::{HeaderName, HeaderValue, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::to_value;
use sqlx::PgPool;

use crate::error::{json_response, ApiError};
use crate::models::IptvChannel;
use crate::routes::{internal, qparams};
use crate::schemas::{ChannelOut, ChannelPageOut, IptvCategoryOut};
use crate::services::{channels, images};
use crate::validate::{parse_page_params, required_query};

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/iptv/categories", get(categories))
        .route("/iptv/categories/{code}/channels", get(category_channels))
        .route("/iptv/catalog", get(catalog))
        .route("/iptv/channels", get(channels_ids))
        .route("/iptv/channels/{channel_id}/logo", get(logo))
}

async fn categories(State(pool): State<PgPool>) -> Result<Response, ApiError> {
    let csvs: Vec<Option<String>> = sqlx::query_scalar("SELECT categories FROM iptv_channels")
        .fetch_all(&pool)
        .await
        .map_err(internal)?;

    let mut tag_counts: HashMap<String, i64> = HashMap::new();
    for csv in csvs {
        let Some(csv) = csv else { continue };
        let mut seen: HashSet<&str> = HashSet::new();
        for part in csv.split(',') {
            let tag = part.trim();
            if tag.is_empty() || !seen.insert(tag) {
                continue;
            }
            *tag_counts.entry(tag.to_string()).or_insert(0) += 1;
        }
    }

    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT code, name_es, name_en FROM iptv_categories")
            .fetch_all(&pool)
            .await
            .map_err(internal)?;

    let mut items: Vec<IptvCategoryOut> = rows
        .into_iter()
        .map(|(code, name_es, name_en)| {
            let count = tag_counts.get(&code).copied().unwrap_or(0);
            IptvCategoryOut {
                code,
                name: name_es,
                name_en,
                count,
            }
        })
        .collect();
    items.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
    Ok(json_response(StatusCode::OK, &to_value(items).unwrap()))
}

async fn category_channels(
    State(pool): State<PgPool>,
    Path(code): Path<String>,
    raw: RawQuery,
) -> Result<Response, ApiError> {
    let params = qparams(&raw);
    let (limit, offset, _) = parse_page_params(&params, false).map_err(ApiError::validation)?;

    let exists: Option<String> =
        sqlx::query_scalar("SELECT code FROM iptv_categories WHERE code = $1")
            .bind(&code)
            .fetch_optional(&pool)
            .await
            .map_err(internal)?;
    if exists.is_none() {
        return Err(ApiError::not_found(format!(
            "Categoria '{code}' no encontrada"
        )));
    }

    let pattern = format!("%{code}%");
    let total: i64 =
        sqlx::query_scalar("SELECT count(*) FROM iptv_channels WHERE categories ILIKE $1")
            .bind(&pattern)
            .fetch_one(&pool)
            .await
            .map_err(internal)?;
    let rows: Vec<IptvChannel> = sqlx::query_as::<_, IptvChannel>(
        "SELECT * FROM iptv_channels WHERE categories ILIKE $1 ORDER BY name, id LIMIT $2 OFFSET $3"
    )
    .bind(&pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(internal)?;

    let out = ChannelPageOut {
        items: rows.iter().map(channels::to_iptv_out).collect(),
        total,
        limit,
        offset,
    };
    Ok(json_response(StatusCode::OK, &to_value(out).unwrap()))
}

async fn catalog(State(pool): State<PgPool>, raw: RawQuery) -> Result<Response, ApiError> {
    let params = qparams(&raw);
    let (limit, offset, q) = parse_page_params(&params, true).map_err(ApiError::validation)?;

    let mut filter: Option<String> = None;
    if let Some(q) = q.as_deref() {
        if !q.trim().is_empty() {
            filter = Some(format!("%{}%", q.trim()));
        }
    }

    let total: i64 = match &filter {
        Some(pattern) => sqlx::query_scalar(
            "SELECT count(*) FROM iptv_channels WHERE name ILIKE $1 OR category ILIKE $1",
        )
        .bind(pattern)
        .fetch_one(&pool)
        .await
        .map_err(internal)?,
        None => sqlx::query_scalar("SELECT count(*) FROM iptv_channels")
            .fetch_one(&pool)
            .await
            .map_err(internal)?,
    };

    let rows: Vec<IptvChannel> = match &filter {
        Some(pattern) => {
            sqlx::query_as::<_, IptvChannel>(
                "SELECT * FROM iptv_channels WHERE name ILIKE $1 OR category ILIKE $1 ORDER BY name, id LIMIT $2 OFFSET $3"
            )
            .bind(pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
            .map_err(internal)?
        }
        None => {
            sqlx::query_as::<_, IptvChannel>(
                "SELECT * FROM iptv_channels ORDER BY name, id LIMIT $1 OFFSET $2"
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
            .map_err(internal)?
        }
    };

    let out = ChannelPageOut {
        items: rows.iter().map(channels::to_iptv_out).collect(),
        total,
        limit,
        offset,
    };
    Ok(json_response(StatusCode::OK, &to_value(out).unwrap()))
}

async fn channels_ids(State(pool): State<PgPool>, raw: RawQuery) -> Result<Response, ApiError> {
    let params = qparams(&raw);
    let ids = required_query(&params, "ids").map_err(ApiError::validation)?;
    let keys: Vec<String> = ids
        .split(',')
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .take(200)
        .map(str::to_string)
        .collect();
    if keys.is_empty() {
        return Ok(json_response(
            StatusCode::OK,
            &to_value(Vec::<ChannelOut>::new()).unwrap(),
        ));
    }

    let mut qb: sqlx::QueryBuilder<sqlx::Postgres> =
        sqlx::QueryBuilder::new("SELECT * FROM iptv_channels WHERE ");
    qb.push("id IN (");
    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        qb.push_bind(key.clone());
    }
    qb.push(") OR uuid IN (");
    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        qb.push_bind(key.clone());
    }
    qb.push(")");
    let rows: Vec<IptvChannel> = qb
        .build_query_as()
        .fetch_all(&pool)
        .await
        .map_err(internal)?;

    let out: Vec<ChannelOut> = rows.iter().map(channels::to_iptv_out).collect();
    Ok(json_response(StatusCode::OK, &to_value(out).unwrap()))
}

async fn logo(
    State(pool): State<PgPool>,
    Path(channel_id): Path<String>,
) -> Result<Response, ApiError> {
    let logo_url: Option<Option<String>> =
        sqlx::query_scalar("SELECT logo_url FROM iptv_channels WHERE id = $1")
            .bind(&channel_id)
            .fetch_optional(&pool)
            .await
            .map_err(internal)?;
    let Some(logo_url) = logo_url else {
        return Err(ApiError::not_found(format!(
            "Canal '{channel_id}' no encontrado"
        )));
    };
    let (data, image_type) = images::get_image(logo_url.as_deref()).await?;
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, image_type)
        .header(CACHE_CONTROL, "public, max-age=86400")
        .header(
            HeaderName::from_static("access-control-allow-origin"),
            HeaderValue::from_static("*"),
        )
        .body(Body::from(data))
        .map_err(|_| ApiError::Http {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            detail: "Internal Server Error".into(),
        })
}
