pub mod config;
pub mod error;
pub mod keyring_store;
pub mod provider;
pub mod responses;
pub mod sys_dashboard;
pub mod xiaomi;

// Re-export public types and helpers
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{ConnectInfo, OriginalUri, Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
pub use config::{AppConfig, GenerationDefaults};
pub use error::ShimError;
use provider::manager::ProviderManager;
use responses::build::build_response_object;
use responses::convert::{ConversionDefaults, convert_responses_to_chat};
use responses::sse::{buffered_sse_events, failed_sse_events};
use responses::store::{ResponseStore, StoredResponse};
use serde_json::{Value, json};
use sys_dashboard::TokenUsageStore;
use sys_dashboard::model_stats::ModelStatsStore;
use sys_dashboard::trc_store::TrcStore;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::info;
use uuid::Uuid;
use xiaomi::reasoning::ReasoningPolicy;
use xiaomi::schema::ProviderTokenStats;

/// Shared mutable generation parameters (temperature, top-p).
/// Updated by the dashboard API and read by request handlers.
#[derive(Debug, Clone, Default)]
pub struct GenerationParamsState {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
}

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub provider_manager: Arc<Mutex<ProviderManager>>,
    store: ResponseStore,
    pub usage_store: Arc<Mutex<TokenUsageStore>>,
    pub config_editor: Arc<sys_dashboard::config_editor::ConfigEditor>,
    pub trc_store: Arc<Mutex<TrcStore>>,
    pub model_stats: Arc<Mutex<ModelStatsStore>>,
    pub generation_params: Arc<Mutex<GenerationParamsState>>,
}

pub async fn create_state(config: AppConfig) -> anyhow::Result<AppState> {
    let store = ResponseStore::new(config.response_store_max);

    let state_dir = resolve_state_dir();
    let usage_store = TokenUsageStore::open(&state_dir)?;

    // Seed ModelStatsStore with per-model history from the bin file.
    let bin_model_entries = usage_store.per_model();
    let mut model_stats = ModelStatsStore::new();
    model_stats.seed_from_bin(&bin_model_entries);

    let config_editor = sys_dashboard::config_editor::ConfigEditor::new(
        config.config_source.as_ref().and_then(|p| p.parent()),
    );

    let provider_manager =
        ProviderManager::new(config.providers.clone(), &state_dir, &config.user_agent)?;

    let generation_params = GenerationParamsState {
        temperature: config.default_temperature,
        top_p: config.default_top_p,
    };

    Ok(AppState {
        config: config.clone(),
        provider_manager: Arc::new(Mutex::new(provider_manager)),
        store,
        usage_store: Arc::new(Mutex::new(usage_store)),
        config_editor: Arc::new(config_editor),
        trc_store: Arc::new(Mutex::new(TrcStore::new())),
        model_stats: Arc::new(Mutex::new(model_stats)),
        generation_params: Arc::new(Mutex::new(generation_params)),
    })
}

/// Resolve the state directory.
/// Priority: `--state-dir` CLI arg > `dirs::data_dir()/codex-shim`.
pub fn resolve_state_dir() -> std::path::PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--state-dir" {
            if let Some(path) = args.next() {
                return std::path::PathBuf::from(path);
            }
        }
        if let Some(path) = arg.strip_prefix("--state-dir=") {
            return std::path::PathBuf::from(path);
        }
    }
    // Platform-standard data directory.
    dirs::data_dir()
        .map(|d| d.join("codex-shim"))
        .unwrap_or_else(|| std::path::PathBuf::from("state"))
}

pub fn init_tracing(filter: &str) {
    let filter = filter.to_string();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .with_target(false)
        .init();
}

pub const VERSION_FLAG: &str = "--version";
pub const BUILD_INFO_FLAG: &str = "--build-info";

pub fn handle_process_metadata_command() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    let flag = args.next()?;

    match flag.as_str() {
        VERSION_FLAG => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            print_build_info();
            Some(0)
        }
        BUILD_INFO_FLAG => {
            print_build_info();
            Some(0)
        }
        _ => None,
    }
}

