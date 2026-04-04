use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A raw item produced by a collector before intelligence processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawItem {
    pub source: String,
    pub collector_id: String,
    pub title: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub metadata: serde_json::Value,
    pub published_at: Option<DateTime<Utc>>,
}

/// A stored item in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub source: String,
    pub collector_id: String,
    pub title: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub metadata: serde_json::Value,
    pub published_at: Option<DateTime<Utc>>,
    pub collected_at: DateTime<Utc>,
}

/// Relevance score assigned by the intelligence pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub id: String,
    pub item_id: String,
    pub interest_name: String,
    pub score: f64,
    pub reasoning: Option<String>,
    pub model_used: String,
    pub scored_at: DateTime<Utc>,
}

/// AI-generated summary of an item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub id: String,
    pub item_id: String,
    pub summary: String,
    pub model_used: String,
    pub created_at: DateTime<Utc>,
}

/// Tag applied to an item (by AI or user).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub item_id: String,
    pub tag: String,
}

/// Record of a collector run for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorRun {
    pub id: String,
    pub collector_id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub items_count: u32,
    pub status: String,
    pub error: Option<String>,
}

/// AI provider configuration stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSetting {
    pub id: String,
    pub display_name: String,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub enabled: bool,
    pub is_active: bool,
    pub extra_config: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

/// API response for the feed endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    pub id: String,
    pub source: String,
    pub collector_id: String,
    pub title: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub metadata: serde_json::Value,
    pub tags: Vec<String>,
    pub score: Option<f64>,
    pub published_at: Option<DateTime<Utc>>,
    pub collected_at: DateTime<Utc>,
}
