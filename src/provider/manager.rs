use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::config::{ProviderDef, default_user_agent};
use crate::error::ShimError;
use crate::keyring_store;
use crate::provider::client::ProviderClient;

/// The active profile: which provider + model + key combination is currently live.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProfile {
    pub provider: String,
    pub model: String,
    pub key_hash: String,
}

/// Manages provider clients and the active profile selection.
pub struct ProviderManager {
    providers: Vec<ProviderDef>,
    clients: HashMap<String, ProviderClient>,
    active: ActiveProfile,
    profile_path: PathBuf,
}

impl ProviderManager {
    pub fn new(
        providers: Vec<ProviderDef>,
        state_dir: &std::path::Path,
        user_agent: &str,
    ) -> anyhow::Result<Self> {
        std::fs::create_dir_all(state_dir)?;

        let profile_path = state_dir.join("active_profile.toml");
        let active = load_active_profile(&profile_path).unwrap_or_else(|_| {
            let pdef = providers.first().cloned().unwrap_or_else(|| ProviderDef {
                name: "mimo".to_string(),
                base_url: "https://token-plan-cn.xiaomimimo.com/v1".to_string(),
                model: "mimo-v2.5".to_string(),
                models: vec!["mimo-v2.5".to_string()],
                thinking: false,
                default_temperature: None,
                default_top_p: None,
                default_max_output_tokens: None,
            });
            let key_hash = keyring_store::get_api_key(&pdef.name)
                .map(|k| keyring_store::short_hash(&k))
                .unwrap_or_default();
            ActiveProfile {
                provider: pdef.name.clone(),
                model: pdef.model.clone(),
                key_hash,
            }
        });

        let mut clients = HashMap::new();
        for pdef in &providers {
            let mut client = ProviderClient::new(&pdef.base_url, 300, user_agent, false, false)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            match keyring_store::get_api_key(&pdef.name) {
                Ok(key) => {
                    client.set_api_key(key);
                    info!(provider = %pdef.name, "Loaded API key from keyring");
                }
                Err(_) => {
                    warn!(provider = %pdef.name, "No API key in keyring; requests will fail until key is imported");
                }
            }
            clients.insert(pdef.name.clone(), client);
        }

        Ok(Self {
            providers,
            clients,
            active,
            profile_path,
        })
    }

    // ── Queries ──

    pub fn active_profile(&self) -> &ActiveProfile {
        &self.active
    }

    pub fn active_name(&self) -> &str {
        &self.active.provider
    }

    pub fn active_model(&self) -> &str {
        &self.active.model
    }

    pub fn active_client(&self) -> Result<&ProviderClient, ShimError> {
        self.clients.get(&self.active.provider).ok_or_else(|| {
            ShimError::InvalidRequest(format!(
                "no client for active provider '{}'",
                self.active.provider
            ))
        })
    }

    pub fn active_provider_def(&self) -> Result<&ProviderDef, ShimError> {
        self.providers.iter().find(|p| p.name == self.active.provider).ok_or_else(|| {
            ShimError::InvalidRequest(format!(
                "active provider '{}' not found in config",
                self.active.provider
            ))
        })
    }

    // ── Switching ──

    pub fn switch(&mut self, provider: &str, model: &str) -> Result<(), ShimError> {
        let pdef = self.providers.iter().find(|p| p.name == provider).ok_or_else(|| {
            ShimError::InvalidRequest(format!("provider '{}' not found", provider))
        })?;

        if !pdef.models.iter().any(|m| m == model) {
            return Err(ShimError::InvalidRequest(format!(
                "model '{}' not in provider '{}' models list: {:?}",
                model, provider, pdef.models
            )));
        }

        let key_hash = keyring_store::get_api_key(provider)
            .map(|k| keyring_store::short_hash(&k))
            .unwrap_or_default();

        if key_hash.is_empty() {
            return Err(ShimError::InvalidRequest(format!(
                "no API key for provider '{}'; import a key first",
                provider
            )));
        }

        self.active = ActiveProfile {
            provider: provider.to_string(),
            model: model.to_string(),
            key_hash,
        };

        if let Err(e) = save_active_profile(&self.profile_path, &self.active) {
            warn!("Failed to persist active_profile.toml: {e}");
        }

        info!(provider = %provider, model = %model, "Switched active profile");
        Ok(())
    }

    // ── Key management ──