pub fn print_build_info() {
    println!(
        "cargo_lock_sha256={}",
        option_env!("BUILD_CARGO_LOCK_SHA256").unwrap_or("unknown")
    );
    println!(
        "ci_workflow_sha256={}",
        option_env!("BUILD_CI_WORKFLOW_SHA256").unwrap_or("unknown")
    );
    println!(
        "public_build_rs_sha256={}",
        option_env!("BUILD_PUBLIC_BUILD_RS_SHA256").unwrap_or("unknown")
    );
}

// ── Internal types ──

struct CreatedResponse {
    response_object: Value,
    provider_ms: u128,
    observed_token_stats: ProviderTokenStats,
    client_model: String,
    provider_model: String,
    model_route: String,
}

#[derive(Debug, Clone)]
struct ResolvedModel {
    client_model: String,
    provider_model: String,
    route: &'static str,
}

// ── Handlers ──

async fn healthz() -> Json<Value> {
    Json(json!({"status":"ok"}))
}

async fn list_models(State(state): State<AppState>) -> Json<Value> {
    let config = &state.config;
    let mut ids = Vec::new();
    for pdef in &config.providers {
        for id in &pdef.models {
            append_unique_model_id(&mut ids, id);
        }
    }
    for id in config.model_aliases.keys() {
        append_unique_model_id(&mut ids, id);
    }
    let data: Vec<Value> = ids
        .iter()
        .map(|id| json!({"id": id, "object": "model", "created": 0, "owned_by": "codex-shim"}))
        .collect();
    Json(json!({"object":"list", "data": data}))
}

fn append_unique_model_id(ids: &mut Vec<String>, id: &str) {
    if !id.trim().is_empty() && !ids.iter().any(|existing| existing == id) {
        ids.push(id.to_string());
    }
}

async fn compact_unsupported(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    let started = Instant::now();
    let stream = payload.get("stream").and_then(Value::as_bool).unwrap_or(false);
    let trace_id = resolve_trace_id(&payload, &headers);
    let request_model = extract_request_model(&payload);
    let path = uri.path().to_string();

    access_in(&state, addr, &path, stream, &trace_id);

    let err = ShimError::UnsupportedFeature {
        code: "compact_not_supported",
        message: "/v1/responses/compact is a Codex remote compaction control-plane request; codex-shim does not emulate compact".to_string(),
    };
    let status = err.status();
    let elapsed_ms = started.elapsed().as_millis();
    let error_msg = err.to_string();

    record_usage(&state, &request_model, elapsed_ms, false, None, "", "").await;
    record_trc(
        &state,
        &trace_id,
        request_model.as_deref(),
        "",
        "",
        None,
        0,
        elapsed_ms as u64,
        status.as_u16(),
        &error_msg,
    )
    .await;
    access_out(
        &state,
        addr,
        &path,
        stream,
        &trace_id,
        request_model.as_deref(),
        None,
        Some("unsupported_codex_control_plane_request"),
        None,
        0,
        elapsed_ms,
        status.as_u16(),
        Some(&error_msg),
    );

    err.into_response()
}

async fn get_response(
    State(state): State<AppState>,
    Path(response_id): Path<String>,
) -> Result<Json<Value>, ShimError> {
    let record = state
        .store
        .get(&response_id)
        .await
        .ok_or_else(|| ShimError::ResponseNotFound(response_id.clone()))?;
    Ok(Json(record.response_object))
}

