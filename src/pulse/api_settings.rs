use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::pulse::PulseState;

use crate::pulse::models::ProviderSetting;

/// Known providers with display names.
const KNOWN_PROVIDERS: &[(&str, &str)] = &[
    ("claude", "Claude"),
    ("openai", "GPT"),
    ("gemini", "Gemini"),
    ("deepseek", "DeepSeek"),
    ("copilot", "Copilot"),
    ("minimax", "MiniMax"),
    ("glm", "GLM"),
];

pub fn routes() -> Router<PulseState> {
    Router::new()
        .route("/providers", get(list_providers))
        .route(
            "/providers/{id}",
            get(get_provider).put(save_provider).delete(delete_provider),
        )
        .route("/providers/{id}/test", post(test_provider))
        .route("/providers/{id}/activate", post(activate_provider))
        .route("/collectors/{id}/interval", put(set_collector_interval))
        .route("/feeds", get(list_user_feeds).post(add_user_feed))
        .route("/feeds/{url}", delete(remove_user_feed))
        .route("/app", get(get_app_settings).put(save_app_settings))
        .route("/videos", get(list_video_channels).post(add_video_channel))
        .route(
            "/videos/{platform}/{channel_id}",
            delete(remove_video_channel),
        )
}

// --- Response types ---

#[derive(Serialize)]
struct ProviderResponse {
    id: String,
    display_name: String,
    api_key_set: bool,
    api_key_preview: Option<String>,
    model: Option<String>,
    endpoint: Option<String>,
    enabled: bool,
    is_active: bool,
    extra_config: serde_json::Value,
}

impl From<ProviderSetting> for ProviderResponse {
    fn from(p: ProviderSetting) -> Self {
        let api_key_preview = p.api_key.as_ref().map(|k| mask_key(k));
        Self {
            id: p.id,
            display_name: p.display_name,
            api_key_set: p.api_key.is_some(),
            api_key_preview,
            model: p.model,
            endpoint: p.endpoint,
            enabled: p.enabled,
            is_active: p.is_active,
            extra_config: p.extra_config,
        }
    }
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "••••••••".to_string();
    }
    let prefix = &key[..4];
    let suffix = &key[key.len() - 4..];
    format!("{}••••{}", prefix, suffix)
}

fn default_provider(id: &str, name: &str) -> ProviderResponse {
    ProviderResponse {
        id: id.to_string(),
        display_name: name.to_string(),
        api_key_set: false,
        api_key_preview: None,
        model: None,
        endpoint: None,
        enabled: false,
        is_active: false,
        extra_config: serde_json::json!({}),
    }
}

// --- Handlers ---

async fn list_providers(State(state): State<PulseState>) -> Json<serde_json::Value> {
    let stored = state.db.get_providers().await.unwrap_or_default();

    let mut providers: Vec<ProviderResponse> = Vec::new();

    for &(id, name) in KNOWN_PROVIDERS {
        if let Some(p) = stored.iter().find(|p| p.id == id) {
            providers.push(ProviderResponse::from(p.clone()));
        } else {
            providers.push(default_provider(id, name));
        }
    }

    Json(serde_json::json!({ "providers": providers }))
}

async fn get_provider(
    State(state): State<PulseState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if let Some(p) = state
        .db
        .get_provider(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        Ok(Json(serde_json::json!(ProviderResponse::from(p))))
    } else if let Some(&(_, name)) = KNOWN_PROVIDERS.iter().find(|&&(pid, _)| pid == id) {
        Ok(Json(serde_json::json!(default_provider(&id, name))))
    } else {
        Err((StatusCode::NOT_FOUND, format!("Unknown provider: {}", id)))
    }
}

#[derive(Deserialize)]
struct SaveProviderRequest {
    api_key: Option<String>,
    model: Option<String>,
    endpoint: Option<String>,
}

async fn save_provider(
    State(state): State<PulseState>,
    Path(id): Path<String>,
    Json(body): Json<SaveProviderRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let display_name = KNOWN_PROVIDERS
        .iter()
        .find(|&&(pid, _)| pid == id)
        .map(|&(_, name)| name.to_string())
        .unwrap_or_else(|| id.clone());

    let setting = ProviderSetting {
        id: id.clone(),
        display_name,
        api_key: body.api_key,
        model: body.model,
        endpoint: body.endpoint,
        enabled: true,
        is_active: false, // Don't change active status on save
        extra_config: serde_json::json!({}),
        created_at: String::new(), // Handled by upsert
        updated_at: String::new(),
    };

    state
        .db
        .upsert_provider(&setting)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        serde_json::json!({ "status": "saved", "provider": id }),
    ))
}

async fn delete_provider(
    State(state): State<PulseState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state
        .db
        .delete_provider_key(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        serde_json::json!({ "status": "deleted", "provider": id }),
    ))
}

#[derive(Deserialize)]
struct TestProviderRequest {
    api_key: Option<String>,
    model: Option<String>,
}

