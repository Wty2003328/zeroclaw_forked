use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use crate::pulse::PulseState;

pub fn routes() -> Router<PulseState> {
    Router::new()
        .route("/auth-url", get(get_auth_url))
        .route("/callback", get(handle_callback))
        .route("/events", get(get_events))
        .route("/status", get(get_status))
        .route("/disconnect", post(disconnect))
}

async fn get_auth_url(
    State(state): State<PulseState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let client_id = state
        .db
        .get_setting("google_client_id")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Google Client ID not configured. Set it in Settings > Data Sources.".to_string(),
        ))?;

    let redirect_uri = state
        .db
        .get_setting("google_redirect_uri")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .unwrap_or_else(|| "http://localhost:8080/api/calendar/callback".to_string());

    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("https://www.googleapis.com/auth/calendar.readonly"),
    );

    Ok(Json(serde_json::json!({ "url": url })))
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
}

async fn handle_callback(
    State(state): State<PulseState>,
    Query(query): Query<CallbackQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
    let client_id = state
        .db
        .get_setting("google_client_id")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::BAD_REQUEST, "Client ID not set".to_string()))?;

    let client_secret = state
        .db
        .get_setting("google_client_secret")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::BAD_REQUEST, "Client secret not set".to_string()))?;

    let redirect_uri = state
        .db
        .get_setting("google_redirect_uri")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .unwrap_or_else(|| "http://localhost:8080/api/calendar/callback".to_string());

    let client = reqwest::Client::new();
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", query.code.as_str()),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("redirect_uri", &redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Token exchange failed: {}", err),
        ));
    }

    let token: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    if let Some(at) = token.get("access_token").and_then(|v| v.as_str()) {
        let _ = state.db.set_setting("google_access_token", at).await;
    }
    if let Some(rt) = token.get("refresh_token").and_then(|v| v.as_str()) {
        let _ = state.db.set_setting("google_refresh_token", rt).await;
    }
    if let Some(exp) = token.get("expires_in").and_then(|v| v.as_u64()) {
        let expiry = chrono::Utc::now().timestamp() + exp as i64;
        let _ = state
            .db
            .set_setting("google_token_expiry", &expiry.to_string())
            .await;
    }

    Ok(Html(
        "<html><body><p>Connected! You can close this window.</p><script>window.close()</script></body></html>".to_string(),
    ))
}

async fn get_status(State(state): State<PulseState>) -> Json<serde_json::Value> {
    let connected = state
        .db
        .get_setting("google_access_token")
        .await
        .ok()
        .flatten()
        .is_some();

    Json(serde_json::json!({ "connected": connected }))
}

async fn disconnect(
    State(state): State<PulseState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    for key in &[
        "google_access_token",
        "google_refresh_token",
        "google_token_expiry",
    ] {
        let _ = state.db.set_setting(key, "").await;
    }
    Ok(Json(serde_json::json!({ "status": "disconnected" })))
}

async fn get_events(
    State(state): State<PulseState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let access_token = ensure_valid_token(&state)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

    let client = reqwest::Client::new();
    let now = chrono::Utc::now().to_rfc3339();

    let resp = client
        .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
        .bearer_auth(&access_token)
        .query(&[
            ("timeMin", now.as_str()),
            ("maxResults", "20"),
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
        ])
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Calendar API error: {}", err),
        ));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let events: Vec<serde_json::Value> = data
        .get("items")
        .and_then(|i| i.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let start = item.get("start").unwrap_or(&serde_json::Value::Null);
                    let end = item.get("end").unwrap_or(&serde_json::Value::Null);
                    let all_day = start.get("date").is_some();

                    serde_json::json!({
                        "id": item.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                        "title": item.get("summary").and_then(|v| v.as_str()).unwrap_or("(No title)"),
                        "start": start.get("dateTime").or(start.get("date")).and_then(|v| v.as_str()).unwrap_or(""),
                        "end": end.get("dateTime").or(end.get("date")).and_then(|v| v.as_str()).unwrap_or(""),
                        "all_day": all_day,
                        "location": item.get("location").and_then(|v| v.as_str()).unwrap_or(""),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Json(serde_json::json!({ "events": events })))
}

async fn ensure_valid_token(state: &PulseState) -> Result<String, String> {
    let token = state
        .db
        .get_setting("google_access_token")
        .await
        .map_err(|e| e.to_string())?
        .filter(|t| !t.is_empty())
        .ok_or("Not connected to Google Calendar")?;

    let expiry: i64 = state
        .db
        .get_setting("google_token_expiry")
        .await
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if chrono::Utc::now().timestamp() < expiry - 60 {
        return Ok(token);
    }

    // Refresh the token
    let refresh_token = state
        .db
        .get_setting("google_refresh_token")
        .await
        .map_err(|e| e.to_string())?
        .filter(|t| !t.is_empty())
        .ok_or("No refresh token available")?;

    let client_id = state
        .db
        .get_setting("google_client_id")
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Client ID not configured")?;

    let client_secret = state
        .db
        .get_setting("google_client_secret")
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Client secret not configured")?;

    let client = reqwest::Client::new();
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("refresh_token", refresh_token.as_str()),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let new_token = data
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("Failed to refresh token")?
        .to_string();

    let _ = state
        .db
        .set_setting("google_access_token", &new_token)
        .await;

    if let Some(exp) = data.get("expires_in").and_then(|v| v.as_u64()) {
        let expiry = chrono::Utc::now().timestamp() + exp as i64;
        let _ = state
            .db
            .set_setting("google_token_expiry", &expiry.to_string())
            .await;
    }

    Ok(new_token)
}