async fn create_response(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    let started = Instant::now();
    let stream = payload.get("stream").and_then(Value::as_bool).unwrap_or(false);
    let trace_id = resolve_trace_id(&payload, &headers);
    let request_model = extract_request_model(&payload);
    let path = uri.path().to_string();

    access_in(&state, addr, &path, stream, &trace_id);

    let result = create_response_inner(state.clone(), payload, trace_id.clone()).await;
    let elapsed_ms = started.elapsed().as_millis();

    match result {
        Ok(created) => {
            let stats = &created.observed_token_stats;
            let (pt, ct, tt, ctok, rt) = extract_token_counts(stats);

            record_usage(
                &state,
                &Some(created.client_model.clone()),
                elapsed_ms,
                true,
                Some(stats),
                &created.provider_model,
                &created.model_route,
            )
            .await;
            record_trc(
                &state,
                &trace_id,
                Some(&created.client_model),
                &created.provider_model,
                &created.model_route,
                Some(stats),
                0,
                elapsed_ms as u64,
                200,
                "",
            )
            .await;

            {
                let mut ms = state.model_stats.lock().await;
                ms.record(
                    &created.provider_model,
                    &created.model_route,
                    pt,
                    ct,
                    tt,
                    ctok,
                    rt,
                    created.provider_ms as u64,
                    elapsed_ms as u64,
                    200,
                    "",
                );
            }

            access_out(
                &state,
                addr,
                &path,
                stream,
                &trace_id,
                Some(&created.client_model),
                Some(&created.provider_model),
                Some(&created.model_route),
                Some(stats),
                created.provider_ms,
                elapsed_ms,
                200,
                None,
            );

            if stream {
                sse_response(buffered_sse_events(&created.response_object))
            } else {
                Json(created.response_object).into_response()
            }
        }
        Err(err) => {
            let status = err.status();
            let error_msg = err.to_string();

            record_usage(&state, &request_model, elapsed_ms, false, None, "", "").await;
            record_trc(
                &state,
                &trace_id,
                request_model.as_deref(),
                "",
                "",
                None,
                0,
                elapsed_ms as u64,
                status.as_u16(),
                &error_msg,
            )
            .await;
            access_out(
                &state,
                addr,
                &path,
                stream,
                &trace_id,
                request_model.as_deref(),
                None,
                None,
                None,
                0,
                elapsed_ms,
                status.as_u16(),
                Some(&error_msg),
            );

            if stream {
                sse_response(failed_sse_events("resp_local_failed", &err))
            } else {
                err.into_response()
            }
        }
    }
}

fn extract_token_counts(stats: &ProviderTokenStats) -> (u64, u64, u64, u64, u64) {
    let pt = stats.input_tokens.unwrap_or(0);
    let ct = stats.output_tokens.unwrap_or(0);
    let tt = stats.total_tokens.unwrap_or_else(|| pt + ct);
    let ctok = stats.cached_tokens.unwrap_or(0);
    let rt = stats.reasoning_tokens.unwrap_or(0);
    (pt, ct, tt, ctok, rt)
}

async fn record_usage(
    state: &AppState,
    model: &Option<String>,
    total_ms: u128,
    success: bool,
    token_stats: Option<&ProviderTokenStats>,
    provider_model: &str,
    model_route: &str,
) {
    let (prompt_tokens, completion_tokens, total_tokens, cached_tokens, reasoning_tokens) =
        match token_stats {
            Some(stats) => (
                stats.input_tokens.unwrap_or(0),
                stats.output_tokens.unwrap_or(0),
                stats.total_tokens.unwrap_or_else(|| {
                    stats.input_tokens.unwrap_or(0) + stats.output_tokens.unwrap_or(0)
                }),
                stats.cached_tokens.unwrap_or(0),
                stats.reasoning_tokens.unwrap_or(0),
            ),
            None => (0, 0, 0, 0, 0),
        };
    let input = sys_dashboard::usage_record::TokenUsageInput {
        prompt_tokens,
        completion_tokens,
        total_tokens,
        request_time_ms: total_ms as u64,
        cached_tokens,
        reasoning_tokens,
        model: model.clone().unwrap_or_default(),
        provider_model: provider_model.to_string(),
        model_route: model_route.to_string(),
        success,
    };
    let mut store = state.usage_store.lock().await;
    if let Err(e) = store.record_usage(input) {
        tracing::warn!("Failed to record usage: {e}");
    }
}

async fn record_trc(
    state: &AppState,
    trace_id: &str,
    client_model: Option<&str>,
    provider_model: &str,
    model_route: &str,
    token_stats: Option<&ProviderTokenStats>,
    provider_ms: u64,
    total_ms: u64,
    status_code: u16,
    error: &str,
) {
    let (pt, ct, tt, ctok, rt) = match token_stats {
        Some(stats) => extract_token_counts(stats),
        None => (0, 0, 0, 0, 0),
    };
    let mut trc = state.trc_store.lock().await;
    trc.record(
        trace_id.to_string(),
        client_model.unwrap_or_default().to_string(),
        provider_model.to_string(),
        model_route.to_string(),
        pt,
        ct,
        tt,
        ctok,
        rt,
        provider_ms,
        total_ms,
        status_code,
        error.to_string(),
        sys_dashboard::trc_store::now_iso8601(),
    );
}