async fn test_provider(
    State(state): State<PulseState>,
    Path(id): Path<String>,
    Json(body): Json<TestProviderRequest>,
) -> Json<serde_json::Value> {
    // Use provided key or fall back to stored key
    let api_key = if let Some(key) = body.api_key.filter(|k| !k.is_empty()) {
        key
    } else if let Ok(Some(stored)) = state.db.get_provider(&id).await {
        match stored.api_key {
            Some(key) => key,
            None => {
                return Json(serde_json::json!({
                    "success": false,
                    "message": "No API key configured"
                }));
            }
        }
    } else {
        return Json(serde_json::json!({
            "success": false,
            "message": "No API key provided"
        }));
    };

    // TODO: integrate with ZeroClaw's provider system for testing
    Json(serde_json::json!({
        "success": false,
        "message": "Provider testing not yet integrated with ZeroClaw. Save the key and try using it."
    }))
}

async fn activate_provider(
    State(state): State<PulseState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Verify the provider has an API key
    let provider = state
        .db
        .get_provider(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match provider {
        Some(p) if p.api_key.is_some() => {}
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Provider must have an API key configured before activation".to_string(),
            ));
        }
    }

    state
        .db
        .set_active_provider(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        serde_json::json!({ "status": "activated", "provider": id }),
    ))
}

// --- Collector interval ---

#[derive(Deserialize)]
struct SetIntervalRequest {
    interval_secs: u64,
}

async fn set_collector_interval(
    State(state): State<PulseState>,
    Path(id): Path<String>,
    Json(body): Json<SetIntervalRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body.interval_secs < 10 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Interval must be at least 10 seconds".to_string(),
        ));
    }

    state
        .db
        .set_collector_interval(&id, body.interval_secs)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        serde_json::json!({ "status": "saved", "collector": id, "interval_secs": body.interval_secs }),
    ))
}

// --- User RSS feeds ---

async fn list_user_feeds(
    State(state): State<PulseState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let feeds = state
        .db
        .get_user_feeds()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let feeds: Vec<serde_json::Value> = feeds
        .into_iter()
        .map(|(name, url)| serde_json::json!({ "name": name, "url": url }))
        .collect();

    Ok(Json(serde_json::json!({ "feeds": feeds })))
}

#[derive(Deserialize)]
struct AddFeedRequest {
    name: String,
    url: String,
}

async fn add_user_feed(
    State(state): State<PulseState>,
    Json(body): Json<AddFeedRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body.name.is_empty() || body.url.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Name and URL are required".to_string(),
        ));
    }

    state
        .db
        .add_user_feed(&body.name, &body.url)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        serde_json::json!({ "status": "added", "name": body.name, "url": body.url }),
    ))
}

async fn remove_user_feed(
    State(state): State<PulseState>,
    Path(url): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let decoded = urlencoding::decode(&url)
        .map(|s| s.into_owned())
        .unwrap_or(url);

    state
        .db
        .remove_user_feed(&decoded)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "status": "removed" })))
}

// --- App settings (weather location, stock symbols, etc.) ---

async fn get_app_settings(
    State(state): State<PulseState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let all = state
        .db
        .get_all_settings()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut map = serde_json::Map::new();
    for (k, v) in all {
        map.insert(k, serde_json::Value::String(v));
    }
    Ok(Json(serde_json::Value::Object(map)))
}

async fn save_app_settings(
    State(state): State<PulseState>,
    Json(body): Json<serde_json::Map<String, serde_json::Value>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    for (key, value) in &body {
        let val_str = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        state
            .db
            .set_setting(key, &val_str)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    Ok(Json(serde_json::json!({ "status": "saved" })))
}

// --- Video subscriptions ---

async fn list_video_channels(
    State(state): State<PulseState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let channels = state
        .db
        .get_video_channels()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let list: Vec<serde_json::Value> = channels
        .into_iter()
        .map(|(p, c, n)| serde_json::json!({"platform": p, "channel_id": c, "name": n}))
        .collect();
    Ok(Json(serde_json::json!({ "channels": list })))
}

#[derive(Deserialize)]
struct AddVideoChannelRequest {
    platform: String,
    channel_id: String,
    name: String,
}

async fn add_video_channel(
    State(state): State<PulseState>,
    Json(body): Json<AddVideoChannelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body.channel_id.is_empty() || body.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Channel ID and name are required".to_string(),
        ));
    }
    if body.platform != "youtube" && body.platform != "bilibili" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Platform must be 'youtube' or 'bilibili'".to_string(),
        ));
    }
    state
        .db
        .add_video_channel(&body.platform, &body.channel_id, &body.name)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "status": "added" })))
}

async fn remove_video_channel(
    State(state): State<PulseState>,
    Path((platform, channel_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state
        .db
        .remove_video_channel(&platform, &channel_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "status": "removed" })))
}
