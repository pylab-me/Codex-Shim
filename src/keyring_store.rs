use std::sync::OnceLock;

use anyhow::anyhow;
use keyring_core::Entry;
use serde::Serialize;
use tracing::{debug, warn};
/// Keyring service name.
const SERVICE: &str = "codex-shim";
static KEYRING_INIT: OnceLock<Result<(), String>> = OnceLock::new();

/// Build the keyring entry name: `CODEX-SHIM-$provider`.
fn entry_name(provider: &str) -> String {
    format!("CODEX-SHIM-{}", provider)
}

fn ensure_keyring_store() -> anyhow::Result<()> {
    let result =
        KEYRING_INIT.get_or_init(|| keyring::use_native_store(false).map_err(|e| e.to_string()));
    result
        .clone()
        .map_err(|error| anyhow!("failed to initialize keyring store: {error}"))
}

fn new_entry(name: &str) -> anyhow::Result<Entry> {
    ensure_keyring_store()?;
    Entry::new(SERVICE, name).map_err(Into::into)
}

#[derive(Debug, Clone, Serialize)]
pub struct KeyringDiagnostic {
    pub provider: String,
    pub platform: String,
    pub current_user: String,
    pub service: String,
    pub entry_name: String,
    pub backend_target_name: String,
    pub has_entry: bool,
    pub key_hash: Option<String>,
    pub read_error: Option<String>,
}

pub fn diagnose_api_key(provider: &str) -> KeyringDiagnostic {
    let name = entry_name(provider);
    let current_user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string());
    let backend_target_name = if cfg!(target_os = "windows") {
        format!("{name}.{SERVICE}")
    } else {
        name.clone()
    };

    match get_api_key(provider) {
        Ok(key) => KeyringDiagnostic {
            provider: provider.to_string(),
            platform: std::env::consts::OS.to_string(),
            current_user,
            service: SERVICE.to_string(),
            entry_name: name,
            backend_target_name,
            has_entry: true,
            key_hash: Some(short_hash(&key)),
            read_error: None,
        },
        Err(error) => KeyringDiagnostic {
            provider: provider.to_string(),
            platform: std::env::consts::OS.to_string(),
            current_user,
            service: SERVICE.to_string(),
            entry_name: name,
            backend_target_name,
            has_entry: false,
            key_hash: None,
            read_error: Some(error.to_string()),
        },
    }
}

/// Store an API key in the OS keyring.
/// Uses write-then-readback verification to catch silent write failures.
/// Retries read-back up to 3 times with a short delay because Windows
/// Credential Manager may have a brief propagation delay.
/// Returns an error if the value still cannot be read back.
pub fn set_api_key(provider: &str, api_key: &str) -> anyhow::Result<()> {
    let name = entry_name(provider);
    let entry = new_entry(&name)?;
    entry.set_password(api_key)?;

    // Verify: create a fresh Entry and read back to confirm persistence.
    // Retry up to 3 times with a short delay to handle Windows Credential
    // Manager propagation lag.
    let verify_entry = new_entry(&name)?;
    let mut verified = false;
    let mut last_error = None;
    for attempt in 0..3 {
        match verify_entry.get_password() {
            Ok(readback) => {
                if readback == api_key {
                    verified = true;
                    break;
                }
                // Value mismatch is always a hard error.
                let _ = entry.delete_credential();
                return Err(anyhow!(
                    "keyring write verification failed: read back value does not match"
                ));
            }
            Err(e) => {
                if attempt < 2 {
                    debug!(
                        provider = %provider,
                        entry = %name,
                        attempt = attempt + 1,
                        error = %e,
                        "Keyring read-back not yet visible, retrying..."
                    );
                    std::thread::sleep(std::time::Duration::from_millis(200));
                } else {
                    last_error = Some(e);
                }
            }
        }
    }

    if !verified {
        if let Some(error) = last_error {
            let _ = entry.delete_credential();

            warn!(
                provider = %provider,
                entry = %name,
                error = %error,
                "Keyring write rejected because read-back verification failed"
            );

            return Err(anyhow!(
                "keyring write verification failed after 3 read-back attempts: {error}"
            ));
        } else {
            return Err(anyhow!(
                "keyring write verification failed without a readable error"
            ));
        }
    }

    debug!(
        provider = %provider,
        entry = %name,
        "API key stored in keyring (verified)"
    );

    Ok(())
}

