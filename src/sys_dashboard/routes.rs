use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::AppState;
use crate::config::ProviderDef;

#[derive(Debug, Deserialize)]
pub struct RecentQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderQuery {
    pub provider: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportKeyRequest {
    pub provider: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct SwitchProfileRequest {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct ProviderNameRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddProviderRequest {
    pub name: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub thinking: bool,
    pub default_temperature: Option<f64>,
    pub default_top_p: Option<f64>,
    pub default_max_output_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub models: Option<Vec<String>>,
    pub thinking: Option<bool>,
    pub default_temperature: Option<f64>,
    pub default_top_p: Option<f64>,
    pub default_max_output_tokens: Option<u64>,
}

/// Redirect "/" to "/sys-dashboard"
async fn root_redirect() -> Response {
    Redirect::permanent("/sys-dashboard").into_response()
}

/// Serve the React SPA dashboard.
async fn dashboard_index() -> Response {
    let html = include_str!("../../frontend/dist/index.html");
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response()
}

// ── Provider management API ──

/// GET /v1/sys-dashboard/providers
async fn get_providers(State(state): State<AppState>) -> Json<Value> {
    let pm = state.provider_manager.lock().await;
    let providers = pm.list_providers();
    let profile = pm.active_profile();
    Json(json!({
        "active_provider": profile.provider,
        "active_model": profile.model,
        "key_hash": profile.key_hash,
        "providers": providers,
    }))
}

/// GET /v1/sys-dashboard/providers/diagnose?provider=...
async fn diagnose_provider_key(Query(query): Query<ProviderQuery>) -> Response {
    let provider = query.provider.trim();
    if provider.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Missing provider",
                "status": 422,
                "detail": "provider query parameter is required",
            })),
        )
            .into_response();
    }

    Json(json!(crate::keyring_store::diagnose_api_key(provider))).into_response()
}

