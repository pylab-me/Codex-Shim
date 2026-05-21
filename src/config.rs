use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs};

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind: SocketAddr,
    pub mimo_base_url: String,
    pub mimo_api_key: String,
    pub mimo_model: String,
    pub mimo_models: Vec<String>,
    pub model_aliases: HashMap<String, String>,
    pub fallback_unknown_model_to_default: bool,
    pub request_timeout: Duration,
    pub access_log: bool,
    pub response_store_max: usize,
    pub trust_env: bool,
    pub http2_prior_knowledge: bool,
    pub user_agent: String,
    pub forward_parallel_tool_calls: bool,
    pub default_temperature: Option<f64>,
    pub default_top_p: Option<f64>,
    pub default_max_output_tokens: Option<u64>,
    pub thinking_mode_enabled: bool,
    pub log_level: String,
    pub config_source: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct FileConfig {
    server: ServerConfig,
    upstream: UpstreamConfig,
    generation: GenerationConfig,
    http: HttpConfig,
    behavior: BehaviorConfig,
    log: LogConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ServerConfig {
    host: String,
    port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 33300,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct UpstreamConfig {
    base_url: String,
    api_key: String,
    model: String,
    models: Vec<String>,
    aliases: HashMap<String, String>,
    fallback_unknown_model_to_default: bool,
    thinking: ThinkingConfig,
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            base_url: "https://token-plan-cn.xiaomimimo.com/v1".to_string(),
            api_key: "NO-KEY".to_string(),
            model: "mimo-v2.5".to_string(),
            models: Vec::new(),
            aliases: HashMap::new(),
            fallback_unknown_model_to_default: true,
            thinking: ThinkingConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct GenerationConfig {
    default_temperature: Option<f64>,
    default_top_p: Option<f64>,
    default_max_output_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ThinkingConfig {
    /// Explicitly request Xiaomi MiMo thinking mode via
    /// `thinking: {"type":"enabled"}` in the upstream Chat Completions body.
    ///
    /// When enabled, the shim also stores and replays assistant
    /// `reasoning_content`. When disabled, the shim does not persist or send
    /// that provider-specific field.
    enabled: bool,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct HttpConfig {
    request_timeout_secs: u64,
    trust_env: bool,
    http2_prior_knowledge: bool,
    user_agent: String,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            request_timeout_secs: 300,
            trust_env: false,
            http2_prior_knowledge: false,
            // -CHANGE-VERSION-BEFORE-RELEASE
            user_agent: "Pokaemon/v_Alpha_2026 codex-mimo-shim/0.1.9".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct BehaviorConfig {
    access_log: bool,
    response_store_max: usize,
    forward_parallel_tool_calls: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            access_log: true,
            response_store_max: 1000,
            forward_parallel_tool_calls: true,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct LogConfig {
    level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "codex_mimo_shim=info,tower_http=warn".to_string(),
        }
    }
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = resolve_config_path();
        let raw = match config_path.as_deref() {
            Some(path) => load_file_config(path)?,
            None => FileConfig::default(),
        };

        let host: IpAddr = raw.server.host.parse()?;
        let bind = SocketAddr::new(host, raw.server.port);

        let upstream = raw.upstream;
        let thinking_mode_enabled = upstream.thinking.enabled;
        let mimo_api_key = sensitive_api_key_override().unwrap_or(upstream.api_key);
        let mimo_model = upstream.model;
        let mut mimo_models = upstream.models;
        if mimo_models.is_empty() {
            mimo_models.push(mimo_model.clone());
        } else if !mimo_models.iter().any(|model| model == &mimo_model) {
            mimo_models.insert(0, mimo_model.clone());
        }
        let model_aliases = upstream.aliases;
        let fallback_unknown_model_to_default = upstream.fallback_unknown_model_to_default;

        Ok(Self {
            bind,
            mimo_base_url: upstream.base_url.trim_end_matches('/').to_string(),
            mimo_api_key,
            mimo_model,
            mimo_models,
            model_aliases,
            fallback_unknown_model_to_default,
            request_timeout: Duration::from_secs(raw.http.request_timeout_secs),
            access_log: raw.behavior.access_log,
            response_store_max: raw.behavior.response_store_max,
            trust_env: raw.http.trust_env,
            http2_prior_knowledge: raw.http.http2_prior_knowledge,
            user_agent: raw.http.user_agent,
            forward_parallel_tool_calls: raw.behavior.forward_parallel_tool_calls,
            default_temperature: raw.generation.default_temperature,
            default_top_p: raw.generation.default_top_p,
            default_max_output_tokens: raw.generation.default_max_output_tokens,
            thinking_mode_enabled,
            log_level: raw.log.level,
            config_source: config_path,
        })
    }
}

fn resolve_config_path() -> Option<PathBuf> {
    if let Some(path) = config_path_from_args() {
        return Some(path);
    }

    ["config.yaml", "config.yml", "config.json"]
        .iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
}

fn config_path_from_args() -> Option<PathBuf> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--config" || arg == "-c" {
            return args.next().map(PathBuf::from);
        }
        if let Some(path) = arg.strip_prefix("--config=") {
            return Some(PathBuf::from(path));
        }
    }
    None
}

fn load_file_config(path: &Path) -> anyhow::Result<FileConfig> {
    let content = fs::read_to_string(path)?;
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "json" => Ok(serde_json::from_str(&content)?),
        "yaml" | "yml" => Ok(serde_yaml::from_str(&content)?),
        other => anyhow::bail!(
            "unsupported config extension {:?}; use .yaml, .yml, or .json",
            other
        ),
    }
}

fn sensitive_api_key_override() -> Option<String> {
    ["MIMO_API_KEY", "AK_XIAOMIMIMO_TKP", "XIAOMIMIMO_API_KEY"]
        .iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
}