/// Retrieve an API key from the OS keyring.
pub fn get_api_key(provider: &str) -> anyhow::Result<String> {
    let name = entry_name(provider);
    let entry = new_entry(&name)?;
    let key = entry.get_password()?;
    Ok(key)
}

/// Delete an API key from the OS keyring.
pub fn delete_api_key(provider: &str) -> anyhow::Result<()> {
    let name = entry_name(provider);
    let entry = new_entry(&name)?;
    entry.delete_credential()?;
    debug!(provider = %provider, entry = %name, "API key deleted from keyring");
    Ok(())
}

/// Check if an API key exists for the given provider.
pub fn has_api_key(provider: &str) -> bool {
    let name = entry_name(provider);
    match new_entry(&name) {
        Ok(entry) => entry.get_password().is_ok(),
        Err(_) => false,
    }
}

/// Compute a short human-readable hash of an API key for display.
/// Returns first 8 hex chars of blake3(app_key).
pub fn short_hash(api_key: &str) -> String {
    let hash = blake3::hash(api_key.as_bytes());
    hash.to_hex()[..8].to_string()
}

// ── Self-build ledger secret (stored in keyring) ──
// Only used when CODEX_SHIM_MASTER_SEED is not set (self-build).

/// Store a self-build ledger secret in the OS keyring.
pub fn set_self_build_secret(secret: &[u8; 32]) -> anyhow::Result<()> {
    let name = "CODEX-SHIM-self-build-secret";
    let entry = new_entry(name)?;
    let hex = hex_encode(secret);
    entry.set_password(&hex)?;
    Ok(())
}

/// Retrieve the self-build ledger secret from the OS keyring.
pub fn get_self_build_secret() -> anyhow::Result<[u8; 32]> {
    let name = "CODEX-SHIM-self-build-secret";
    let entry = new_entry(name)?;
    let hex = entry.get_password()?;
    hex_decode(&hex)
}

/// Generate a new random self-build secret and store it in the keyring.
pub fn generate_self_build_secret() -> anyhow::Result<[u8; 32]> {
    let mut secret = [0u8; 32];
    let mut rng = rand::rng();
    rand::RngExt::fill(&mut rng, &mut secret);
    set_self_build_secret(&secret)?;
    Ok(secret)
}

/// Get or create the self-build ledger secret.
pub fn get_or_create_self_build_secret() -> anyhow::Result<[u8; 32]> {
    match get_self_build_secret() {
        Ok(secret) => Ok(secret),
        Err(_) => generate_self_build_secret(),
    }
}

