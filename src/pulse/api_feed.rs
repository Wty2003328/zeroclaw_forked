use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::pulse::scheduler;
use crate::pulse::PulseState;

/// Build the API route tree.
pub fn routes() -> Router<PulseState> {
    Router::new()
        .route("/health", get(health))
        .route("/feed", get(get_feed))
        .route("/feed/digest", get(get_digest))
        .route("/collectors", get(get_collectors))
        .route("/collectors/{id}/run", post(trigger_collector))
}

// --- Health ---

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// --- Feed ---

#[derive(Debug, Deserialize)]
struct FeedQuery {
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    offset: u32,
    source: Option<String>,
}

fn default_limit() -> u32 {
    50
}

async fn get_feed(
    State(state): State<PulseState>,
    Query(query): Query<FeedQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let items = state
        .db
        .get_feed(query.limit, query.offset, query.source.as_deref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "items": items,
        "count": items.len(),
        "limit": query.limit,
        "offset": query.offset,
    })))
}

// --- Digest (AI-curated top items) ---

async fn get_digest(
    State(state): State<PulseState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Get items sorted by highest AI score
    let items = state
        .db
        .get_digest(10)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "items": items,
        "count": items.len(),
        "limit": 10,
        "offset": 0,
    })))
}

// --- Collectors ---

#[derive(Serialize)]
struct CollectorInfo {
    id: String,
    name: String,
    enabled: bool,
    interval_secs: u64,
}

async fn get_collectors(State(state): State<PulseState>) -> Json<serde_json::Value> {
    let overrides = state
        .db
        .get_all_collector_intervals()
        .await
        .unwrap_or_default();

    let collectors: Vec<CollectorInfo> = state
        .collectors
        .iter()
        .map(|c| {
            let id = c.id().to_string();
            let interval = overrides
                .iter()
                .find(|(oid, _)| *oid == id)
                .map(|(_, s)| *s)
                .unwrap_or_else(|| c.default_interval().as_secs());
            CollectorInfo {
                id,
                name: c.name().to_string(),
                enabled: c.enabled(),
                interval_secs: interval,
            }
        })
        .collect();

    let status = state.db.get_collector_status().await.unwrap_or_default();

    Json(serde_json::json!({
        "collectors": collectors,
        "recent_runs": status,
    }))
}

async fn trigger_collector(
    State(state): State<PulseState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    scheduler::trigger_collector(&state.collectors, &state.db, &id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "triggered",
        "collector": id,
    })))
}
