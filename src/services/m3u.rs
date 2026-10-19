use std::fmt;
use std::sync::LazyLock;

use regex::Regex;

use super::webfetch::{fetch_text, WebFetchError, DEFAULT_MAX_BYTES};

pub const MAX_PLAYLIST_ENTRIES: usize = 10_000;
pub const DIRECT_EXTENSIONS: &[&str] = &[".mp4", ".ts", ".mkv", ".webm", ".mov", ".avi", ".flv"];
pub const HLS_EXTENSIONS: &[&str] = &[".m3u8", ".m3u"];

static EXTINF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^#EXTINF:\s*-?[\d.]*\s*(?P<attrs>.*?),\s*(?P<name>.*)$").unwrap()
});
static ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"([A-Za-z0-9_-]+)="([^"]*)""#).unwrap());
static VLC_USER_AGENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^#EXTVLCOPT:\s*http-user-agent\s*=\s*(.+)$").unwrap());
static VLC_REFERRER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^#EXTVLCOPT:\s*http-referrer\s*=\s*(.+)$").unwrap());

#[derive(Debug)]
pub struct M3UParseError(pub String);

impl fmt::Display for M3UParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
pub struct Entry {
    pub name: String,
    pub url: String,
    pub logo: Option<String>,
    pub group: Option<String>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
}

pub fn url_path(url: &str) -> &str {
    let Some(idx) = url.find("://") else {
        let end = url.find(['?', '#']).unwrap_or(url.len());
        return &url[..end];
    };
    let rest = &url[idx + 3..];
    match rest.find('/') {
        Some(i) => {
            let after = &rest[i..];
            let end = after.find(['?', '#']).unwrap_or(after.len());
            &after[..end]
        }
        None => "",
    }
}

pub fn path_ext(url: &str) -> &'static str {
    let path = url_path(url).to_lowercase();
    for ext in DIRECT_EXTENSIONS.iter().chain(HLS_EXTENSIONS) {
        if path.ends_with(ext) {
            return ext;
        }
    }
    ""
}

fn entry_name(url: &str) -> String {
    let path = url_path(url);
    let path = path.trim_end_matches('/');
    let mut name = path.rsplit('/').next().unwrap_or("").to_string();
    for ext in [".m3u8", ".m3u", ".mp4", ".ts"] {
        if name.to_lowercase().ends_with(ext) {
            name.truncate(name.len() - ext.len());
        }
    }
    if name.is_empty() {
        super::hls::url_hostname(url).unwrap_or_else(|| "Canal".to_string())
    } else {
        name
    }
}

fn slice_chars(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

fn first_attr<'a>(
    attrs: &'a std::collections::HashMap<String, String>,
    key: &str,
) -> Option<&'a str> {
    attrs.get(key).map(String::as_str).filter(|v| !v.is_empty())
}

