use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::header::{HeaderName, HeaderValue, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{delete, get};
use axum::Router;
use serde_json::to_value;
use sqlx::PgPool;

use crate::error::{json_response, ApiError};
use crate::models::CustomSource;
use crate::routes::internal;
use crate::schemas::CustomSourceOut;
use crate::services::images;
use crate::services::m3u::{self, DIRECT_EXTENSIONS};
use crate::services::palette::{accent_for, make_initials};
use crate::validate::{parse_json_body, validate_custom_source_in};

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/custom/sources", get(list_sources).post(add_source))
        .route("/custom/sources/{source_id}", delete(remove_source))
        .route("/custom/channels/{channel_id}/logo", get(logo))
}

fn is_direct(url: &str) -> bool {
    let ext = m3u::path_ext(url);
    DIRECT_EXTENSIONS.contains(&ext)
}

fn to_out(row: &CustomSource, already: bool) -> CustomSourceOut {
    CustomSourceOut {
        id: row.id.clone(),
        url: row.url.clone(),
        type_: row.type_.clone(),
        status: row.status.clone(),
        error: row.error.clone(),
        channel_count: row.channel_count,
        created_at: row.created_at,
        already,
    }
}

async fn fetch_source_by_id(pool: &PgPool, id: &str) -> Result<CustomSource, sqlx::Error> {
    sqlx::query_as::<_, CustomSource>("SELECT * FROM custom_sources WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

async fn list_sources(State(pool): State<PgPool>) -> Result<Response, ApiError> {
    let rows: Vec<CustomSource> = sqlx::query_as::<_, CustomSource>(
        "SELECT * FROM custom_sources ORDER BY created_at DESC, id",
    )
    .fetch_all(&pool)
    .await
    .map_err(internal)?;
    let out: Vec<CustomSourceOut> = rows.iter().map(|r| to_out(r, false)).collect();
    Ok(json_response(StatusCode::OK, &to_value(out).unwrap()))
}

async fn add_source(State(pool): State<PgPool>, body: Bytes) -> Result<Response, ApiError> {
    let value = parse_json_body(&body).map_err(|e| ApiError::validation(vec![e]))?;
    let raw_url = validate_custom_source_in(value).map_err(ApiError::validation)?;
    let url = raw_url.trim().to_string();

    let existing: Option<CustomSource> =
        sqlx::query_as::<_, CustomSource>("SELECT * FROM custom_sources WHERE url = $1")
            .bind(&url)
            .fetch_optional(&pool)
            .await
            .map_err(internal)?;

    if let Some(s) = &existing {
        if s.status == "active" {
            return Ok(json_response(
                StatusCode::OK,
                &to_value(to_out(s, true)).unwrap(),
            ));
        }
    }

    let prepared = m3u::prepare_source(&url).await;
    let (kind, entries) = match prepared {
        Ok(v) => v,
        Err(e) => {
            let msg = e.to_string();
            let source = if let Some(mut s) = existing {
                sqlx::query(
                    "UPDATE custom_sources SET type = 'playlist', status = 'error', error = $1, channel_count = 0 WHERE id = $2",
                )
                .bind(&msg)
                .bind(&s.id)
                .execute(&pool)
                .await
                .map_err(internal)?;
                s.status = "error".into();
                s.type_ = "playlist".into();
                s.error = Some(msg);
                s.channel_count = 0;
                s
            } else {
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO custom_sources (id, url, type, status, error, channel_count) VALUES ($1, $2, 'playlist', 'error', $3, 0)",
                )
                .bind(&id)
                .bind(&url)
                .bind(&msg)
                .execute(&pool)
                .await
                .map_err(internal)?;
                fetch_source_by_id(&pool, &id).await.map_err(internal)?
            };
            return Ok(json_response(
                StatusCode::OK,
                &to_value(to_out(&source, false)).unwrap(),
            ));
        }
    };

    let source_id = if let Some(s) = existing {
        sqlx::query("DELETE FROM custom_channels WHERE source_id = $1")
            .bind(&s.id)
            .execute(&pool)
            .await
            .map_err(internal)?;
        s.id
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO custom_sources (id, url, type, status, channel_count) VALUES ($1, $2, $3, 'active', 0)",
        )
        .bind(&id)
        .bind(&url)
        .bind(kind)
        .execute(&pool)
        .await
        .map_err(internal)?;
        id
    };

    let max_num: Option<i32> =
        sqlx::query_scalar("SELECT max(number) FROM custom_channels WHERE number > 0")
            .fetch_one(&pool)
            .await
            .map_err(internal)?;
    let start_num = max_num.unwrap_or(99) + 1;

    for (number, entry) in (start_num..).zip(entries.iter()) {
        let cid = uuid::Uuid::new_v4().to_string();
        let cuuid = uuid::Uuid::new_v4().to_string();
        let initials = make_initials(&entry.name);
        let accent = accent_for(&entry.url);
        let direct = is_direct(&entry.url);
        let category = entry
            .group
            .clone()
            .unwrap_or_else(|| "Personalizado".to_string());
        sqlx::query(
            "INSERT INTO custom_channels (id, uuid, source_id, name, url, logo_url, group_name, category, number, initials, accent, is_direct, user_agent, referrer)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(&cid)
        .bind(&cuuid)
        .bind(&source_id)
        .bind(&entry.name)
        .bind(&entry.url)
        .bind(&entry.logo)
        .bind(&entry.group)
        .bind(&category)
        .bind(number)
        .bind(&initials)
        .bind(&accent)
        .bind(direct)
        .bind(&entry.user_agent)
        .bind(&entry.referrer)
        .execute(&pool)
        .await
        .map_err(internal)?;
    }

    sqlx::query(
        "UPDATE custom_sources SET type = $1, status = 'active', error = NULL, channel_count = $2 WHERE id = $3",
    )
    .bind(kind)
    .bind(entries.len() as i32)
    .bind(&source_id)
    .execute(&pool)
    .await
    .map_err(internal)?;

    let final_row = fetch_source_by_id(&pool, &source_id)
        .await
        .map_err(internal)?;
    Ok(json_response(
        StatusCode::OK,
        &to_value(to_out(&final_row, false)).unwrap(),
    ))
}

async fn remove_source(
    State(pool): State<PgPool>,
    Path(source_id): Path<String>,
) -> Result<Response, ApiError> {
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM custom_sources WHERE id = $1")
        .bind(&source_id)
        .fetch_optional(&pool)
        .await
        .map_err(internal)?;
    if exists.is_none() {
        return Err(ApiError::not_found("Enlace no encontrado"));
    }
    sqlx::query("DELETE FROM custom_sources WHERE id = $1")
        .bind(&source_id)
        .execute(&pool)
        .await
        .map_err(internal)?;
    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .expect("response builder"))
}

async fn logo(
    State(pool): State<PgPool>,
    Path(channel_id): Path<String>,
) -> Result<Response, ApiError> {
    let logo_url: Option<Option<String>> =
        sqlx::query_scalar("SELECT logo_url FROM custom_channels WHERE id = $1")
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