    pub fn import_key(&mut self, provider_name: &str, api_key: &str) -> Result<String, ShimError> {
        let hash = keyring_store::short_hash(api_key);
        keyring_store::set_api_key(provider_name, api_key).map_err(|e| {
            ShimError::InvalidRequest(format!("failed to store key in keyring: {e}"))
        })?;

        if let Some(client) = self.clients.get_mut(provider_name) {
            client.set_api_key(api_key.to_string());
        } else if let Some(pdef) = self.providers.iter().find(|p| p.name == provider_name) {
            let mut client =
                ProviderClient::new(&pdef.base_url, 300, &default_user_agent(), false, false)
                    .map_err(|e| ShimError::Transport(e.to_string()))?;
            client.set_api_key(api_key.to_string());
            self.clients.insert(provider_name.to_string(), client);
        }

        if self.active.provider == provider_name {
            self.active.key_hash = hash.clone();
            let _ = save_active_profile(&self.profile_path, &self.active);
        }

        info!(provider = %provider_name, hash = %hash, "API key imported");
        Ok(hash)
    }

    pub fn remove_key(&mut self, provider_name: &str) -> Result<(), ShimError> {
        keyring_store::delete_api_key(provider_name).map_err(|e| {
            ShimError::InvalidRequest(format!("failed to delete key from keyring: {e}"))
        })?;

        if let Some(client) = self.clients.get_mut(provider_name) {
            client.set_api_key(String::new());
        }

        info!(provider = %provider_name, "API key removed");
        Ok(())
    }

    // ── Provider CRUD ──

    pub fn add_provider(&mut self, pdef: ProviderDef) -> Result<(), ShimError> {
        if self.providers.iter().any(|p| p.name == pdef.name) {
            return Err(ShimError::InvalidRequest(format!(
                "provider '{}' already exists",
                pdef.name
            )));
        }

        let client = ProviderClient::new(&pdef.base_url, 300, &default_user_agent(), false, false)
            .map_err(|e| ShimError::Transport(e.to_string()))?;
        self.clients.insert(pdef.name.clone(), client);
        self.providers.push(pdef);

        info!("Provider added");
        Ok(())
    }

    pub fn update_provider(&mut self, pdef: ProviderDef) -> Result<(), ShimError> {
        let idx = self.providers.iter().position(|p| p.name == pdef.name).ok_or_else(|| {
            ShimError::InvalidRequest(format!("provider '{}' not found", pdef.name))
        })?;

        let mut client =
            ProviderClient::new(&pdef.base_url, 300, &default_user_agent(), false, false)
                .map_err(|e| ShimError::Transport(e.to_string()))?;
        if let Ok(key) = keyring_store::get_api_key(&pdef.name) {
            client.set_api_key(key);
        }
        self.clients.insert(pdef.name.clone(), client);
        self.providers[idx] = pdef;

        info!("Provider updated");
        Ok(())
    }

    pub fn remove_provider(&mut self, name: &str) -> Result<(), ShimError> {
        if !self.providers.iter().any(|p| p.name == name) {
            return Err(ShimError::InvalidRequest(format!(
                "provider '{}' not found",
                name
            )));
        }

        if self.active.provider == name {
            return Err(ShimError::InvalidRequest(
                "cannot remove the active provider; switch first".to_string(),
            ));
        }

        self.providers.retain(|p| p.name != name);
        self.clients.remove(name);

        info!(provider = %name, "Provider removed");
        Ok(())
    }

    /// Save current providers to config.toml via snapshot-then-rename.
    pub fn save_config_snapshot(
        &self,
        config_source: Option<&std::path::Path>,
    ) -> Result<String, ShimError> {
        let config_path = config_source
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("config.toml"));

        let snapshot_path = config_path.with_extension("toml.snapshot");

        let content = build_config_toml(&self.providers);
        std::fs::write(&snapshot_path, &content)
            .map_err(|e| ShimError::InvalidRequest(format!("failed to write snapshot: {e}")))?;

        // Validate: parse it back.
        let parsed: std::collections::HashMap<String, toml::Value> = toml::from_str(&content)
            .map_err(|e| {
                let _ = std::fs::remove_file(&snapshot_path);
                ShimError::InvalidRequest(format!("snapshot validation failed: {e}"))
            })?;

        // Check provider entries are parseable.
        if let Some(prov) = parsed.get("provider") {
            if let Some(entries) = prov.get("entries") {
                let _: Vec<ProviderDef> =
                    entries.clone().try_into().map_err(|e: toml::de::Error| {
                        let _ = std::fs::remove_file(&snapshot_path);
                        ShimError::InvalidRequest(format!("provider entries invalid: {e}"))
                    })?;
            }
        }

