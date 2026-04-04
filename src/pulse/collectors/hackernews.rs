use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::time::Duration;

use super::{parse_interval, Collector};
use crate::pulse::config::HackerNewsConfig;
use crate::pulse::models::RawItem;

const HN_API_BASE: &str = "https://hacker-news.firebaseio.com/v0";

#[derive(Debug, Deserialize)]
struct HnItem {
    id: u64,
    title: Option<String>,
    url: Option<String>,
    text: Option<String>,
    score: Option<u32>,
    by: Option<String>,
    time: Option<i64>,
    #[serde(rename = "type")]
    item_type: Option<String>,
    descendants: Option<u32>,
}

pub struct HackerNewsCollector {
    config: HackerNewsConfig,
    client: reqwest::Client,
}

impl HackerNewsCollector {
    pub fn new(config: HackerNewsConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Pulse/0.1.0")
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    async fn fetch_item(&self, id: u64) -> Result<Option<HnItem>> {
        let url = format!("{}/item/{}.json", HN_API_BASE, id);
        let item: Option<HnItem> = self.client.get(&url).send().await?.json().await?;
        Ok(item)
    }
}

#[async_trait]
impl Collector for HackerNewsCollector {
    fn id(&self) -> &str {
        "hackernews"
    }

    fn name(&self) -> &str {
        "Hacker News"
    }

    fn default_interval(&self) -> Duration {
        parse_interval(&self.config.interval)
    }

    fn enabled(&self) -> bool {
        self.config.enabled
    }

    async fn collect(&self) -> Result<Vec<RawItem>> {
        tracing::debug!("Fetching Hacker News top stories");

        // Get top story IDs
        let url = format!("{}/topstories.json", HN_API_BASE);
        let story_ids: Vec<u64> = self.client.get(&url).send().await?.json().await?;

        // Fetch top 30 stories (to keep it reasonable)
        let story_ids: Vec<u64> = story_ids.into_iter().take(30).collect();

        let mut items = Vec::new();

        // Fetch stories concurrently in batches
        let mut handles = Vec::new();
        for id in story_ids {
            let client = self.client.clone();
            handles.push(tokio::spawn(async move {
                let url = format!("{}/item/{}.json", HN_API_BASE, id);
                let result: Result<Option<HnItem>, reqwest::Error> =
                    client.get(&url).send().await?.json().await;
                result
            }));
        }

        for handle in handles {
            match handle.await {
                Ok(Ok(Some(hn_item))) => {
                    // Filter by minimum score
                    let score = hn_item.score.unwrap_or(0);
                    if score < self.config.min_score {
                        continue;
                    }

                    let title = hn_item.title.unwrap_or_else(|| "Untitled".to_string());
                    let published_at = hn_item
                        .time
                        .map(|t| DateTime::from_timestamp(t, 0).unwrap_or_else(|| Utc::now()));

                    let hn_url = format!("https://news.ycombinator.com/item?id={}", hn_item.id);

                    let metadata = serde_json::json!({
                        "hn_id": hn_item.id,
                        "score": score,
                        "by": hn_item.by,
                        "comments": hn_item.descendants.unwrap_or(0),
                        "type": hn_item.item_type,
                        "hn_url": hn_url,
                    });

                    items.push(RawItem {
                        source: "hackernews".to_string(),
                        collector_id: "hackernews".to_string(),
                        title,
                        url: hn_item.url.or(Some(hn_url)),
                        content: hn_item.text,
                        metadata,
                        published_at,
                    });
                }
                Ok(Ok(None)) => {}
                Ok(Err(e)) => tracing::warn!("Failed to fetch HN item: {}", e),
                Err(e) => tracing::warn!("Task failed for HN item: {}", e),
            }
        }

        tracing::info!(
            "Fetched {} items from Hacker News (min_score: {})",
            items.len(),
            self.config.min_score
        );
        Ok(items)
    }
}
