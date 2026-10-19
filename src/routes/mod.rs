pub mod channels;
pub mod custom;
pub mod health;
pub mod iptv;
pub mod stream;

use axum::extract::RawQuery;

use crate::error::ApiError;
use crate::validate::{last_value, parse_query};

pub fn qparams(raw: &RawQuery) -> Vec<(String, String)> {
    parse_query(raw.0.as_deref())
}

pub fn optional_param<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    last_value(params, key)
}

pub fn internal(err: sqlx::Error) -> ApiError {
    let _ = err;
    ApiError::Http {
        status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        detail: "Internal Server Error".into(),
    }
}
