use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Channel {
    pub id: String,
    pub uuid: String,
    pub number: i32,
    pub name: String,
    pub category: String,
    pub initials: String,
    pub accent: String,
    pub logo_file: String,
    pub description: Option<String>,
    pub stream_url: Option<String>,
    pub website: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}
