use axum::body::Body;
use axum::extract::{Path, RawQuery, State};
use axum::http::header::{HeaderName, HeaderValue, CONTENT_TYPE};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use sqlx::PgPool;

use crate::config::API_PREFIX;
use crate::error::ApiError;
use crate::routes::{internal, qparams};
use crate::services::hls;
use crate::validate::required_query;

const M3U8_CONTENT_TYPE: &str = "application/vnd.apple.mpegurl";

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/stream/{channel_id}/playlist.m3u8", get(playlist))
        .route("/stream/{channel_id}/variant", get(variant))
        .route("/stream/{channel_id}/asset", get(asset))
}

struct Resolved {
    url: String,
    is_direct: bool,
    headers: Vec<(String, String)>,
}

fn headers_from(user_agent: Option<&str>, referrer: Option<&str>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(ua) = user_agent {
        if !ua.is_empty() {
            out.push(("User-Agent".to_string(), ua.to_string()));
        }
    }
    if let Some(rf) = referrer {
        if !rf.is_empty() {
            out.push(("Referer".to_string(), rf.to_string()));
        }
    }
    out
}

async fn resolve_stream(pool: &PgPool, channel_id: &str) -> Result<Resolved, ApiError> {
    let row: Option<Option<String>> =
        sqlx::query_scalar("SELECT stream_url FROM channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(pool)
            .await
            .map_err(internal)?;
    if let Some(stream_url) = row {
        return match stream_url.filter(|s| !s.is_empty()) {
            Some(url) => Ok(Resolved {
                url,
                is_direct: false,
                headers: Vec::new(),
            }),
            None => Err(ApiError::not_found(format!(
                "El canal '{channel_id}' no tiene senal asignada"
            ))),
        };
    }

    let iptv: Option<(Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT stream_url, user_agent, referrer FROM iptv_channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(pool)
            .await
            .map_err(internal)?;
    if let Some((stream_url, ua, rf)) = iptv {
        return match stream_url.filter(|s| !s.is_empty()) {
            Some(url) => Ok(Resolved {
                url,
                is_direct: false,
                headers: headers_from(ua.as_deref(), rf.as_deref()),
            }),
            None => Err(ApiError::not_found(format!(
                "El canal '{channel_id}' no tiene senal asignada"
            ))),
        };
    }

    let custom: Option<(String, bool, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT url, is_direct, user_agent, referrer FROM custom_channels WHERE id = $1",
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await
    .map_err(internal)?;
    if let Some((url, is_direct, ua, rf)) = custom {
        return Ok(Resolved {
            url,
            is_direct,
            headers: headers_from(ua.as_deref(), rf.as_deref()),
        });
    }

    Err(ApiError::not_found(format!(
        "Canal '{channel_id}' no encontrado"
    )))
}

fn set_channel_headers(channel_id: &str, headers: Vec<(String, String)>) {
    hls::set_channel_headers(channel_id, &headers);
}

async fn playlist(
    State(pool): State<PgPool>,
    Path(channel_id): Path<String>,
) -> Result<Response, ApiError> {
    let resolved = resolve_stream(&pool, &channel_id).await?;
    set_channel_headers(&channel_id, resolved.headers);
    let prefix = API_PREFIX.as_str();
    let body = if resolved.is_direct {
        hls::build_direct_playlist(&channel_id, &resolved.url, prefix)
    } else {
        hls::get_playlist(&channel_id, &resolved.url, prefix).await?
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, M3U8_CONTENT_TYPE)
        .body(Body::from(body))
        .expect("response builder"))
}

async fn variant(
    State(pool): State<PgPool>,
    Path(channel_id): Path<String>,
    raw: RawQuery,
) -> Result<Response, ApiError> {
    let params = qparams(&raw);
    let u = required_query(&params, "u").map_err(ApiError::validation)?;
    let resolved = resolve_stream(&pool, &channel_id).await?;
    set_channel_headers(&channel_id, resolved.headers);
    hls::validate_proxy_url(&channel_id, u)?;
    let prefix = API_PREFIX.as_str();
    let body = hls::get_playlist(&channel_id, u, prefix).await?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, M3U8_CONTENT_TYPE)
        .body(Body::from(body))
        .expect("response builder"))
}

async fn asset(
    State(pool): State<PgPool>,
    Path(channel_id): Path<String>,
    raw: RawQuery,
) -> Result<Response, ApiError> {
    let params = qparams(&raw);
    let u = required_query(&params, "u").map_err(ApiError::validation)?;
    let resolved = resolve_stream(&pool, &channel_id).await?;
    set_channel_headers(&channel_id, resolved.headers);
    let (content_type, response) = hls::proxy_asset(&channel_id, u).await?;
    let content_type = HeaderValue::from_str(&content_type)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    let res = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(
            HeaderName::from_static("access-control-allow-origin"),
            HeaderValue::from_static("*"),
        )
        .body(hls::asset_body_stream(response))
        .expect("response builder");
    Ok(res)
}
