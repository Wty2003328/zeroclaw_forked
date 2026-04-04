use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::time::Duration;

use super::{parse_interval, Collector};
use crate::pulse::models::RawItem;
use crate::pulse::storage::PulseDatabase;

/// Video subscription collector — fetches latest videos from YouTube channels
/// and Bilibili UP主 via RSS/Atom feeds (no API keys needed).
/// Reads channel list from database on every collect() call for hot-reload.
pub struct VideoCollector {
    client: reqwest::Client,
    db: PulseDatabase,
}

impl VideoCollector {
    pub fn new(db: PulseDatabase) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Pulse/0.1.0")
            .build()
            .unwrap_or_default();

        Self { client, db }
    }

    fn feed_url(platform: &str, channel_id: &str) -> String {
        match platform {
            "youtube" => format!(
                "https://www.youtube.com/feeds/videos.xml?channel_id={}",
                channel_id
            ),
            "bilibili" => format!("https://rsshub.app/bilibili/user/video/{}", channel_id),
            _ => String::new(),
        }
    }

    fn video_url(platform: &str, video_id: &str) -> String {
        match platform {
            "youtube" => format!("https://www.youtube.com/watch?v={}", video_id),
            "bilibili" => format!("https://www.bilibili.com/video/{}", video_id),
            _ => String::new(),
        }
    }
}

#[async_trait]
impl Collector for VideoCollector {
    fn id(&self) -> &str {
        "videos"
    }

    fn name(&self) -> &str {
        "Video Subscriptions"
    }

    fn default_interval(&self) -> Duration {
        Duration::from_secs(1800) // 30 minutes
    }

    fn enabled(&self) -> bool {
        true // Always enabled — checks DB for channels each cycle
    }

    async fn collect(&self) -> Result<Vec<RawItem>> {
        // Hot-reload: read channels from database on every call
        let channels = self.db.get_video_channels().await.unwrap_or_default();

        if channels.is_empty() {
            return Ok(Vec::new());
        }

        tracing::debug!("Fetching videos from {} channels", channels.len());

        // Read custom RSSHub URL from settings (for Bilibili)
        let rsshub_base = self
            .db
            .get_setting("rsshub_url")
            .await
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://rsshub.app".to_string());

        let mut items = Vec::new();

        for (platform, channel_id, display_name) in &channels {
            // Build feed URLs — use custom RSSHub for Bilibili
            let urls = if platform == "bilibili" {
                vec![format!(
                    "{}/bilibili/user/video/{}",
                    rsshub_base.trim_end_matches('/'),
                    channel_id
                )]
            } else {
                vec![Self::feed_url(platform, channel_id)]
            };

            if urls[0].is_empty() {
                continue;
            }

            let mut fetched = false;
            for feed_url in &urls {
                match self
                    .fetch_feed(feed_url, platform, channel_id, display_name)
                    .await
                {
                    Ok(mut feed_items) => {
                        items.append(&mut feed_items);
                        fetched = true;
                        break;
                    }
                    Err(e) => {
                        tracing::debug!("Feed {} failed: {}", feed_url, e);
                    }
                }
            }
            if !fetched {
                tracing::warn!(
                    "All feeds failed for {} {} ({})",
                    platform,
                    display_name,
                    channel_id
                );
            }

            // Small delay between requests
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        tracing::info!(
            "Fetched {} videos from {} channels",
            items.len(),
            channels.len()
        );
        Ok(items)
    }
}

impl VideoCollector {
    async fn fetch_feed(
        &self,
        url: &str,
        platform: &str,
        channel_id: &str,
        display_name: &str,
    ) -> Result<Vec<RawItem>> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("HTTP {}", response.status());
        }

        let body = response.bytes().await?;
        let feed = feed_rs::parser::parse(&body[..])?;

        let items: Vec<RawItem> = feed
            .entries
            .into_iter()
            .take(10)
            .map(|entry| {
                let title = entry
                    .title
                    .map(|t| t.content)
                    .unwrap_or_else(|| "Untitled".to_string());

                let video_id = entry.id.clone();
                let entry_url = entry
                    .links
                    .first()
                    .map(|l| l.href.clone())
                    .unwrap_or_else(|| Self::video_url(platform, &video_id));

                let published = entry
                    .published
                    .or(entry.updated)
                    .unwrap_or_else(|| Utc::now());

                let thumbnail = entry
                    .media
                    .first()
                    .and_then(|m| m.thumbnails.first())
                    .map(|t| t.image.uri.clone())
                    .or_else(|| {
                        if platform == "youtube" {
                            let vid = video_id.strip_prefix("yt:video:").unwrap_or(&video_id);
                            Some(format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", vid))
                        } else {
                            None
                        }
                    });

                let description = entry
                    .summary
                    .map(|s| s.content)
                    .or_else(|| entry.content.and_then(|c| c.body))
                    .unwrap_or_default();

                let author = entry
                    .authors
                    .first()
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| display_name.to_string());

                let metadata = serde_json::json!({
                    "platform": platform,
                    "channel_id": channel_id,
                    "channel_name": display_name,
                    "author": author,
                    "video_id": video_id,
                    "thumbnail": thumbnail,
                    "description": description.chars().take(300).collect::<String>(),
                });

                RawItem {
                    source: format!("video:{}:{}", platform, channel_id),
                    collector_id: "videos".to_string(),
                    title,
                    url: Some(entry_url),
                    content: if description.is_empty() {
                        None
                    } else {
                        Some(description)
                    },
                    metadata,
                    published_at: Some(published),
                }
            })
            .collect();

        Ok(items)
    }
}
