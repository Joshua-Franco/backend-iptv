use std::collections::HashSet;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use futures_util::StreamExt;
use std::sync::LazyLock;

use super::webfetch::err_class;
use crate::error::ApiError;

pub static BROWSER_HEADERS: LazyLock<[(&'static str, &'static str); 2]> = LazyLock::new(|| {
    [
        (
            "User-Agent",
            "Mozilla/5.0 (SMART-TV; Linux; Tizen 6.5) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36",
        ),
        ("Accept", "*/*"),
    ]
});

static CACHE_TTL: Duration = Duration::from_secs(3);

static PLAYLIST_CACHE: LazyLock<DashMap<String, (Instant, String, String)>> =
    LazyLock::new(DashMap::new);

static ALLOWED_HOSTS: LazyLock<DashMap<String, HashSet<String>>> = LazyLock::new(DashMap::new);

static CHANNEL_HEADERS: LazyLock<DashMap<String, Vec<(String, String)>>> =
    LazyLock::new(DashMap::new);

pub fn set_channel_headers(channel_id: &str, headers: &[(String, String)]) {
    let filtered: Vec<(String, String)> = headers
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    CHANNEL_HEADERS.insert(channel_id.to_string(), filtered);
}

fn headers_for(channel_id: &str) -> reqwest::header::HeaderMap {
    let mut map: Vec<(String, String)> = BROWSER_HEADERS
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    if let Some(extra) = CHANNEL_HEADERS.get(channel_id) {
        for (k, v) in extra.iter() {
            map.retain(|(ek, _)| ek != k);
            map.push((k.clone(), v.clone()));
        }
    }
    let mut out = reqwest::header::HeaderMap::new();
    for (k, v) in map {
        if let (Ok(name), Ok(value)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(&v),
        ) {
            out.insert(name, value);
        }
    }
    out
}

fn register_host(channel_id: &str, url: &str) {
    if let Some(host) = url_hostname(url) {
        ALLOWED_HOSTS
            .entry(channel_id.to_string())
            .or_default()
            .insert(host);
    }
}

fn is_allowed(channel_id: &str, url: &str) -> bool {
    match url_hostname(url) {
        None => false,
        Some(host) => ALLOWED_HOSTS
            .get(channel_id)
            .map(|s| s.contains(&host))
            .unwrap_or(false),
    }
}

/// hostname estilo urlparse: minusculas, sin puerto/userinfo.
pub fn url_hostname(url: &str) -> Option<String> {
    let idx = url.find("://")?;
    let rest = &url[idx + 3..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let hostpart = &rest[..end];
    let hostpart = match hostpart.rfind('@') {
        Some(i) => &hostpart[i + 1..],
        None => hostpart,
    };
    let host = if hostpart.starts_with('[') {
        let close = hostpart.find(']')?;
        &hostpart[..=close]
    } else {
        match hostpart.find(':') {
            Some(i) => &hostpart[..i],
            None => hostpart,
        }
    };
    if host.is_empty() {
        None
    } else {
        Some(host.to_lowercase())
    }
}

/// quote(url, safe='') — escapa todo salvo A-Za-z0-9_.-~
pub fn percent_encode_full(url: &str) -> String {
    let mut out = String::with_capacity(url.len());
    for &b in url.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn build_direct_playlist(channel_id: &str, url: &str, prefix: &str) -> String {
    register_host(channel_id, url);
    let target = format!(
        "{prefix}/stream/{channel_id}/asset?u={}",
        percent_encode_full(url)
    );
    [
        "#EXTM3U",
        "#EXT-X-VERSION:3",
        "#EXT-X-TARGETDURATION:9999",
        "#EXT-X-MEDIA-SEQUENCE:0",
        "#EXTINF:9999.0,",
        target.as_str(),
        "#EXT-X-ENDLIST",
    ]
    .join("\n")
}

fn rewrite(playlist: &str, base_url: &str, channel_id: &str, prefix: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for raw in playlist.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            lines.push(raw.to_string());
            continue;
        }
        let absolute = super::webfetch::join_url(base_url, line);
        register_host(channel_id, &absolute);
        let kind = if absolute.to_lowercase().contains(".m3u8") {
            "variant"
        } else {
            "asset"
        };
        lines.push(format!(
            "{prefix}/stream/{channel_id}/{kind}?u={}",
            percent_encode_full(&absolute)
        ));
    }
    lines.join("\n")
}

async fn fetch_text(url: &str, channel_id: &str) -> Result<(String, String), ApiError> {
    if let Some(entry) = PLAYLIST_CACHE.get(url) {
        let (ts, text, ct) = entry.value();
        if ts.elapsed() < CACHE_TTL {
            return Ok((text.clone(), ct.clone()));
        }
    }
    let headers = headers_for(channel_id);
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("reqwest client");
    let response = client
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| {
            ApiError::bad_gateway(format!("No se pudo obtener la senal: {}", err_class(&e)))
        })?;
    let status = response.status().as_u16();
    if status >= 400 {
        return Err(ApiError::bad_gateway(format!(
            "El origen respondio {status}"
        )));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/vnd.apple.mpegurl")
        .to_string();
    let bytes = response.bytes().await.map_err(|e| {
        ApiError::bad_gateway(format!("No se pudo obtener la senal: {}", err_class(&e)))
    })?;
    let text = super::webfetch::decode_text(&bytes);
    PLAYLIST_CACHE.insert(
        url.to_string(),
        (Instant::now(), text.clone(), content_type.clone()),
    );
    Ok((text, content_type))
}

pub async fn get_playlist(channel_id: &str, url: &str, prefix: &str) -> Result<String, ApiError> {
    register_host(channel_id, url);
    let (text, _) = fetch_text(url, channel_id).await?;
    Ok(rewrite(&text, url, channel_id, prefix))
}

pub fn validate_proxy_url(channel_id: &str, url: &str) -> Result<(), ApiError> {
    if is_allowed(channel_id, url) {
        Ok(())
    } else {
        Err(ApiError::forbidden("Host del recurso no permitido"))
    }
}

pub async fn proxy_asset(
    channel_id: &str,
    url: &str,
) -> Result<(String, reqwest::Response), ApiError> {
    if !is_allowed(channel_id, url) {
        return Err(ApiError::forbidden("Host del recurso no permitido"));
    }
    let headers = headers_for(channel_id);
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("reqwest client");
    let response = client
        .get(url)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| ApiError::bad_gateway("No se pudo obtener el recurso"))?;
    let status = response.status().as_u16();
    if status >= 400 {
        return Err(ApiError::bad_gateway(format!(
            "El origen respondio {status}"
        )));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    Ok((content_type, response))
}

pub fn asset_body_stream(response: reqwest::Response) -> axum::body::Body {
    let stream = response
        .bytes_stream()
        .map(|item| item.map_err(|e| std::io::Error::other(e.to_string())));
    axum::body::Body::from_stream(stream)
}