/// POST /v1/sys-dashboard/providers/import-key
async fn import_key(
    State(state): State<AppState>,
    Json(payload): Json<ImportKeyRequest>,
) -> Response {
    let mut pm = state.provider_manager.lock().await;
    match pm.import_key(&payload.provider, &payload.api_key) {
        Ok(hash) => Json(json!({
            "status": "ok",
            "message": format!("API key imported for provider '{}'", payload.provider),
            "key_hash": hash,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Failed to import key",
                "status": 422,
                "detail": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// POST /v1/sys-dashboard/providers/switch
async fn switch_provider(
    State(state): State<AppState>,
    Json(payload): Json<SwitchProfileRequest>,
) -> Response {
    let mut pm = state.provider_manager.lock().await;
    match pm.switch(&payload.provider, &payload.model) {
        Ok(()) => Json(json!({
            "status": "ok",
            "message": format!("Switched to provider '{}' model '{}'", payload.provider, payload.model),
            "active_provider": payload.provider,
            "active_model": payload.model,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Failed to switch",
                "status": 422,
                "detail": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// POST /v1/sys-dashboard/providers/remove-key
async fn remove_key(
    State(state): State<AppState>,
    Json(payload): Json<ProviderNameRequest>,
) -> Response {
    let mut pm = state.provider_manager.lock().await;
    match pm.remove_key(&payload.name) {
        Ok(()) => Json(json!({
            "status": "ok",
            "message": format!("API key removed for provider '{}'", payload.name),
        }))
        .into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Failed to remove key",
                "status": 422,
                "detail": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// POST /v1/sys-dashboard/providers/add
async fn add_provider(
    State(state): State<AppState>,
    Json(payload): Json<AddProviderRequest>,
) -> Response {
    let mut models = payload.models.clone();
    if models.is_empty() {
        models.push(payload.model.clone());
    } else if !models.iter().any(|m| m == &payload.model) {
        models.insert(0, payload.model.clone());
    }

    let pdef = ProviderDef {
        name: payload.name.clone(),
        base_url: payload.base_url.clone(),
        model: payload.model.clone(),
        models,
        thinking: payload.thinking,
        default_temperature: payload.default_temperature,
        default_top_p: payload.default_top_p,
        default_max_output_tokens: payload.default_max_output_tokens,
    };

    let mut pm = state.provider_manager.lock().await;
    match pm.add_provider(pdef) {
        Ok(()) => Json(json!({
            "status": "ok",
            "message": format!("Provider '{}' added", payload.name),
        }))
        .into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Failed to add provider",
                "status": 422,
                "detail": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// POST /v1/sys-dashboard/providers/update
async fn update_provider(
    State(state): State<AppState>,
    Json(payload): Json<UpdateProviderRequest>,
) -> Response {
    let mut pm = state.provider_manager.lock().await;

    // Merge with existing.
    let existing = pm.provider_defs().iter().find(|p| p.name == payload.name).cloned();
    let Some(existing) = existing else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Provider not found",
                "status": 422,
                "detail": format!("provider '{}' not found", payload.name),
            })),
        )
            .into_response();
    };

    let mut models = payload.models.unwrap_or(existing.models.clone());
    let model = payload.model.unwrap_or(existing.model.clone());
    if models.is_empty() {
        models.push(model.clone());
    } else if !models.iter().any(|m| m == &model) {
        models.insert(0, model.clone());
    }

    let pdef = ProviderDef {
        name: payload.name.clone(),
        base_url: payload.base_url.unwrap_or(existing.base_url),
        model,
        models,
        thinking: payload.thinking.unwrap_or(existing.thinking),
        default_temperature: payload.default_temperature.or(existing.default_temperature),
        default_top_p: payload.default_top_p.or(existing.default_top_p),
        default_max_output_tokens: payload
            .default_max_output_tokens
            .or(existing.default_max_output_tokens),
    };

    match pm.update_provider(pdef) {
        Ok(()) => Json(json!({
            "status": "ok",
            "message": format!("Provider '{}' updated", payload.name),
        }))
        .into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Failed to update provider",
                "status": 422,
                "detail": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// POST /v1/sys-dashboard/providers/remove
async fn remove_provider(
    State(state): State<AppState>,
    Json(payload): Json<ProviderNameRequest>,
) -> Response {
    let mut pm = state.provider_manager.lock().await;
    match pm.remove_provider(&payload.name) {
        Ok(()) => Json(json!({
            "status": "ok",
            "message": format!("Provider '{}' removed", payload.name),
        }))
        .into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Failed to remove provider",
                "status": 422,
                "detail": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// POST /v1/sys-dashboard/providers/save-config
async fn save_config(State(state): State<AppState>) -> Response {
    let pm = state.provider_manager.lock().await;
    let config_source = state.config.config_source.as_deref();
    match pm.save_config_snapshot(config_source) {
        Ok(checksum) => Json(json!({
            "status": "ok",
            "message": "Configuration saved and validated.",
            "checksum": checksum,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Failed to save config",
                "status": 422,
                "detail": e.to_string(),
            })),
        )
            .into_response(),
    }
}

// ── Dashboard data API ──

/// GET /v1/sys-dashboard/summary
async fn get_summary(State(state): State<AppState>) -> Json<Value> {
    let store = state.usage_store.lock().await;
    Json(json!(store.summary()))
}

/// GET /v1/sys-dashboard/model-stats
///
/// All data now comes from `ModelStatsStore`, which is seeded from `token_usage.bin`
/// on startup and updated incrementally at runtime. The bin file stores
/// cached_tokens, reasoning_tokens, provider_model, and model_route, so
/// all summary fields and per-model stats survive restarts.
async fn get_model_stats(State(state): State<AppState>) -> Json<Value> {
    let ms = state.model_stats.lock().await;
    Json(json!({
        "summary": ms.summary(),
        "models": ms.per_model(),
    }))
}

/// GET /v1/sys-dashboard/requests?limit=20
async fn get_requests(
    State(state): State<AppState>,
    Query(query): Query<RecentQuery>,
) -> Json<Value> {
    let limit = query.limit.unwrap_or(20).min(10000);
    let trc = state.trc_store.lock().await;
    let records = trc.recent(limit);
    Json(super::trc_store::trc_to_json(&records))
}

/// GET /v1/sys-dashboard/config
async fn get_config(State(state): State<AppState>) -> Response {
    let editor = &state.config_editor;
    match editor.read() {
        Some(content) => Json(json!(content)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "type": "https://api.example.com/problems/not-found",
                "title": "Config file not found",
                "status": 404,
                "detail": "config.toml does not exist",
            })),
        )
            .into_response(),
    }
}

/// PUT /v1/sys-dashboard/config
async fn put_config(State(state): State<AppState>, Json(payload): Json<Value>) -> Response {
    let content = match payload.get("content").and_then(Value::as_str) {
        Some(c) => c,
        None => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "type": "https://api.example.com/problems/validation-error",
                    "title": "Invalid request body",
                    "status": 422,
                    "detail": "Missing 'content' field (string)",
                })),
            )
                .into_response();
        }
    };

    let editor = &state.config_editor;
    match editor.write(content) {
        Ok(saved) => Json(json!({
            "status": "ok",
            "message": "Configuration saved successfully.",
            "checksum": saved.checksum,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "type": "https://api.example.com/problems/validation-error",
                "title": "Invalid configuration",
                "status": 422,
                "detail": e,
            })),
        )
            .into_response(),
    }
}

/// GET /v1/sys-dashboard/version
async fn get_version(State(state): State<AppState>) -> Json<Value> {
    let pm = state.provider_manager.lock().await;
    let profile = pm.active_profile();
    Json(json!({
        "app_name": "codex-shim",
        "version": env!("CARGO_PKG_VERSION"),
        "config_source": state.config.config_source.as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
        "bind": state.config.bind.to_string(),
        "active_provider": profile.provider,
        "active_model": profile.model,
    }))
}

pub fn dashboard_router() -> Router<AppState> {
    Router::new()
        .route("/", get(root_redirect))
        // React SPA routes
        .route("/sys-dashboard", get(dashboard_index))
        .route("/sys-dashboard/trc", get(dashboard_index))
        .route("/sys-dashboard/providers", get(dashboard_index))
        .route("/sys-dashboard/config", get(dashboard_index))
        // Provider management
        .route("/v1/sys-dashboard/providers", get(get_providers))
        .route(
            "/v1/sys-dashboard/providers/diagnose",
            get(diagnose_provider_key),
        )
        .route("/v1/sys-dashboard/providers/import-key", post(import_key))
        .route("/v1/sys-dashboard/providers/switch", post(switch_provider))
        .route("/v1/sys-dashboard/providers/remove-key", post(remove_key))
        .route("/v1/sys-dashboard/providers/add", post(add_provider))
        .route("/v1/sys-dashboard/providers/update", post(update_provider))
        .route("/v1/sys-dashboard/providers/remove", post(remove_provider))
        .route("/v1/sys-dashboard/providers/save-config", post(save_config))
        // Generation parameters (temperature, top-p)
        .merge(super::generation_params::generation_params_router())
        // Dashboard data
        .route("/v1/sys-dashboard/summary", get(get_summary))
        .route("/v1/sys-dashboard/model-stats", get(get_model_stats))
        .route("/v1/sys-dashboard/requests", get(get_requests))
        .route("/v1/sys-dashboard/config", get(get_config).put(put_config))
        .route("/v1/sys-dashboard/version", get(get_version))
}