fn hex_encode(data: &[u8; 32]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(hex: &str) -> anyhow::Result<[u8; 32]> {
    if hex.len() != 64 {
        return Err(anyhow!(
            "invalid hex length: expected 64, got {}",
            hex.len()
        ));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique test prefix to avoid collisions with real credentials.
    const TEST_PREFIX: &str = "__test_keyring_roundtrip__";

    fn test_provider(label: &str) -> String {
        format!("{}{}", TEST_PREFIX, label)
    }

    fn cleanup(provider: &str) {
        let name = entry_name(provider);
        if let Ok(entry) = new_entry(&name) {
            let _ = entry.delete_credential();
        }
    }

    /// Test: set_api_key → get_api_key → values match → delete → get fails.
    #[test]
    fn test_keyring_set_get_delete_roundtrip() {
        let provider = test_provider("roundtrip");
        let api_key = "sk-test-key-abc123-roundtrip";

        cleanup(&provider);

        set_api_key(&provider, api_key).expect(
            "set_api_key should succeed (Windows Credential Manager works for normal users)",
        );

        let retrieved = get_api_key(&provider).expect("get_api_key should succeed after set");
        assert_eq!(retrieved, api_key, "retrieved key must match stored key");

        delete_api_key(&provider).expect("delete_api_key should succeed");

        let result = get_api_key(&provider);
        assert!(result.is_err(), "get after delete should fail");
    }

    /// Test: set_api_key write-then-readback verification catches silent failures.
    #[test]
    fn test_set_api_key_verifies_readback() {
        let provider = test_provider("verify");
        let api_key = "sk-verify-key-xyz789";

        cleanup(&provider);

        set_api_key(&provider, api_key).expect("set_api_key with verification should succeed");

        // Confirm with a separate Entry object.
        let name = entry_name(&provider);
        let entry2 = new_entry(&name).expect("create entry for readback");
        let retrieved = entry2.get_password().expect("key should exist after verified set");
        assert_eq!(retrieved, api_key);

        cleanup(&provider);
    }

    /// Test: overwriting an existing key works.
    #[test]
    fn test_keyring_overwrite() {
        let provider = test_provider("overwrite");
        cleanup(&provider);

        set_api_key(&provider, "old-key").expect("first set should succeed");
        set_api_key(&provider, "new-key").expect("overwrite should succeed");

        let retrieved = get_api_key(&provider).expect("get after overwrite should succeed");
        assert_eq!(retrieved, "new-key", "should have the new value");

        cleanup(&provider);
    }

    /// Test: delete on non-existent key returns error.
    #[test]
    fn test_delete_nonexistent_key_fails() {
        let provider = test_provider("nonexistent_delete");
        cleanup(&provider);

        let result = delete_api_key(&provider);
        assert!(result.is_err(), "deleting non-existent key should fail");
    }

    /// Test: get on non-existent key returns error.
    #[test]
    fn test_get_nonexistent_key_fails() {
        let provider = test_provider("nonexistent_get");
        cleanup(&provider);

        let result = get_api_key(&provider);
        assert!(result.is_err(), "getting non-existent key should fail");
    }

    /// Test: has_api_key returns correct state.
    #[test]
    fn test_has_api_key() {
        let provider = test_provider("has_key");
        cleanup(&provider);

        assert!(!has_api_key(&provider), "should not have key before set");

        set_api_key(&provider, "test-key").expect("set should succeed");
        assert!(has_api_key(&provider), "should have key after set");

        delete_api_key(&provider).expect("delete should succeed");
        assert!(!has_api_key(&provider), "should not have key after delete");
    }

    /// Test: short_hash is deterministic and produces 8-char hex.
    #[test]
    fn test_short_hash_deterministic() {
        let h1 = short_hash("sk-test-deterministic");
        let h2 = short_hash("sk-test-deterministic");
        assert_eq!(h1, h2, "short_hash should be deterministic");
        assert_eq!(h1.len(), 8, "short_hash should be 8 hex chars");

        let h3 = short_hash("sk-other-key");
        assert_ne!(h1, h3, "different keys should produce different hashes");
    }

    /// Test: self-build secret roundtrip.
    #[test]
    fn test_self_build_secret_roundtrip() {
        let name = "CODEX-SHIM-self-build-secret";
        if let Ok(entry) = new_entry(name) {
            let _ = entry.delete_credential();
        }

        let mut secret = [0u8; 32];
        for i in 0..32 {
            secret[i] = i as u8;
        }

        set_self_build_secret(&secret).expect("set_self_build_secret should succeed");
        let retrieved = get_self_build_secret().expect("get_self_build_secret should succeed");
        assert_eq!(
            retrieved, secret,
            "self-build secret roundtrip should match"
        );

        // Cleanup.
        if let Ok(entry) = new_entry(name) {
            let _ = entry.delete_credential();
        }
    }
}
