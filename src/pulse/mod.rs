//! Pulse Dashboard — personal intelligence dashboard module for ZeroClaw.
//!
//! Provides data collectors (RSS, stocks, weather, videos), a widget-based
//! dashboard frontend, and settings management. Runs as an optional module
//! within ZeroClaw's gateway.

pub mod api_calendar;
pub mod api_feed;
pub mod api_settings;
pub mod api_system;
pub mod collectors;
pub mod config;
pub mod config_loader;
pub mod models;
pub mod scheduler;
pub mod storage;

use axum::{routing::get, Router};
use std::sync::{Arc, Mutex};

use collectors::Collector;
use storage::PulseDatabase;

/// Shared state for all Pulse API handlers.
#[derive(Clone)]
pub struct PulseState {
    pub db: PulseDatabase,
    pub collectors: Vec<Arc<dyn Collector>>,
    pub sysinfo: Arc<Mutex<sysinfo::System>>,
}

/// Build the Pulse API router with all routes.
pub fn routes() -> Router<PulseState> {
    let feed_routes = api_feed::routes();
    let settings_routes = api_settings::routes();

    Router::new()
        .nest("/", feed_routes)
        .nest("/settings", settings_routes)
        .nest("/system", api_system::routes())
        .nest("/calendar", api_calendar::routes())
}

/// Initialize the Pulse dashboard module.
/// Returns a PulseState ready to be mounted in the gateway.
pub async fn init(data_dir: &str) -> anyhow::Result<PulseState> {
    let db_path = format!("{}/pulse.db", data_dir);

    // Initialize database
    let db = PulseDatabase::new(&db_path).await?;

    // Initialize system monitor
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    // Create collectors — videos collector gets DB handle for hot-reload
    let mut collector_list: Vec<Arc<dyn Collector>> = Vec::new();

    // RSS collector with default feeds
    let rss_config = config::RssConfig {
        enabled: true,
        interval: "30m".to_string(),
        feeds: vec![
            config::FeedEntry {
                name: "Hacker News".to_string(),
                url: "https://hnrss.org/frontpage".to_string(),
            },
            config::FeedEntry {
                name: "Lobsters".to_string(),
                url: "https://lobste.rs/rss".to_string(),
            },
        ],
    };
    collector_list.push(Arc::new(collectors::rss::RssCollector::new(rss_config)));

    // HN collector
    let hn_config = config::HackerNewsConfig {
        enabled: true,
        interval: "15m".to_string(),
        min_score: 50,
    };
    collector_list.push(Arc::new(
        collectors::hackernews::HackerNewsCollector::new(hn_config),
    ));

    // Stocks collector
    let stocks_config = config::StocksConfig {
        enabled: true,
        interval: "5m".to_string(),
        symbols: vec![
            "AAPL".into(),
            "GOOGL".into(),
            "MSFT".into(),
            "NVDA".into(),
            "TSLA".into(),
        ],
        api_key: None,
    };
    collector_list.push(Arc::new(collectors::stocks::StocksCollector::new(
        stocks_config,
    )));

    // Weather collector
    let weather_config = config::WeatherConfig {
        enabled: true,
        interval: "1h".to_string(),
        location: Some("auto".to_string()),
        api_key: None,
    };
    collector_list.push(Arc::new(collectors::weather::WeatherCollector::new(
        weather_config,
    )));

    // Video collector (hot-reloads channels from DB)
    collector_list.push(Arc::new(collectors::videos::VideoCollector::new(
        db.clone(),
    )));

    tracing::info!("Pulse: initialized with {} collectors", collector_list.len());

    // Start scheduler in background
    let sched = Arc::new(scheduler::Scheduler::new(
        collector_list.clone(),
        db.clone(),
    ));
    let sched_handle = Arc::clone(&sched);
    tokio::spawn(async move {
        sched_handle.start().await;
    });

    Ok(PulseState {
        db,
        collectors: collector_list,
        sysinfo: Arc::new(Mutex::new(sys)),
    })
}
