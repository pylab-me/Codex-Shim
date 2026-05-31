use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SetTemperatureRequest {
    pub temperature: f64,
}

#[derive(Debug, Deserialize)]
pub struct SetTopPRequest {
    pub top_p: f64,
}

/// GET /v1/sys-dashboard/generation-params
async fn get_generation_params(State(state): State<AppState>) -> Json<Value> {
    let params = state.generation_params.lock().await;
    Json(json!({
        "temperature": params.temperature,
        "top_p": params.top_p,
    }))
}

/// POST /v1/sys-dashboard/generation-params/temperature
async fn set_temperature(
    State(state): State<AppState>,
    Json(payload): Json<SetTemperatureRequest>,
) -> Response {
    let temp = payload.temperature;
    if !(0.0..=2.0).contains(&temp) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Invalid temperature",
                "status": 422,
                "detail": "temperature must be between 0.0 and 2.0",
            })),
        )
            .into_response();
    }

    // Persist to config file
    if let Err(e) = persist_generation_param(&state, "default_temperature", &temp.to_string()) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "type": "https://api.example.com/problems/internal-error",
                "title": "Failed to persist temperature",
                "status": 500,
                "detail": e.to_string(),
            })),
        )
            .into_response();
    }

    // Update in-memory shared state
    {
        let mut params = state.generation_params.lock().await;
        params.temperature = Some(temp);
    }

    Json(json!({
        "status": "ok",
        "message": format!("Temperature set to {temp}"),
        "temperature": temp,
    }))
    .into_response()
}

/// POST /v1/sys-dashboard/generation-params/top-p
async fn set_top_p(State(state): State<AppState>, Json(payload): Json<SetTopPRequest>) -> Response {
    let top_p = payload.top_p;
    if !(0.0..=1.0).contains(&top_p) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Invalid top_p",
                "status": 422,
                "detail": "top_p must be between 0.0 and 1.0",
            })),
        )
            .into_response();
    }

    if let Err(e) = persist_generation_param(&state, "default_top_p", &top_p.to_string()) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "type": "https://api.example.com/problems/internal-error",
                "title": "Failed to persist top_p",
                "status": 500,
                "detail": e.to_string(),
            })),
        )
            .into_response();
    }

    // Update in-memory shared state
    {
        let mut params = state.generation_params.lock().await;
        params.top_p = Some(top_p);
    }

    Json(json!({
        "status": "ok",
        "message": format!("Top-p set to {top_p}"),
        "top_p": top_p,
    }))
    .into_response()
}

/// Persist a generation parameter to the config file.
fn persist_generation_param(state: &AppState, key: &str, value: &str) -> Result<(), String> {
    let config_path = state
        .config
        .config_source
        .as_ref()
        .ok_or_else(|| "no config file path".to_string())?;

    let content =
        std::fs::read_to_string(config_path).map_err(|e| format!("failed to read config: {e}"))?;

    // Simple TOML value replacement — works for our known config structure
    let new_content = if content.contains(&format!("{key} =")) {
        // Replace existing value
        let pattern = format!(r#"{key}\s*=\s*[^\n]*"#);
        let replacement = format!("{key} = {value}");
        regex::Regex::new(&pattern)
            .map_err(|e| format!("regex error: {e}"))?
            .replace(&content, replacement.as_str())
            .into_owned()
    } else {
        // Add to [generation] section
        if let Some(idx) = content.find("[generation]") {
            let (_before, after) = content.split_at(idx);
            let insert_pos = after.find('\n').map(|i| idx + i + 1).unwrap_or(content.len());
            let (before_insert, after_insert) = content.split_at(insert_pos);
            format!("{before_insert}{key} = {value}\n{after_insert}")
        } else {
            format!("{content}\n[generation]\n{key} = {value}\n")
        }
    };

    std::fs::write(config_path, new_content).map_err(|e| format!("failed to write config: {e}"))?;

    Ok(())
}

pub fn generation_params_router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/sys-dashboard/generation-params",
            get(get_generation_params),
        )
        .route(
            "/v1/sys-dashboard/generation-params/temperature",
            post(set_temperature),
        )
        .route("/v1/sys-dashboard/generation-params/top-p", post(set_top_p))
}
