//! Ledger HMAC key management.
//!
//! Two modes:
//! - **Official build**: `CODEX_SHIM_MASTER_SEED` is set at compile time (injected by CI).
//!   HMAC keys are deterministically derived per-version via `blake3_kdf(seed, version)`.
//! - **Self-build**: No master seed. A random secret is generated and stored in the
//!   OS keyring. All records use this single key.
//!
//! Verification during scan tries:
//! 1. Official key derived from the record's embedded build version.
//! 2. Official key derived from the current build version (for forward compat).
//! 3. Self-build keyring secret (for records written by a self-build).

use tracing::info;

/// Compile-time official master seed. Empty string = self-build.
const OFFICIAL_SEED_HEX: &str = env!("CODEX_SHIM_OFFICIAL_SEED");

/// Context string for blake3 KDF derivation.
const KDF_CONTEXT: &str = "codex-shim-ledger-key-2026";

/// Whether this binary was built with an official master seed.
pub fn is_official_build() -> bool {
    !OFFICIAL_SEED_HEX.is_empty()
}

/// Decode the compile-time hex seed into 32 bytes.
/// Returns None for self-builds.
fn decode_official_seed() -> Option<[u8; 32]> {
    if OFFICIAL_SEED_HEX.is_empty() {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&OFFICIAL_SEED_HEX[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Derive a per-version HMAC key from the master seed.
/// `version_str` should be "major.minor.patch" (e.g. "0.3.2").
fn derive_version_key(seed: &[u8; 32], version_str: &str) -> [u8; 32] {
    let mut ctx = String::from(KDF_CONTEXT);
    ctx.push_str(":");
    ctx.push_str(version_str);
    blake3::derive_key(&ctx, seed)
}

/// Get the HMAC key for signing new records in the current build.
///
/// - Official: derives from master seed + current version.
/// - Self-build: uses the random keyring secret.
pub fn current_signing_key() -> anyhow::Result<[u8; 32]> {
    if let Some(seed) = decode_official_seed() {
        let version = env!("CARGO_PKG_VERSION");
        let key = derive_version_key(&seed, version);
        info!(version = %version, "Using official per-version ledger key");
        Ok(key)
    } else {
        let key = crate::keyring_store::get_or_create_self_build_secret()?;
        info!("Using self-build keyring ledger key");
        Ok(key)
    }
}

/// Verification result for a single ledger record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyStatus {
    /// Verified with official master seed (record's embedded version).
    Official,
    /// Verified with self-build keyring secret.
    SelfBuild,
    /// MAC does not match any known key.
    Unverified,
}

/// Try to verify a record's MAC against all available keys.
///
/// Strategy (in order):
/// 1. Official key derived from the record's build_version field.
/// 2. Self-build keyring secret (if available).
pub fn verify_record_mac(
    payload: &[u8],
    record_mac: &[u8; 32],
    record_build_version: &str,
) -> VerifyStatus {
    // Try official key derived from the record's version.
    if let Some(seed) = decode_official_seed() {
        let key = derive_version_key(&seed, record_build_version);
        let computed = blake3::keyed_hash(&key, payload);
        if computed.as_bytes() == record_mac {
            return VerifyStatus::Official;
        }
    }

    // Try self-build keyring secret.
    if let Ok(self_build_key) = crate::keyring_store::get_self_build_secret() {
        let computed = blake3::keyed_hash(&self_build_key, payload);
        if computed.as_bytes() == record_mac {
            return VerifyStatus::SelfBuild;
        }
    }

    VerifyStatus::Unverified
}

/// Compute the MAC for a new record using the current signing key.
pub fn compute_mac(payload: &[u8]) -> anyhow::Result<[u8; 32]> {
    let key = current_signing_key()?;
    let mac = blake3::keyed_hash(&key, payload);
    Ok(mac.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_version_key_deterministic() {
        let seed = [0xABu8; 32];
        let k1 = derive_version_key(&seed, "0.3.0");
        let k2 = derive_version_key(&seed, "0.3.0");
        assert_eq!(k1, k2, "same seed + version should produce same key");
    }

    #[test]
    fn test_derive_version_key_differs_across_versions() {
        let seed = [0xABu8; 32];
        let k1 = derive_version_key(&seed, "0.3.0");
        let k2 = derive_version_key(&seed, "0.4.0");
        assert_ne!(k1, k2, "different versions should produce different keys");
    }

    #[test]
    fn test_derive_version_key_differs_across_seeds() {
        let seed1 = [0x01u8; 32];
        let seed2 = [0x02u8; 32];
        let k1 = derive_version_key(&seed1, "0.3.0");
        let k2 = derive_version_key(&seed2, "0.3.0");
        assert_ne!(k1, k2, "different seeds should produce different keys");
    }
}
