use std::path::{Path, PathBuf};

/// Reads and writes config.toml for the dashboard editor.
#[derive(Clone)]
pub struct ConfigEditor {
    config_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfigContent {
    pub content: String,
    pub path: String,
    pub checksum: String,
}

impl ConfigEditor {
    pub fn new(config_dir: Option<&Path>) -> Self {
        let dir = config_dir.unwrap_or_else(|| Path::new("."));
        Self {
            config_path: dir.join("config.toml"),
        }
    }

    /// Read the config file contents.
    pub fn read(&self) -> Option<ConfigContent> {
        let content = std::fs::read_to_string(&self.config_path).ok()?;
        let checksum = format!(
            "blake3:{}",
            blake3::Hasher::new().update(content.as_bytes()).finalize().to_hex()
        );
        Some(ConfigContent {
            content,
            path: self
                .config_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            checksum,
        })
    }

    /// Validate TOML and write to disk (with backup).
    pub fn write(&self, content: &str) -> Result<ConfigContent, String> {
        // 1. Validate TOML syntax.
        toml::from_str::<toml::Value>(content).map_err(|e| format!("TOML parse error: {e}"))?;

        // 2. Backup existing file.
        let backup = self.config_path.with_extension("toml.bak");
        if self.config_path.exists() {
            std::fs::copy(&self.config_path, &backup)
                .map_err(|e| format!("Failed to create backup: {e}"))?;
        }

        // 3. Atomic write: write to .tmp then rename.
        let tmp = self.config_path.with_extension("toml.tmp");
        std::fs::write(&tmp, content).map_err(|e| format!("Failed to write temp file: {e}"))?;
        std::fs::rename(&tmp, &self.config_path)
            .map_err(|e| format!("Failed to rename temp file: {e}"))?;

        let checksum = format!(
            "blake3:{}",
            blake3::Hasher::new().update(content.as_bytes()).finalize().to_hex()
        );
        Ok(ConfigContent {
            content: content.to_string(),
            path: self
                .config_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            checksum,
        })
    }
}