async fn create_response_inner(
    state: AppState,
    payload: Value,
    trace_id: String,
) -> Result<CreatedResponse, ShimError> {
    let previous_id = payload.get("previous_response_id").and_then(Value::as_str);
    let previous_messages = if let Some(id) = previous_id {
        let record = state
            .store
            .get(id)
            .await
            .ok_or_else(|| ShimError::ResponseNotFound(id.to_string()))?;
        Some(record.chat_messages)
    } else {
        None
    };

    let requested_model = extract_request_model(&payload);
    let resolved_model = {
        let pm = state.provider_manager.lock().await;
        resolve_provider_model(&pm, requested_model.as_deref())?
    };

    let generation_defaults = {
        let pm = state.provider_manager.lock().await;
        // Merge: provider-level overrides > shared generation params state > config defaults
        let provider_def = pm.active_provider_def().ok();
        let shared = state.generation_params.lock().await;
        GenerationDefaults {
            temperature: provider_def
                .and_then(|p| p.default_temperature)
                .or(shared.temperature)
                .or(state.config.default_temperature),
            top_p: provider_def
                .and_then(|p| p.default_top_p)
                .or(shared.top_p)
                .or(state.config.default_top_p),
            max_output_tokens: provider_def
                .and_then(|p| p.default_max_output_tokens)
                .or(state.config.default_max_output_tokens),
        }
    };
    let defaults = ConversionDefaults {
        temperature: generation_defaults.temperature,
        top_p: generation_defaults.top_p,
        max_output_tokens: generation_defaults.max_output_tokens,
    };
    let reasoning_policy = ReasoningPolicy {
        thinking_enabled: state.config.thinking_mode_enabled,
    };

    let converted = convert_responses_to_chat(
        &payload,
        previous_messages,
        &resolved_model.client_model,
        &resolved_model.provider_model,
        state.config.forward_parallel_tool_calls,
        &defaults,
        reasoning_policy,
    )?;

    // Use the active provider client.
    let provider_started = Instant::now();
    let chat_result = {
        let pm = state.provider_manager.lock().await;
        let client = pm.active_client()?;
        client.chat_completions(converted.chat_payload.clone()).await?
    };
    let provider_ms = provider_started.elapsed().as_millis();
    let observed_token_stats = ProviderTokenStats::from_usage(chat_result.usage.as_ref());

    let mut built = build_response_object(
        &converted.response_id,
        &converted.client_model,
        &converted.chat_messages,
        chat_result,
        converted.parallel_tool_calls,
        converted.store,
        reasoning_policy,
    )?;

    if let Some(obj) = built.response_object.get_mut("local_gateway").and_then(Value::as_object_mut)
    {
        obj.insert("trace_id".to_string(), json!(trace_id));
        obj.insert("provider_ms".to_string(), json!(provider_ms));
        obj.insert(
            "client_model".to_string(),
            json!(converted.client_model.clone()),
        );
        obj.insert(
            "provider_model".to_string(),
            json!(converted.provider_model.clone()),
        );
        obj.insert("model_route".to_string(), json!(resolved_model.route));
    }

    state
        .store
        .put(
            converted.response_id,
            StoredResponse {
                response_object: built.response_object.clone(),
                chat_messages: built.updated_chat_messages,
            },
        )
        .await;

    Ok(CreatedResponse {
        response_object: built.response_object,
        provider_ms,
        observed_token_stats,
        client_model: converted.client_model,
        provider_model: converted.provider_model,
        model_route: resolved_model.route.to_string(),
    })
}

fn sse_response(events: Vec<String>) -> Response {
    let body = events.concat();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header("x-accel-buffering", "no")
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub fn resolve_trace_id(payload: &Value, headers: &HeaderMap) -> String {
    extract_trace_id(payload, headers)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("TRC-{}", Uuid::new_v4().simple().to_string().to_uppercase()))
}