pub fn parse_entries(text: &str, base_url: &str) -> Vec<Entry> {
    use std::collections::HashMap;
    let mut entries: Vec<Entry> = Vec::new();
    let mut attrs: HashMap<String, String> = HashMap::new();
    let mut display_name = String::new();
    let mut group_name: Option<String> = None;
    let mut user_agent: Option<String> = None;
    let mut referrer: Option<String> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("#EXTINF:") {
            attrs.clear();
            display_name.clear();
            if let Some(caps) = EXTINF_RE.captures(line) {
                attrs = ATTR_RE
                    .captures_iter(&caps["attrs"])
                    .filter_map(|c| {
                        Some((
                            c.get(1)?.as_str().to_string(),
                            c.get(2)?.as_str().to_string(),
                        ))
                    })
                    .collect();
                display_name = caps["name"].trim().to_string();
            }
            group_name = attrs.get("group-title").cloned().filter(|v| !v.is_empty());
            user_agent = None;
            referrer = None;
            continue;
        }
        if line.to_uppercase().starts_with("#EXTVLCOPT:") {
            if let Some(caps) = VLC_USER_AGENT_RE.captures(line) {
                let v = caps[1].trim().to_string();
                user_agent = if v.is_empty() { None } else { Some(v) };
            }
            if let Some(caps) = VLC_REFERRER_RE.captures(line) {
                let v = caps[1].trim().to_string();
                referrer = if v.is_empty() { None } else { Some(v) };
            }
            continue;
        }
        if line.to_uppercase().starts_with("#EXTGRP:") {
            let v = line
                .split_once(':')
                .map(|x| x.1)
                .unwrap_or("")
                .trim()
                .to_string();
            if !v.is_empty() {
                group_name = Some(v);
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }

        let url = super::webfetch::join_url(base_url, line);
        let name = if !display_name.is_empty() {
            display_name.clone()
        } else if let Some(t) = first_attr(&attrs, "tvg-name") {
            t.to_string()
        } else {
            entry_name(&url)
        };
        let name: String = name.chars().take(255).collect();
        let logo = first_attr(&attrs, "logo")
            .or_else(|| first_attr(&attrs, "tvg-logo"))
            .map(str::to_string);
        let group = group_name.as_ref().map(|g| g.chars().take(160).collect());
        entries.push(Entry {
            name,
            url,
            logo,
            group,
            user_agent: user_agent.clone(),
            referrer: referrer.clone(),
        });
        display_name.clear();
        attrs.clear();
        group_name = None;
        if entries.len() >= MAX_PLAYLIST_ENTRIES {
            break;
        }
    }
    entries
}

fn stream_entry(name: String, url: String) -> Entry {
    Entry {
        name,
        url,
        logo: None,
        group: None,
        user_agent: None,
        referrer: None,
    }
}

/// Descarga el enlace y detecta si es lista M3U o un canal suelto.
/// Retorna ("playlist" | "stream", entries).
pub async fn prepare_source(url: &str) -> Result<(&'static str, Vec<Entry>), M3UParseError> {
    let url = url.trim();
    if url.is_empty() {
        return Err(M3UParseError("Pega un enlace M3U o .m3u8".into()));
    }
    let lower = url.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(M3UParseError(
            "El enlace debe empezar con http:// o https://".into(),
        ));
    }

    let ext = path_ext(url);
    let (text, content_type, final_url) = match fetch_text(url, DEFAULT_MAX_BYTES, 15).await {
        Ok(v) => v,
        Err(WebFetchError::Blocked(msg)) => return Err(M3UParseError(msg)),
        Err(WebFetchError::Other(msg)) => {
            if ext == ".m3u8" || DIRECT_EXTENSIONS.contains(&ext) {
                return Ok((
                    "stream",
                    vec![stream_entry(entry_name(url), url.to_string())],
                ));
            }
            return Err(M3UParseError(msg));
        }
    };

    let is_hls_tags = slice_chars(&text, 200_000).contains("#EXT-X-");
    let starts_m3u = text
        .trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n'])
        .get(..7)
        .map(|s| s.eq_ignore_ascii_case("#EXTM3U"))
        .unwrap_or(false);

    if is_hls_tags {
        return Ok((
            "stream",
            vec![stream_entry(entry_name(&final_url), final_url)],
        ));
    }

    if starts_m3u || slice_chars(&text, 200_000).contains("#EXTINF") {
        let entries = parse_entries(&text, &final_url);
        if !entries.is_empty() {
            return Ok(("playlist", entries));
        }
        if HLS_EXTENSIONS.contains(&ext) {
            return Ok((
                "stream",
                vec![stream_entry(entry_name(&final_url), final_url)],
            ));
        }
        return Err(M3UParseError("La lista M3U esta vacia".into()));
    }

    if HLS_EXTENSIONS.contains(&ext) || DIRECT_EXTENSIONS.contains(&ext) {
        return Ok((
            "stream",
            vec![stream_entry(entry_name(&final_url), final_url)],
        ));
    }

    if content_type.to_lowercase().contains("mpegurl") {
        return Ok((
            "stream",
            vec![stream_entry(entry_name(&final_url), final_url)],
        ));
    }

    Err(M3UParseError(
        "Formato no reconocido: solo se aceptan listas M3U y enlaces .m3u8".into(),
    ))
}
