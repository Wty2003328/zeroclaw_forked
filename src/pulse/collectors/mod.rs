pub mod github;
pub mod hackernews;
pub mod reddit;
pub mod rss;
pub mod stocks;
pub mod videos;
pub mod weather;

use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;

use super::models::RawItem;

/// Trait that all data collectors must implement.
#[async_trait]
pub trait Collector: Send + Sync {
    /// Unique identifier for this collector (e.g., "rss", "hackernews").
    fn id(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Default polling interval.
    fn default_interval(&self) -> Duration;

    /// Whether this collector is enabled.
    fn enabled(&self) -> bool;

    /// Fetch items from the data source.
    async fn collect(&self) -> Result<Vec<RawItem>>;
}

/// Parse an interval string like "30m", "1h", "5m" into a Duration.
pub fn parse_interval(s: &str) -> Duration {
    let s = s.trim();
    if let Some(mins) = s.strip_suffix('m') {
        if let Ok(n) = mins.parse::<u64>() {
            return Duration::from_secs(n * 60);
        }
    }
    if let Some(hours) = s.strip_suffix('h') {
        if let Ok(n) = hours.parse::<u64>() {
            return Duration::from_secs(n * 3600);
        }
    }
    if let Some(secs) = s.strip_suffix('s') {
        if let Ok(n) = secs.parse::<u64>() {
            return Duration::from_secs(n);
        }
    }
    // Default to 30 minutes
    Duration::from_secs(30 * 60)
}
