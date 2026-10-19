use std::time::{Duration, Instant};

use dashmap::DashMap;
use std::sync::LazyLock;

use super::webfetch::{fetch_bytes, WebFetchError};
use crate::error::ApiError;

const TTL: Duration = Duration::from_secs(6 * 3600);
const MAX_ENTRIES: usize = 2000;
pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

type ImageCacheEntry = (Instant, Vec<u8>, String);
static CACHE: LazyLock<DashMap<String, ImageCacheEntry>> = LazyLock::new(DashMap::new);

fn sniff_image_type(content_type: &str, data: &[u8]) -> String {
    let ctype = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if ctype.starts_with("image/") {
        return ctype;
    }
    if data.starts_with(&[0x89, b'P', b'N', b'G']) {
        return "image/png".into();
    }
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "image/jpeg".into();
    }
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return "image/gif".into();
    }
    let head = &data[..data.len().min(1000)];
    if String::from_utf8_lossy(head)
        .to_lowercase()
        .contains("<svg")
    {
        return "image/svg+xml".into();
    }
    if data.starts_with(b"RIFF") && data.len() >= 12 && &data[8..12] == b"WEBP" {
        return "image/webp".into();
    }
    "image/png".into()
}

pub async fn get_image(url: Option<&str>) -> Result<(Vec<u8>, String), ApiError> {
    let Some(url) = url else {
        return Err(ApiError::not_found("Este canal no tiene imagen"));
    };
    if let Some(entry) = CACHE.get(url) {
        let (ts, data, ctype) = entry.value();
        if ts.elapsed() < TTL {
            return Ok((data.clone(), ctype.clone()));
        }
    }
    let fetched = fetch_bytes(url, MAX_IMAGE_BYTES, 10).await;
    let (data, content_type, _) = match fetched {
        Ok(v) => v,
        Err(WebFetchError::Blocked(msg)) | Err(WebFetchError::Other(msg)) => {
            return Err(ApiError::bad_gateway(format!(
                "No se pudo obtener la imagen: {msg}"
            )));
        }
    };
    let image_type = sniff_image_type(&content_type, &data);
    if CACHE.len() >= MAX_ENTRIES {
        let oldest = CACHE
            .iter()
            .min_by_key(|e| e.value().0)
            .map(|e| e.key().clone());
        if let Some(key) = oldest {
            CACHE.remove(&key);
        }
    }
    CACHE.insert(
        url.to_string(),
        (Instant::now(), data.clone(), image_type.clone()),
    );
    Ok((data, image_type))
}
