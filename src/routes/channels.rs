use axum::body::Bytes;
use axum::extract::{Path, RawQuery, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, patch};
use axum::Router;
use serde_json::to_value;
use sqlx::PgPool;

use crate::error::{json_response, ApiError};
use crate::routes::{internal, optional_param, qparams};
use crate::services::channels;
use crate::validate::{parse_json_body, validate_stream_update};

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/channels", get(list))
        .route("/channels/categories", get(categories))
        .route("/channels/by-uuid/{channel_uuid}", get(by_uuid))
        .route("/channels/{channel_id}", get(by_id))
        .route("/channels/{channel_id}/stream", patch(set_stream))
}

async fn list(State(pool): State<PgPool>, raw: RawQuery) -> Result<Response, ApiError> {
    let params = qparams(&raw);
    let category = optional_param(&params, "category").map(str::to_string);
    let q = optional_param(&params, "q").map(str::to_string);
    let rows = channels::list_channels(&pool, category.as_deref(), q.as_deref())
        .await
        .map_err(internal)?;
    Ok(json_response(StatusCode::OK, &to_value(rows).unwrap()))
}

async fn categories(State(pool): State<PgPool>) -> Result<Response, ApiError> {
    let rows = channels::list_categories(&pool).await.map_err(internal)?;
    Ok(json_response(StatusCode::OK, &to_value(rows).unwrap()))
}

async fn by_uuid(
    State(pool): State<PgPool>,
    Path(channel_uuid): Path<String>,
) -> Result<Response, ApiError> {
    let channel = channels::get_channel_by_uuid(&pool, &channel_uuid)
        .await
        .map_err(internal)?;
    match channel {
        Some(c) => Ok(json_response(StatusCode::OK, &to_value(c).unwrap())),
        None => Err(ApiError::not_found(format!(
            "Canal con uuid '{channel_uuid}' no encontrado"
        ))),
    }
}

async fn by_id(
    State(pool): State<PgPool>,
    Path(channel_id): Path<String>,
) -> Result<Response, ApiError> {
    let channel = channels::get_channel(&pool, &channel_id)
        .await
        .map_err(internal)?;
    match channel {
        Some(c) => Ok(json_response(StatusCode::OK, &to_value(c).unwrap())),
        None => Err(ApiError::not_found(format!(
            "Canal '{channel_id}' no encontrado"
        ))),
    }
}

async fn set_stream(
    State(pool): State<PgPool>,
    Path(channel_id): Path<String>,
    body: Bytes,
) -> Result<Response, ApiError> {
    let value = parse_json_body(&body).map_err(|e| ApiError::validation(vec![e]))?;
    let stream_url = validate_stream_update(value).map_err(ApiError::validation)?;
    let channel = channels::update_stream(&pool, &channel_id, stream_url)
        .await
        .map_err(internal)?;
    match channel {
        Some(c) => Ok(json_response(StatusCode::OK, &to_value(c).unwrap())),
        None => Err(ApiError::not_found(format!(
            "Canal '{channel_id}' no encontrado"
        ))),
    }
}
