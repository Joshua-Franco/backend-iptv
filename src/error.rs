use axum::body::Body;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::{json, Value};

/// Error de validación con la forma exacta de pydantic v2 (FastAPI).
#[derive(Serialize)]
pub struct PyErr {
    #[serde(rename = "type")]
    pub type_: String,
    pub loc: Vec<Value>,
    pub msg: String,
    pub input: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<Value>,
}

impl PyErr {
    pub fn new(type_: &str, loc: Vec<Value>, msg: &str, input: Value) -> Self {
        Self {
            type_: type_.to_string(),
            loc,
            msg: msg.to_string(),
            input,
            ctx: None,
        }
    }

    pub fn with_ctx(mut self, ctx: Value) -> Self {
        self.ctx = Some(ctx);
        self
    }
}

pub enum ApiError {
    Http { status: StatusCode, detail: String },
    Validation(Vec<PyErr>),
}

impl ApiError {
    pub fn not_found(detail: impl Into<String>) -> Self {
        ApiError::Http {
            status: StatusCode::NOT_FOUND,
            detail: detail.into(),
        }
    }

    pub fn forbidden(detail: impl Into<String>) -> Self {
        ApiError::Http {
            status: StatusCode::FORBIDDEN,
            detail: detail.into(),
        }
    }

    pub fn bad_gateway(detail: impl Into<String>) -> Self {
        ApiError::Http {
            status: StatusCode::BAD_GATEWAY,
            detail: detail.into(),
        }
    }

    pub fn validation(errors: Vec<PyErr>) -> Self {
        ApiError::Validation(errors)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Http { status, detail } => {
                json_response(status, &json!({ "detail": detail }))
            }
            ApiError::Validation(errors) => json_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                &json!({ "detail": errors }),
            ),
        }
    }
}

pub fn json_response(status: StatusCode, value: &Value) -> Response {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(bytes))
        .unwrap()
}
