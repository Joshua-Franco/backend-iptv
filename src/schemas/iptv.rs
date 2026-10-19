use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::channel::ChannelOut;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IptvCategoryOut {
    pub code: String,
    pub name: String,
    pub name_en: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPageOut {
    pub items: Vec<ChannelOut>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSourceIn {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSourceOut {
    pub id: String,
    pub url: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub status: String,
    pub error: Option<String>,
    pub channel_count: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub already: bool,
}
