use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Carga .env desde la raiz del proyecto con la misma tolerancia que
/// python-dotenv (comillas opcionales, tolerante a espacios sin comillas,
/// sin pisar variables de entorno ya definidas).
pub fn load_dotenv() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir.clone();
    let path = if candidate.join(".env").exists() {
        candidate.join(".env")
    } else if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent();
        let mut found = None;
        for _ in 0..4 {
            if let Some(p) = cur {
                let check = p.join(".env");
                if check.exists() {
                    found = Some(check);
                    break;
                }
                cur = p.parent();
            }
        }
        found.unwrap_or_else(|| manifest_dir.join(".env"))
    } else {
        manifest_dir.join(".env")
    };

    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim();
        let value = if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };
        if std::env::var(key).is_err() {
            std::env::set_var(key, value);
        }
    }
}

pub static PROJECT_NAME: LazyLock<String> =
    LazyLock::new(|| std::env::var("PROJECT_NAME").unwrap_or_else(|_| "IPTV Peru API".to_string()));

pub static API_PREFIX: LazyLock<String> =
    LazyLock::new(|| std::env::var("API_PREFIX").unwrap_or_else(|_| "/api".to_string()));

pub fn cors_wildcard() -> bool {
    std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| "*".to_string())
        .split(',')
        .map(|o| o.trim())
        .any(|o| o == "*")
}

pub fn host() -> String {
    std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string())
}

pub fn port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000)
}

pub fn database_url() -> String {
    let url = std::env::var("DATABASE_URL").unwrap_or_default();
    if url.is_empty() {
        panic!("DATABASE_URL no esta definida en el archivo .env");
    }
    normalize_db_url(&url)
}

pub fn logos_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("static")
        .join("logos")
}

pub fn normalize_db_url(url: &str) -> String {
    let url = if let Some(rest) = url.strip_prefix("postgres://") {
        format!("postgresql://{rest}")
    } else {
        url.to_string()
    };
    let url = if let Some(rest) = url.strip_prefix("postgresql+psycopg://") {
        format!("postgresql://{rest}")
    } else {
        url
    };
    // sqlx no reconoce channel_binding; lo descartamos.
    if let Some(idx) = url.find('?') {
        let (base, query) = url.split_at(idx);
        let query = &query[1..];
        let kept: Vec<&str> = query
            .split('&')
            .filter(|p| !p.starts_with("channel_binding"))
            .collect();
        if kept.is_empty() {
            base.to_string()
        } else {
            format!("{base}?{}", kept.join("&"))
        }
    } else {
        url
    }
}
