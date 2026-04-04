use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::time::Duration;

use super::{parse_interval, Collector};
use crate::pulse::config::RssConfig;
use crate::pulse::models::RawItem;

pub struct RssCollector {
    config: RssConfig,
    client: reqwest::Client,
}

impl RssCollector {
    pub fn new(config: RssConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Pulse/0.1.0")
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    async fn fetch_feed(&self, name: &str, url: &str) -> Result<Vec<RawItem>> {
        tracing::debug!("Fetching RSS feed: {} ({})", name, url);

        let response = self.client.get(url).send().await?;
        let body = response.bytes().await?;
        let feed = feed_rs::parser::parse(&body[..])?;

        let items: Vec<RawItem> = feed
            .entries
            .into_iter()
            .map(|entry| {
                let title = entry
                    .title
                    .map(|t| t.content)
                    .unwrap_or_else(|| "Untitled".to_string());

                let url = entry.links.first().map(|l| l.href.clone());

                let content = entry
                    .summary
                    .map(|s| s.content)
                    .or_else(|| entry.content.and_then(|c| c.body));

                let published_at: Option<DateTime<Utc>> = entry
                    .published
                    .or(entry.updated)
                    .map(|d| d.with_timezone(&Utc));

                let metadata = serde_json::json!({
                    "feed_name": name,
                    "feed_url": url,
                    "authors": entry.authors.iter().map(|a| &a.name).collect::<Vec<_>>(),
                    "categories": entry.categories.iter().map(|c| &c.term).collect::<Vec<_>>(),
                });

                RawItem {
                    source: format!("rss:{}", name.to_lowercase().replace(' ', "-")),
                    collector_id: "rss".to_string(),
                    title,
                    url: entry.links.first().map(|l| l.href.clone()),
                    content,
                    metadata,
                    published_at,
                }
            })
            .collect();

        tracing::info!("Fetched {} items from RSS feed: {}", items.len(), name);
        Ok(items)
    }
}

#[async_trait]
impl Collector for RssCollector {
    fn id(&self) -> &str {
        "rss"
    }

    fn name(&self) -> &str {
        "RSS Feeds"
    }

    fn default_interval(&self) -> Duration {
        parse_interval(&self.config.interval)
    }

    fn enabled(&self) -> bool {
        self.config.enabled
    }

    async fn collect(&self) -> Result<Vec<RawItem>> {
        let mut all_items = Vec::new();

        for feed in &self.config.feeds {
            match self.fetch_feed(&feed.name, &feed.url).await {
                Ok(items) => all_items.extend(items),
                Err(e) => {
                    tracing::warn!("Failed to fetch RSS feed '{}': {}", feed.name, e);
                }
            }
        }

        Ok(all_items)
    }
}
