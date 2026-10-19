use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct IptvCategory {
    pub code: String,
    pub name_en: String,
    pub name_es: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct IptvChannel {
    pub id: String,
    pub uuid: String,
    pub name: String,
    pub category: String,
    pub categories: String,
    pub website: Option<String>,
    pub is_nsfw: bool,
    pub accent: String,
    pub initials: String,
    pub number: i32,
    pub stream_url: Option<String>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub logo_url: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct IptvStream {
    pub id: i32,
    pub channel_id: String,
    pub feed: Option<String>,
    pub title: String,
    pub url: String,
    pub quality: Option<String>,
    pub labels: String,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct IptvLogo {
    pub id: i32,
    pub channel_id: String,
    pub feed: Option<String>,
    pub url: String,
    pub format: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub in_use: bool,
    pub is_primary: bool,
}
