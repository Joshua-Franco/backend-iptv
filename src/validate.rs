#![allow(clippy::result_large_err)]
use crate::error::PyErr;
use serde_json::{json, Value};

/// Parsea un query string con semántica de parse_qsl (keep_blank_values).
pub fn parse_query(raw: Option<&str>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(raw) = raw else { return out };
    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.find('=') {
            Some(i) => (&pair[..i], &pair[i + 1..]),
            None => (pair, ""),
        };
        out.push((percent_decode(k), percent_decode(v)));
    }
    out
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                (Some(a), Some(b)) => {
                    out.push(a * 16 + b);
                    i += 3;
                }
                _ => {
                    out.push(b'%');
                    i += 1;
                }
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Último valor para una clave (FastAPI/Starlette: last wins).
pub fn last_value<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .rev()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn int_parsing(key: &str, raw: &str) -> PyErr {
    PyErr::new(
        "int_parsing",
        vec![json!("query"), json!(key)],
        "Input should be a valid integer, unable to parse string as an integer",
        json!(raw),
    )
}

fn missing(key: &str) -> PyErr {
    PyErr::new(
        "missing",
        vec![json!("query"), json!(key)],
        "Field required",
        Value::Null,
    )
}

/// limit (ge=1, le=200, default 60), offset (ge=0, default 0), q (max_length=100).
pub fn parse_page_params(
    params: &[(String, String)],
    with_q: bool,
) -> Result<(i64, i64, Option<String>), Vec<PyErr>> {
    let mut errors = Vec::new();
    let mut limit: i64 = 60;
    let mut offset: i64 = 0;
    let mut q: Option<String> = None;

    if let Some(raw) = last_value(params, "limit") {
        match raw.parse::<i64>() {
            Ok(v) => {
                if v < 1 {
                    errors.push(
                        PyErr::new(
                            "greater_than_equal",
                            vec![json!("query"), json!("limit")],
                            "Input should be greater than or equal to 1",
                            json!(raw),
                        )
                        .with_ctx(json!({"ge": 1})),
                    );
                } else if v > 200 {
                    errors.push(
                        PyErr::new(
                            "less_than_equal",
                            vec![json!("query"), json!("limit")],
                            "Input should be less than or equal to 200",
                            json!(raw),
                        )
                        .with_ctx(json!({"le": 200})),
                    );
                } else {
                    limit = v;
                }
            }
            Err(_) => errors.push(int_parsing("limit", raw)),
        }
    }

    if let Some(raw) = last_value(params, "offset") {
        match raw.parse::<i64>() {
            Ok(v) => {
                if v < 0 {
                    errors.push(
                        PyErr::new(
                            "greater_than_equal",
                            vec![json!("query"), json!("offset")],
                            "Input should be greater than or equal to 0",
                            json!(raw),
                        )
                        .with_ctx(json!({"ge": 0})),
                    );
                } else {
                    offset = v;
                }
            }
            Err(_) => errors.push(int_parsing("offset", raw)),
        }
    }

    if with_q {
        if let Some(raw) = last_value(params, "q") {
            if raw.chars().count() > 100 {
                errors.push(
                    PyErr::new(
                        "string_too_long",
                        vec![json!("query"), json!("q")],
                        "String should have at most 100 characters",
                        json!(raw),
                    )
                    .with_ctx(json!({"max_length": 100})),
                );
            } else {
                q = Some(raw.to_string());
            }
        }
    }

    if errors.is_empty() {
        Ok((limit, offset, q))
    } else {
        Err(errors)
    }
}

/// Parámetro requerido tipo string en query.
pub fn required_query<'a>(
    params: &'a [(String, String)],
    key: &str,
) -> Result<&'a str, Vec<PyErr>> {
    last_value(params, key).ok_or_else(|| vec![missing(key)])
}

/// Parsea un body JSON con la misma forma de error que pydantic/FastAPI.
pub fn parse_json_body(bytes: &[u8]) -> Result<Value, PyErr> {
    if bytes.is_empty() {
        return Err(PyErr::new(
            "missing",
            vec![json!("body")],
            "Field required",
            Value::Null,
        ));
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(_) => return Err(json_invalid()),
    };
    serde_json::from_str::<Value>(text).map_err(|_| json_invalid())
}

fn json_invalid() -> PyErr {
    PyErr::new(
        "json_invalid",
        vec![json!("body"), json!(0)],
        "JSON decode error",
        json!({}),
    )
    .with_ctx(json!({"error": "Expecting value"}))
}

fn model_attributes_type(input: Value) -> PyErr {
    PyErr::new(
        "model_attributes_type",
        vec![json!("body")],
        "Input should be a valid dictionary or object to extract fields from",
        input,
    )
}

/// POST /custom/sources — { url: String, min_length=8, max_length=2000 }
pub fn validate_custom_source_in(value: Value) -> Result<String, Vec<PyErr>> {
    let obj = match value.as_object() {
        Some(_) => value,
        None => return Err(vec![model_attributes_type(value)]),
    };
    match obj.get("url") {
        None => Err(vec![PyErr::new(
            "missing",
            vec![json!("body"), json!("url")],
            "Field required",
            obj.clone(),
        )]),
        Some(Value::String(s)) => {
            let len = s.chars().count();
            if len < 8 {
                return Err(vec![PyErr::new(
                    "string_too_short",
                    vec![json!("body"), json!("url")],
                    "String should have at least 8 characters",
                    json!(s),
                )
                .with_ctx(json!({"min_length": 8}))]);
            }
            if len > 2000 {
                return Err(vec![PyErr::new(
                    "string_too_long",
                    vec![json!("body"), json!("url")],
                    "String should have at most 2000 characters",
                    json!(s),
                )
                .with_ctx(json!({"max_length": 2000}))]);
            }
            Ok(s.clone())
        }
        Some(other) => Err(vec![PyErr::new(
            "string_type",
            vec![json!("body"), json!("url")],
            "Input should be a valid string",
            other.clone(),
        )]),
    }
}

/// PATCH /api/channels/{id}/stream — { stream_url: str | None }
pub fn validate_stream_update(value: Value) -> Result<Option<String>, Vec<PyErr>> {
    let obj = match value.as_object() {
        Some(_) => value,
        None => return Err(vec![model_attributes_type(value)]),
    };
    match obj.get("stream_url") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(vec![PyErr::new(
            "string_type",
            vec![json!("body"), json!("stream_url")],
            "Input should be a valid string",
            other.clone(),
        )]),
    }
}
