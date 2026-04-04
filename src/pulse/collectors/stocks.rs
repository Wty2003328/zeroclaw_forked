use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::time::Duration;

use super::{parse_interval, Collector};
use crate::pulse::config::StocksConfig;
use crate::pulse::models::RawItem;

/// Uses Yahoo Finance v8 chart API (no key required) for stock quotes.
pub struct StocksCollector {
    config: StocksConfig,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct ChartResponse {
    chart: ChartResult,
}

#[derive(Debug, Deserialize)]
struct ChartResult {
    result: Option<Vec<ChartData>>,
}

#[derive(Debug, Deserialize)]
struct ChartData {
    meta: ChartMeta,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChartMeta {
    symbol: String,
    short_name: Option<String>,
    long_name: Option<String>,
    regular_market_price: Option<f64>,
    chart_previous_close: Option<f64>,
    regular_market_volume: Option<u64>,
    regular_market_day_high: Option<f64>,
    regular_market_day_low: Option<f64>,
    fifty_two_week_high: Option<f64>,
    fifty_two_week_low: Option<f64>,
    currency: Option<String>,
}

impl StocksCollector {
    pub fn new(config: StocksConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .unwrap_or_default();

        Self { config, client }
    }
}

#[async_trait]
impl Collector for StocksCollector {
    fn id(&self) -> &str {
        "stocks"
    }

    fn name(&self) -> &str {
        "Stock Prices"
    }

    fn default_interval(&self) -> Duration {
        parse_interval(&self.config.interval)
    }

    fn enabled(&self) -> bool {
        self.config.enabled
    }

    async fn collect(&self) -> Result<Vec<RawItem>> {
        if self.config.symbols.is_empty() {
            return Ok(Vec::new());
        }

        tracing::debug!("Fetching stock quotes for: {:?}", self.config.symbols);

        let now = Utc::now();
        let mut items = Vec::new();

        for symbol in &self.config.symbols {
            match self.fetch_quote(symbol).await {
                Ok(item) => items.push(item),
                Err(e) => {
                    tracing::warn!("Failed to fetch quote for {}: {}", symbol, e);
                }
            }
            // Small delay between requests to avoid rate limiting
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        tracing::info!("Fetched {} stock quotes", items.len());
        Ok(items)
    }
}

impl StocksCollector {
    async fn fetch_quote(&self, symbol: &str) -> Result<RawItem> {
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d",
            symbol
        );

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("Yahoo Finance returned status {}", response.status());
        }

        let chart: ChartResponse = response.json().await?;
        let data = chart
            .chart
            .result
            .and_then(|r| r.into_iter().next())
            .ok_or_else(|| anyhow::anyhow!("No data for {}", symbol))?;

        let meta = data.meta;
        let price = meta.regular_market_price.unwrap_or(0.0);
        let prev_close = meta.chart_previous_close.unwrap_or(price);
        let change = price - prev_close;
        let change_pct = if prev_close > 0.0 {
            (change / prev_close) * 100.0
        } else {
            0.0
        };
        let direction = if change >= 0.0 { "up" } else { "down" };

        let name = meta
            .long_name
            .or(meta.short_name)
            .unwrap_or_else(|| symbol.to_string());

        let title = format!(
            "{} ({}) ${:.2} {:+.2} ({:+.2}%)",
            meta.symbol, name, price, change, change_pct
        );

        let metadata = serde_json::json!({
            "symbol": meta.symbol,
            "name": name,
            "price": price,
            "change": change,
            "change_percent": change_pct,
            "direction": direction,
            "volume": meta.regular_market_volume,
            "previous_close": prev_close,
            "open": prev_close, // v8 chart doesn't give open directly
            "day_high": meta.regular_market_day_high,
            "day_low": meta.regular_market_day_low,
            "52w_high": meta.fifty_two_week_high,
            "52w_low": meta.fifty_two_week_low,
            "currency": meta.currency,
        });

        Ok(RawItem {
            source: format!("stock:{}", meta.symbol),
            collector_id: "stocks".to_string(),
            title,
            url: Some(format!("https://finance.yahoo.com/quote/{}", meta.symbol)),
            content: None,
            metadata,
            published_at: Some(Utc::now()),
        })
    }
}
