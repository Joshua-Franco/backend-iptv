use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::to_value;
use sqlx::PgPool;

use crate::error::{json_response, ApiError};
use crate::routes::internal;
use crate::schemas::HealthOut;

pub fn router() -> Router<PgPool> {
    Router::new().route("/health", get(health))
}

async fn health(State(pool): State<PgPool>) -> Result<Response, ApiError> {
    let channels: i64 = sqlx::query_scalar("SELECT count(*) FROM channels")
        .fetch_one(&pool)
        .await
        .map_err(internal)?;
    let iptv: i64 = sqlx::query_scalar("SELECT count(*) FROM iptv_channels")
        .fetch_one(&pool)
        .await
        .map_err(internal)?;
    let custom: i64 = sqlx::query_scalar("SELECT count(*) FROM custom_channels")
        .fetch_one(&pool)
        .await
        .map_err(internal)?;
    let out = HealthOut {
        status: "ok",
        database: "connected",
        channels,
        iptv,
        custom,
    };
    Ok(json_response(
        axum::http::StatusCode::OK,
        &to_value(out).unwrap(),
    ))
}
