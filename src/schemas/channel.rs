use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelOut {
    pub id: String,
    pub uuid: String,
    pub number: i32,
    pub name: String,
    pub category: String,
    pub initials: String,
    pub accent: String,
    pub logo_url: String,
    pub description: Option<String>,
    pub stream_url: Option<String>,
    pub website: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryOut {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamUpdate {
    pub stream_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthOut {
    pub status: &'static str,
    pub database: &'static str,
    pub channels: i64,
    pub iptv: i64,
    pub custom: i64,
}