        // Backup original if it exists.
        if config_path.exists() {
            let backup = config_path.with_extension("toml.bak");
            let _ = std::fs::copy(&config_path, &backup);
        }

        // Replace.
        std::fs::rename(&snapshot_path, &config_path)
            .map_err(|e| ShimError::InvalidRequest(format!("failed to replace config: {e}")))?;

        let checksum = format!(
            "blake3:{}",
            blake3::Hasher::new().update(content.as_bytes()).finalize().to_hex()
        );

        info!(checksum = %checksum, "Config snapshot saved and replaced");
        Ok(checksum)
    }

    // ── Listing ──

    pub fn list_providers(&self) -> Vec<ProviderStatus> {
        self.providers
            .iter()
            .map(|pdef| {
                let has_key = keyring_store::get_api_key(&pdef.name).is_ok();
                let is_active = pdef.name == self.active.provider;
                ProviderStatus {
                    name: pdef.name.clone(),
                    base_url: pdef.base_url.clone(),
                    model: pdef.model.clone(),
                    models: pdef.models.clone(),
                    thinking: pdef.thinking,
                    has_key,
                    is_active,
                    active_model: if is_active {
                        self.active.model.clone()
                    } else {
                        String::new()
                    },
                    default_temperature: pdef.default_temperature,
                    default_top_p: pdef.default_top_p,
                    default_max_output_tokens: pdef.default_max_output_tokens,
                }
            })
            .collect()
    }

    pub fn provider_defs(&self) -> &[ProviderDef] {
        &self.providers
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub models: Vec<String>,
    pub thinking: bool,
    pub has_key: bool,
    pub is_active: bool,
    pub active_model: String,
    pub default_temperature: Option<f64>,
    pub default_top_p: Option<f64>,
    pub default_max_output_tokens: Option<u64>,
}

// ── Persistence ──

fn load_active_profile(path: &std::path::Path) -> anyhow::Result<ActiveProfile> {
    let content = std::fs::read_to_string(path)?;
    let ap: ActiveProfile = toml::from_str(&content)?;
    Ok(ap)
}

fn save_active_profile(path: &std::path::Path, ap: &ActiveProfile) -> anyhow::Result<()> {
    let content = toml::to_string_pretty(ap)?;
    std::fs::write(path, content)?;
    Ok(())
}

fn build_config_toml(providers: &[ProviderDef]) -> String {
    let mut lines = vec![
        "# codex-shim configuration".to_string(),
        "# Auto-generated by provider manager.".to_string(),
        String::new(),
        "[server]".to_string(),
        "host = \"127.0.0.1\"".to_string(),
        "port = 33300".to_string(),
        String::new(),
        "[provider]".to_string(),
        "fallback_unknown_model_to_default = true".to_string(),
        String::new(),
    ];

    for pdef in providers {
        lines.push("  [[provider.entries]]".to_string());
        lines.push(format!("  name = \"{}\"", pdef.name));
        lines.push(format!("  base_url = \"{}\"", pdef.base_url));
        lines.push(format!("  model = \"{}\"", pdef.model));
        // Use proper TOML array syntax
        let models_str: Vec<String> = pdef.models.iter().map(|m| format!("\"{}\"", m)).collect();
        lines.push(format!("  models = [{}]", models_str.join(", ")));
        lines.push(format!("  thinking = {}", pdef.thinking));
        if let Some(temp) = pdef.default_temperature {
            lines.push(format!("  default_temperature = {}", temp));
        }
        if let Some(top_p) = pdef.default_top_p {
            lines.push(format!("  default_top_p = {}", top_p));
        }
        if let Some(tokens) = pdef.default_max_output_tokens {
            lines.push(format!("  default_max_output_tokens = {}", tokens));
        }
        lines.push(String::new());
    }

    lines.push("[provider.aliases]".to_string());
    lines.push(String::new());
    lines.push("[generation]".to_string());
    lines.push(String::new());
    lines.push("[http]".to_string());
    lines.push("request_timeout_secs = 300".to_string());
    lines.push("trust_env = false".to_string());
    lines.push("http2_prior_knowledge = false".to_string());
    lines.push(format!("user_agent = \"{}\"", default_user_agent()));
    lines.push(String::new());
    lines.push("[behavior]".to_string());
    lines.push("access_log = true".to_string());
    lines.push("response_store_max = 1000".to_string());
    lines.push("forward_parallel_tool_calls = true".to_string());
    lines.push(String::new());
    lines.push("[log]".to_string());
    lines.push("level = \"codex_mimo_shim=info,tower_http=warn\"".to_string());
    lines.push(String::new());

    lines.join("\n")
}
