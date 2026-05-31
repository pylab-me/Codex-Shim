use std::env;

fn env_or_unknown(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| "unknown".to_string())
}

fn main() {
    println!(
        "cargo:rustc-env=BUILD_CARGO_LOCK_SHA256={}",
        env_or_unknown("BUILD_CARGO_LOCK_SHA256")
    );
    println!(
        "cargo:rustc-env=BUILD_CI_WORKFLOW_SHA256={}",
        env_or_unknown("BUILD_CI_WORKFLOW_SHA256")
    );
    println!(
        "cargo:rustc-env=BUILD_PUBLIC_BUILD_RS_SHA256={}",
        env_or_unknown("BUILD_PUBLIC_BUILD_RS_SHA256")
    );
    println!(
        "cargo:rustc-env=BUILD_SOURCE_REF={}",
        env_or_unknown("BUILD_SOURCE_REF")
    );
    println!(
        "cargo:rustc-env=BUILD_SOURCE_REPOSITORY={}",
        env_or_unknown("BUILD_SOURCE_REPOSITORY")
    );
    println!(
        "cargo:rustc-env=BUILD_RELEASE_VERSION={}",
        env_or_unknown("BUILD_RELEASE_VERSION")
    );
    println!(
        "cargo:rustc-env=BUILD_RELEASE_TARGET={}",
        env_or_unknown("BUILD_RELEASE_TARGET")
    );

    // Official master seed: injected via CODEX_SHIM_MASTER_SEED in CI.
    // When present, the binary is an official build with deterministic
    // ledger HMAC keys derived per-version from this seed.
    // When absent, the binary is a self-build and uses a random keyring secret.
    let seed_hex = env::var("CODEX_SHIM_MASTER_SEED").unwrap_or_default();
    if seed_hex.is_empty() {
        println!("cargo:rustc-env=CODEX_SHIM_OFFICIAL_SEED=");
    } else {
        // Validate: must be exactly 64 hex chars (32 bytes).
        if seed_hex.len() != 64 || !seed_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            panic!(
                "CODEX_SHIM_MASTER_SEED must be exactly 64 hex characters, got len={}",
                seed_hex.len()
            );
        }
        println!("cargo:rustc-env=CODEX_SHIM_OFFICIAL_SEED={seed_hex}");
    }
}
