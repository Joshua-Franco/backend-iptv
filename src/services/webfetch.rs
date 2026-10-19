use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use super::hls::BROWSER_HEADERS;
use super::ipcheck::is_global;

pub const MAX_REDIRECTS: usize = 5;
pub const DEFAULT_MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug)]
pub enum WebFetchError {
    Blocked(String),
    Other(String),
}

impl fmt::Display for WebFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebFetchError::Blocked(m) | WebFetchError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl WebFetchError {
    pub fn blocked(msg: &str) -> Self {
        WebFetchError::Blocked(msg.to_string())
    }

    pub fn other(msg: &str) -> Self {
        WebFetchError::Other(msg.to_string())
    }
}

fn split_url(url: &str) -> Option<(&str, &str, &str)> {
    let idx = url.find("://")?;
    let scheme = &url[..idx];
    let rest = &url[idx + 3..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = &rest[..end];
    Some((scheme, host, &rest[end..]))
}

/// Replica de assert_public_url (SSRF).
pub fn assert_public_url(url: &str) -> Result<(), WebFetchError> {
    let Some((scheme, hostpart, _)) = split_url(url) else {
        return Err(WebFetchError::blocked("Solo se aceptan enlaces http/https"));
    };
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(WebFetchError::blocked("Solo se aceptan enlaces http/https"));
    }
    let hostpart = match hostpart.rfind('@') {
        Some(i) => &hostpart[i + 1..],
        None => hostpart,
    };
    let host = if hostpart.starts_with('[') {
        let close = hostpart.find(']').unwrap_or(hostpart.len());
        &hostpart[..=close]
    } else {
        match hostpart.find(':') {
            Some(i) => &hostpart[..i],
            None => hostpart,
        }
    };
    let host = host.to_lowercase();
    if host.is_empty() {
        return Err(WebFetchError::blocked("El enlace no tiene un host valido"));
    }
    let port: u16 = explicit_port(hostpart).unwrap_or(if scheme == "https" { 443 } else { 80 });

    let addrs: Vec<SocketAddr> = match (host.as_str(), port).to_socket_addrs() {
        Ok(it) => it.collect(),
        Err(_) => {
            return Err(WebFetchError::other(
                "No se pudo resolver el dominio del enlace",
            ));
        }
    };
    for addr in addrs {
        if !is_global(addr.ip()) {
            return Err(WebFetchError::blocked(
                "El enlace apunta a una red interna y fue bloqueada",
            ));
        }
    }
    Ok(())
}

fn explicit_port(hostpart: &str) -> Option<u16> {
    if let Some(rest) = hostpart.strip_prefix('[') {
        let close = rest.find(']')?;
        let after = &rest[close + 1..];
        after.strip_prefix(':')?.parse().ok()
    } else {
        let colon = hostpart.find(':')?;
        hostpart[colon + 1..].parse().ok()
    }
}

fn client() -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    for (k, v) in BROWSER_HEADERS.iter() {
        if let (Ok(name), Ok(value)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(v),
        ) {
            headers.insert(name, value);
        }
    }
    reqwest::Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client")
}

/// Descarga validando cada salto contra SSRF. Retorna (contenido, content_type, url_final).
pub async fn fetch_bytes(
    url: &str,
    max_bytes: usize,
    timeout_secs: u64,
) -> Result<(Vec<u8>, String, String), WebFetchError> {
    let client = client();
    let mut current = url.to_string();
    let mut response: Option<reqwest::Response> = None;
    for i in 0..=MAX_REDIRECTS {
        assert_public_url(&current)?;
        let resp = client
            .get(&current)
            .timeout(Duration::from_secs(timeout_secs))
            .send()
            .await
            .map_err(|e| {
                WebFetchError::other(&format!(
                    "No se pudo conectar con el enlace ({})",
                    err_class(&e)
                ))
            })?;
        response = Some(resp);
        let status = response.as_ref().expect("response").status().as_u16();
        if matches!(status, 301 | 302 | 303 | 307 | 308) {
            let location = response
                .as_ref()
                .expect("response")
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let Some(location) = location else {
                return Err(WebFetchError::other("El enlace redirige sin destino"));
            };
            if i == MAX_REDIRECTS {
                // Como httpx: tras el ultimo salto se conserva la respuesta final.
                break;
            }
            current = join_url(&current, &location);
            continue;
        }
        break;
    }
    let Some(resp) = response else {
        return Err(WebFetchError::other("El enlace respondio ?"));
    };
    let status = resp.status().as_u16();
    if status >= 400 {
        return Err(WebFetchError::other(&format!(
            "El enlace respondio {status}"
        )));
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let final_url = resp.url().to_string();

    let mut chunks: Vec<u8> = Vec::new();
    let mut stream = resp.bytes_stream();
    use futures_util::StreamExt;
    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| {
            WebFetchError::other(&format!("Error leyendo la respuesta ({})", err_class(&e)))
        })?;
        if chunks.len() + chunk.len() > max_bytes {
            return Err(WebFetchError::other("El enlace trae demasiados datos"));
        }
        chunks.extend_from_slice(&chunk);
    }
    Ok((chunks, content_type, final_url))
}

pub fn decode_text(raw: &[u8]) -> String {
    match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => raw.iter().map(|&b| b as char).collect(),
    }
}

pub async fn fetch_text(
    url: &str,
    max_bytes: usize,
    timeout_secs: u64,
) -> Result<(String, String, String), WebFetchError> {
    let (raw, content_type, final_url) = fetch_bytes(url, max_bytes, timeout_secs).await?;
    Ok((decode_text(&raw), content_type, final_url))
}

/// Clase de error estilo httpx para mensajes de usuario.
pub fn err_class(e: &reqwest::Error) -> &'static str {
    if e.is_timeout() {
        if e.is_connect() {
            "ConnectTimeout"
        } else {
            "ReadTimeout"
        }
    } else if e.is_connect() {
        "ConnectError"
    } else if e.is_request() {
        "RequestError"
    } else {
        "HTTPError"
    }
}

/// urljoin a nivel de cadena.
pub fn join_url(base: &str, rel: &str) -> String {
    if rel.is_empty() {
        return base.to_string();
    }
    if let Ok(b) = url::Url::parse(base) {
        if let Ok(j) = b.join(rel) {
            return j.to_string();
        }
    }
    rel.to_string()
}