fn extract_trace_id(payload: &Value, headers: &HeaderMap) -> Option<String> {
    payload
        .get("metadata")
        .and_then(|m| m.get("trace_id"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| payload.get("trace_id").and_then(Value::as_str).map(ToOwned::to_owned))
        .or_else(|| {
            payload
                .get("metadata")
                .and_then(|m| m.get("request_id"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| payload.get("request_id").and_then(Value::as_str).map(ToOwned::to_owned))
        .or_else(|| {
            headers.get("x-request-id").and_then(|v| v.to_str().ok()).map(ToOwned::to_owned)
        })
}

fn access_in(state: &AppState, _addr: SocketAddr, path: &str, stream: bool, trace_id: &str) {
    if !state.config.access_log {
        return;
    }
    info!(
        unstream = !stream,
        path = path,
        trace_id = trace_id,
        "CODEX-SHIM-IN"
    );
}

fn access_out(
    state: &AppState,
    _addr: SocketAddr,
    path: &str,
    stream: bool,
    trace_id: &str,
    model: Option<&str>,
    provider_model: Option<&str>,
    model_route: Option<&str>,
    token_stats: Option<&ProviderTokenStats>,
    provider_ms: u128,
    total_ms: u128,
    status: u16,
    error: Option<&str>,
) {
    if !state.config.access_log {
        return;
    }
    let model = model.unwrap_or("-");
    let provider_model = provider_model.unwrap_or("-");
    let model_route = model_route.unwrap_or("-");
    let obs_usage = if token_stats.map(ProviderTokenStats::has_any).unwrap_or(false) {
        "provider_usage"
    } else {
        "-"
    };
    let obs_input_tokens = format_optional_u64(token_stats.and_then(|s| s.input_tokens));
    let obs_output_tokens = format_optional_u64(token_stats.and_then(|s| s.output_tokens));
    let obs_total_tokens = format_optional_u64(token_stats.and_then(|s| s.total_tokens));
    let obs_cached_tokens = format_optional_u64(token_stats.and_then(|s| s.cached_tokens));
    let obs_reasoning_tokens = format_optional_u64(token_stats.and_then(|s| s.reasoning_tokens));
    info!(
        unstream = !stream,
        path = path,
        trace_id = trace_id,
        model = model,
        provider_model = provider_model,
        model_route = model_route,
        obs_usage = obs_usage,
        obs_input_tokens = obs_input_tokens.as_str(),
        obs_output_tokens = obs_output_tokens.as_str(),
        obs_total_tokens = obs_total_tokens.as_str(),
        obs_cached_tokens = obs_cached_tokens.as_str(),
        obs_reasoning_tokens = obs_reasoning_tokens.as_str(),
        provider_ms = provider_ms,
        total_ms = total_ms,
        status = status,
        error = error.unwrap_or("-"),
        "CODEX-SHIM-OUT"
    );
}

fn resolve_provider_model(
    provider_manager: &ProviderManager,
    requested_model: Option<&str>,
) -> Result<ResolvedModel, ShimError> {
    let requested_model = requested_model.and_then(|m| {
        let t = m.trim();
        if t.is_empty() { None } else { Some(t) }
    });

    let client_model = requested_model
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| provider_manager.active_model().to_string());
    let provider_model = provider_manager.active_model().to_string();
    let route = if requested_model.is_some() {
        "active_provider_override_requested_model"
    } else {
        "active_provider_default_model"
    };

    Ok(ResolvedModel {
        client_model,
        provider_model,
        route,
    })
}

fn extract_request_model(payload: &Value) -> Option<String> {
    payload
        .get("model")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn format_optional_u64(value: Option<u64>) -> String {
    value.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string())
}

/// Build the full axum router with all API routes and dashboard routes.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/models", get(list_models))
        .route("/v1/responses", post(create_response))
        .route("/v1/responses/compact", post(compact_unsupported))
        .route("/v1/responses/{response_id}", get(get_response))
        .merge(sys_dashboard::dashboard_router())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
