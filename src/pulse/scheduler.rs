use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::pulse::collectors::Collector;
use crate::pulse::storage::PulseDatabase;

/// Manages periodic execution of collectors.
pub struct Scheduler {
    collectors: Vec<Arc<dyn Collector>>,
    db: PulseDatabase,
}

impl Scheduler {
    pub fn new(collectors: Vec<Arc<dyn Collector>>, db: PulseDatabase) -> Self {
        Self { collectors, db }
    }

    /// Start all collector loops. Each collector runs on its own interval.
    pub async fn start(self: Arc<Self>) {
        for collector in &self.collectors {
            if !collector.enabled() {
                tracing::info!("Collector '{}' is disabled, skipping", collector.name());
                continue;
            }

            let collector = Arc::clone(collector);
            let db = self.db.clone();
            let interval = collector.default_interval();

            tracing::info!(
                "Scheduling collector '{}' every {}s",
                collector.name(),
                interval.as_secs()
            );

            tokio::spawn(async move {
                // Run immediately on startup
                run_collector(&collector, &db).await;

                // Then run on interval
                let mut ticker = time::interval(interval);
                ticker.tick().await; // Skip the first immediate tick
                loop {
                    ticker.tick().await;
                    run_collector(&collector, &db).await;
                }
            });
        }
    }
}

/// Execute a single collector run with logging and error tracking.
async fn run_collector(collector: &Arc<dyn Collector>, db: &PulseDatabase) {
    let run_id = match db.start_collector_run(collector.id()).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(
                "Failed to record collector run start for '{}': {}",
                collector.id(),
                e
            );
            return;
        }
    };

    tracing::info!("Running collector: {}", collector.name());

    match collector.collect().await {
        Ok(items) => {
            let mut inserted = 0u32;
            for item in &items {
                // Deduplicate by URL
                if let Some(ref url) = item.url {
                    match db.item_exists_by_url(url).await {
                        Ok(true) => continue,
                        Ok(false) => {}
                        Err(e) => {
                            tracing::warn!("Failed to check item existence: {}", e);
                        }
                    }
                }

                match db.insert_item(item).await {
                    Ok(_) => inserted += 1,
                    Err(e) => {
                        tracing::warn!("Failed to insert item '{}': {}", item.title, e);
                    }
                }
            }

            tracing::info!(
                "Collector '{}' finished: {} fetched, {} new",
                collector.name(),
                items.len(),
                inserted
            );

            if let Err(e) = db.finish_collector_run(&run_id, inserted, None).await {
                tracing::error!("Failed to record collector run completion: {}", e);
            }
        }
        Err(e) => {
            tracing::error!("Collector '{}' failed: {}", collector.name(), e);
            let _ = db
                .finish_collector_run(&run_id, 0, Some(&e.to_string()))
                .await;
        }
    }
}

/// Manually trigger a specific collector by ID.
pub async fn trigger_collector(
    collectors: &[Arc<dyn Collector>],
    db: &PulseDatabase,
    collector_id: &str,
) -> Result<()> {
    let collector = collectors
        .iter()
        .find(|c| c.id() == collector_id)
        .ok_or_else(|| anyhow::anyhow!("Collector '{}' not found", collector_id))?;

    run_collector(collector, db).await;
    Ok(())
}
