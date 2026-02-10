use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};

/// Single log event for ingestion.
#[derive(Debug, Deserialize)]
pub struct IngestEvent {
    #[serde(default = "default_ts")]
    pub ts: DateTimeWithTimeZone,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub sourcetype: Option<String>,
    #[serde(default)]
    pub severity: Option<i32>,
    pub message: String,
    #[serde(default)]
    pub fields: serde_json::Value,
}

/// Request payload for ingesting multiple events.
#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub app_id: String,
    pub events: Vec<IngestEvent>,
}

/// Response after ingesting events.
#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub accepted: usize,
}

/// Request payload for creating a new application.
#[derive(Debug, Deserialize)]
pub struct CreateAppRequest {
    pub name: String,
}

/// Application summary for UI usage.
#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub app_id: String,
    pub name: String,
    pub created_at: DateTimeWithTimeZone,
}

/// Request payload for searching logs.
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub app_id: String,
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub sources: Option<Vec<String>>,
    #[serde(default)]
    pub hosts: Option<Vec<String>>,
    #[serde(default)]
    pub severities: Option<Vec<i32>>,
    #[serde(default)]
    pub start_ts: Option<DateTimeWithTimeZone>,
    #[serde(default)]
    pub end_ts: Option<DateTimeWithTimeZone>,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

/// Single search result item.
#[derive(Debug, Serialize)]
pub struct SearchItem {
    pub id: i64,
    pub app_id: String,
    pub ts: DateTimeWithTimeZone,
    pub host: String,
    pub source: String,
    pub sourcetype: Option<String>,
    pub severity: Option<i32>,
    pub message: String,
    pub fields: serde_json::Value,
}

/// Search response with total count and items.
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub total: u64,
    pub items: Vec<SearchItem>,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

/// Request payload for creating a log source.
#[derive(Debug, Deserialize)]
pub struct CreateSourceRequest {
    pub app_id: String,
    pub kind: String,
    pub path: String,
    #[serde(default)]
    pub recursive: Option<bool>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub include_glob: Option<String>,
    #[serde(default)]
    pub exclude_glob: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// Request payload for updating a log source.
#[derive(Debug, Deserialize)]
pub struct UpdateSourceRequest {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub recursive: Option<bool>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub include_glob: Option<String>,
    #[serde(default)]
    pub exclude_glob: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// Source information for API responses.
#[derive(Debug, Serialize)]
pub struct SourceInfo {
    pub id: i64,
    pub app_id: String,
    pub kind: String,
    pub path: String,
    pub recursive: bool,
    pub encoding: String,
    pub include_glob: Option<String>,
    pub exclude_glob: Option<String>,
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
}

fn default_ts() -> DateTimeWithTimeZone {
    use chrono::{FixedOffset, Utc};
    let utc = Utc::now();
    utc.with_timezone(&FixedOffset::east_opt(0).unwrap())
}

fn default_limit() -> u64 {
    100
}
