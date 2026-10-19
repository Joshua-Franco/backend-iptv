use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct CustomSource {
    pub id: String,
    pub url: String,
    #[sqlx(rename = "type")]
    pub type_: String,
    pub status: String,
    pub error: Option<String>,
    pub channel_count: i32,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CustomChannel {
    pub id: String,
    pub uuid: String,
    pub source_id: String,
    pub name: String,
    pub url: String,
    pub logo_url: Option<String>,
    pub group_name: Option<String>,
    pub category: String,
    pub number: i32,
    pub initials: String,
    pub accent: String,
    pub is_direct: bool,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}
