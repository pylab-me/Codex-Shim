use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs};

use serde::Deserialize;

const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../config/config.default.toml");
const VERSION_PLACEHOLDER: &str = "__CODEX_SHIM_VERSION__";

/// Top-level application config loaded from config.toml.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind: SocketAddr,
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
    /// All configured providers.
    pub providers: Vec<ProviderDef>,
    /// Model aliases: client-facing name → provider_model name.
    pub model_aliases: HashMap<String, String>,
    /// Whether to fall back to the default model when an unknown model is requested.
    pub fallback_unknown_model_to_default: bool,
}

/// A single provider definition from config.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderDef {
    /// Unique provider name (e.g. "mimo", "ollama").
    pub name: String,
    /// Base URL for the provider's OpenAI-compatible API.
    pub base_url: String,
    /// Default model for this provider.
    pub model: String,
    /// All supported models.
    #[serde(default)]
    pub models: Vec<String>,
    /// Whether this provider supports thinking/reasoning mode.
    #[serde(default)]
    pub thinking: bool,
    /// Per-provider generation override: temperature.
    #[serde(default)]
    pub default_temperature: Option<f64>,
    /// Per-provider generation override: top_p.
    #[serde(default)]
    pub default_top_p: Option<f64>,
    /// Per-provider generation override: max_output_tokens.
    #[serde(default)]
    pub default_max_output_tokens: Option<u64>,
}

// ── TOML file structures ──

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct FileConfig {
    server: ServerSection,
    provider: ProviderSection,
    generation: GenerationSection,
    http: HttpSection,
    behavior: BehaviorSection,
    log: LogSection,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ServerSection {
    host: String,
    port: u16,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 33300,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ProviderSection {
    /// Model aliases: client name → provider model.
    #[serde(default)]
    aliases: HashMap<String, String>,
    /// Fall back to default model for unknown requests.
    #[serde(default = "default_true")]
    fallback_unknown_model_to_default: bool,
    /// Provider definitions.
    #[serde(default)]
    entries: Vec<ProviderDef>,
}

impl Default for ProviderSection {
    fn default() -> Self {
        Self {
            aliases: HashMap::new(),
            fallback_unknown_model_to_default: true,
            entries: vec![ProviderDef {
                name: "mimo".to_string(),
                base_url: "https://token-plan-cn.xiaomimimo.com/v1".to_string(),
                model: "mimo-v2.5".to_string(),
                models: vec!["mimo-v2.5".to_string()],
                thinking: false,
                default_temperature: None,
                default_top_p: None,
                default_max_output_tokens: None,
            }],
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct GenerationSection {
    default_temperature: Option<f64>,
    default_top_p: Option<f64>,
    default_max_output_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct HttpSection {
    request_timeout_secs: u64,
    trust_env: bool,
    http2_prior_knowledge: bool,
    user_agent: String,
}

impl Default for HttpSection {
    fn default() -> Self {
        Self {
            request_timeout_secs: 300,
            trust_env: false,
            http2_prior_knowledge: false,
            user_agent: default_user_agent(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct BehaviorSection {
    access_log: bool,
    response_store_max: usize,
    forward_parallel_tool_calls: bool,
}

impl Default for BehaviorSection {
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
struct LogSection {
    level: String,
}

impl Default for LogSection {
    fn default() -> Self {
        Self {
            level: "codex_shim=info,tower_http=warn".to_string(),
        }
    }
}

fn default_true() -> bool {
    true
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = resolve_config_path();
        let raw = match config_path.as_deref() {
            Some(path) => load_toml_config(path)?,
            None => FileConfig::default(),
        };

        let host: IpAddr = raw.server.host.parse()?;
        let bind = SocketAddr::new(host, raw.server.port);

        let providers = normalize_providers(raw.provider.entries);

        // Thinking mode is enabled if any provider declares it.
        let thinking_mode_enabled = providers.iter().any(|p| p.thinking);

        Ok(Self {
            bind,
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
            providers,
            model_aliases: raw.provider.aliases,
            fallback_unknown_model_to_default: raw.provider.fallback_unknown_model_to_default,
        })
    }

    /// Find a provider by name.
    pub fn get_provider(&self, name: &str) -> Option<&ProviderDef> {
        self.providers.iter().find(|p| p.name == name)
    }

    /// Resolve generation defaults for a specific provider.
    /// Provider-level values take precedence over global `[generation]` values.
    pub fn generation_defaults_for(&self, provider_name: &str) -> GenerationDefaults {
        let provider = self.get_provider(provider_name);
        GenerationDefaults {
            temperature: provider.and_then(|p| p.default_temperature).or(self.default_temperature),
            top_p: provider.and_then(|p| p.default_top_p).or(self.default_top_p),
            max_output_tokens: provider
                .and_then(|p| p.default_max_output_tokens)
                .or(self.default_max_output_tokens),
        }
    }

    /// Generate a default config.toml at the given path.
    pub fn generate_default(path: &Path) -> anyhow::Result<()> {
        let content = render_default_config_toml();
        fs::write(path, content)?;
        Ok(())
    }
}

/// Resolved generation defaults after merging provider-level overrides with global values.
#[derive(Debug, Clone, Default)]
pub struct GenerationDefaults {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_output_tokens: Option<u64>,
}

pub fn default_user_agent() -> String {
    format!(
        "Pokaemon/v_Alpha_2026 Codex-Shim/{}",
        env!("CARGO_PKG_VERSION")
    )
}

fn render_default_config_toml() -> String {
    DEFAULT_CONFIG_TEMPLATE.replace(VERSION_PLACEHOLDER, env!("CARGO_PKG_VERSION"))
}

/// Ensure each provider has at least its default model in the models list.
fn normalize_providers(entries: Vec<ProviderDef>) -> Vec<ProviderDef> {
    if entries.is_empty() {
        return ProviderSection::default().entries;
    }
    entries
        .into_iter()
        .map(|mut p| {
            if p.models.is_empty() {
                p.models.push(p.model.clone());
            } else if !p.models.iter().any(|m| m == &p.model) {
                p.models.insert(0, p.model.clone());
            }
            p
        })
        .collect()
}

fn resolve_config_path() -> Option<PathBuf> {
    if let Some(path) = config_path_from_args() {
        return Some(path);
    }
    PathBuf::from("config.toml").exists().then(|| PathBuf::from("config.toml"))
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

fn load_toml_config(path: &Path) -> anyhow::Result<FileConfig> {
    let content = fs::read_to_string(path)?;
    let config: FileConfig = toml::from_str(&content)?;
    Ok(config)
}
