use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::time::Duration;

use super::{parse_interval, Collector};
use crate::pulse::config::RedditConfig;
use crate::pulse::models::RawItem;

#[derive(Debug, Deserialize)]
struct RedditListing {
    data: RedditListingData,
}

#[derive(Debug, Deserialize)]
struct RedditListingData {
    children: Vec<RedditChild>,
}

#[derive(Debug, Deserialize)]
struct RedditChild {
    data: RedditPost,
}

#[derive(Debug, Deserialize)]
struct RedditPost {
    id: String,
    title: String,
    selftext: Option<String>,
    url: Option<String>,
    permalink: String,
    subreddit: String,
    author: String,
    score: i64,
    num_comments: u32,
    created_utc: f64,
    is_self: bool,
    link_flair_text: Option<String>,
}

pub struct RedditCollector {
    config: RedditConfig,
    client: reqwest::Client,
}

impl RedditCollector {
    pub fn new(config: RedditConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Pulse/0.1.0 (personal dashboard)")
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    async fn fetch_subreddit(&self, subreddit: &str) -> Result<Vec<RawItem>> {
        tracing::debug!("Fetching Reddit r/{}", subreddit);

        let url = format!("https://www.reddit.com/r/{}/hot.json?limit=25", subreddit);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Reddit API returned status {}", response.status());
        }

        let listing: RedditListing = response.json().await?;

        let items: Vec<RawItem> = listing
            .data
            .children
            .into_iter()
            .map(|child| {
                let post = child.data;
                let published_at = DateTime::from_timestamp(post.created_utc as i64, 0)
                    .unwrap_or_else(|| Utc::now());

                let content = if post.is_self {
                    post.selftext.clone()
                } else {
                    None
                };

                let external_url = if !post.is_self {
                    post.url.clone()
                } else {
                    None
                };

                let reddit_url = format!("https://www.reddit.com{}", post.permalink);

                let metadata = serde_json::json!({
                    "reddit_id": post.id,
                    "subreddit": post.subreddit,
                    "author": post.author,
                    "score": post.score,
                    "comments": post.num_comments,
                    "is_self": post.is_self,
                    "flair": post.link_flair_text,
                    "external_url": external_url,
                    "reddit_url": reddit_url,
                });

                RawItem {
                    source: format!("reddit:r/{}", post.subreddit),
                    collector_id: "reddit".to_string(),
                    title: post.title,
                    url: Some(external_url.unwrap_or(reddit_url)),
                    content,
                    metadata,
                    published_at: Some(published_at),
                }
            })
            .collect();

        tracing::info!("Fetched {} items from r/{}", items.len(), subreddit);
        Ok(items)
    }
}

#[async_trait]
impl Collector for RedditCollector {
    fn id(&self) -> &str {
        "reddit"
    }

    fn name(&self) -> &str {
        "Reddit"
    }

    fn default_interval(&self) -> Duration {
        parse_interval(&self.config.interval)
    }

    fn enabled(&self) -> bool {
        self.config.enabled
    }

    async fn collect(&self) -> Result<Vec<RawItem>> {
        let mut all_items = Vec::new();

        for subreddit in &self.config.subreddits {
            match self.fetch_subreddit(subreddit).await {
                Ok(items) => all_items.extend(items),
                Err(e) => {
                    tracing::warn!("Failed to fetch r/{}: {}", subreddit, e);
                }
            }
            // Small delay between subreddit fetches to be polite
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(all_items)
    }
}
