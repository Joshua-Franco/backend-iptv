pub mod config;
pub mod database;
pub mod error;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod schemas;
pub mod services;
pub mod validate;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::json;
use sqlx::PgPool;
use tower_http::services::ServeDir;

use crate::error::json_response;

pub async fn fallback_404() -> Response {
    json_response(StatusCode::NOT_FOUND, &json!({ "detail": "Not Found" }))
}

async fn root(State(_pool): State<PgPool>) -> Response {
    json_response(
        StatusCode::OK,
        &json!({
            "name": *config::PROJECT_NAME,
            "docs": "/docs",
            "health": format!("{}/health", *config::API_PREFIX),
            "channels": format!("{}/channels", *config::API_PREFIX),
        }),
    )
}

pub fn build_router(pool: PgPool) -> Router {
    let api = routes::channels::router()
        .merge(routes::custom::router())
        .merge(routes::health::router())
        .merge(routes::iptv::router())
        .merge(routes::stream::router());

    let static_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static");
    let static_fallback = tower::service_fn(|_req: axum::extract::Request| async {
        Ok::<_, std::convert::Infallible>(fallback_404().await)
    });

    Router::new()
        .route("/", get(root))
        .nest(config::API_PREFIX.as_str(), api)
        .nest_service(
            "/static",
            ServeDir::new(static_root).fallback(static_fallback),
        )
        .fallback(fallback_404)
        .layer(axum::middleware::from_fn(middleware::global_middleware))
        .with_state(pool)
}

pub fn build_app(pool: PgPool) -> middleware::OuterService<Router> {
    let router = build_router(pool);
    middleware::OuterService::new(router)
}
