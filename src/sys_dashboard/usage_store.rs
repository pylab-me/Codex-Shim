use std::collections::HashMap;
use std::fs::{
    File, OpenOptions, {self},
};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use tracing::{info, warn};

use super::ledger_key::{VerifyStatus, verify_record_mac};
use super::usage_record::{RECORD_SIZE, TokenUsageInput, TokenUsageRecord};
use super::usage_stats::TokenUsageStats;

/// Per-model aggregated stats recovered from the bin file.
/// Uses the same struct as `ModelStatsEntry` conceptually but kept here
/// to avoid coupling to the in-memory store.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct BinModelEntry {
    pub provider_model: String,
    pub model_route: String,
    pub request_count: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_request_time_ms: u64,
}

/// Persistent token usage store backed by a flat binary file.
///
/// HMAC keys are managed by the `ledger_key` module:
/// - Official builds use a master seed with per-version key derivation.
/// - Self-builds use a random secret stored in the OS keyring.
pub struct TokenUsageStore {
    path: PathBuf,
    stats: TokenUsageStats,
    /// Per-model aggregations recovered from bin on startup.
    per_model: HashMap<String, BinModelEntry>,
}

impl TokenUsageStore {
    /// Open or create the store under `root` (typically platform data dir).
    ///
    /// Files created:
    /// - `root/token_usage.bin` (the ledger)
    pub fn open(root: impl AsRef<Path>) -> anyhow::Result<Self> {
        let root = root.as_ref();
        fs::create_dir_all(root)?;

        let bin_path = root.join("token_usage.bin");
        let mut stats = TokenUsageStats::default();
        let mut per_model = HashMap::new();

        if bin_path.exists() {
            let (s, pm) = scan_and_verify(&bin_path)?;
            stats = s;
            per_model = pm;
        }

        info!(
            records = stats.request_count,
            unverified = stats.unverified_count,
            models = per_model.len(),
            ledger_status = %stats.ledger_status,
            official = super::ledger_key::is_official_build(),
            "TokenUsageStore opened"
        );

        Ok(Self {
            path: bin_path,
            stats,
            per_model,
        })
    }

    /// Record a completed request. Appends to the bin file and updates in-memory stats.
    pub fn record_usage(&mut self, input: TokenUsageInput) -> anyhow::Result<()> {
        let next_seq = self.stats.last_seq + 1;

        let mut record = TokenUsageRecord::new(next_seq, &input);

        // Enforce monotonic timestamps.
        let now_ms = record.created_at_ms;
        record.created_at_ms = now_ms.max(self.stats.last_created_at_ms + 1);

        // Compute MAC using the current signing key (official or self-build).
        let buf = record.encode();
        let mac = super::ledger_key::compute_mac(TokenUsageRecord::mac_payload(&buf))?;
        record.record_mac = mac;

        // Append to file.
        let encoded = record.encode();
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        file.write_all(&encoded)?;
        file.sync_all()?;

        // Update in-memory stats.
        self.stats.request_count += 1;
        self.stats.prompt_tokens += input.prompt_tokens;
        self.stats.completion_tokens += input.completion_tokens;
        self.stats.total_tokens += input.total_tokens;
        self.stats.total_request_time_ms += input.request_time_ms;
        self.stats.total_cached_tokens += input.cached_tokens;
        self.stats.total_reasoning_tokens += input.reasoning_tokens;
        self.stats.last_request_time_ms = input.request_time_ms;
        self.stats.last_created_at_ms = record.created_at_ms;
        self.stats.last_seq = next_seq;

        // Update per-model aggregation.
        let key = input.provider_model.clone();
        let entry = self.per_model.entry(key).or_insert_with(|| BinModelEntry {
            provider_model: input.provider_model.clone(),
            model_route: input.model_route.clone(),
            ..Default::default()
        });
        if !input.model_route.is_empty() {
            entry.model_route = input.model_route.clone();
        }
        entry.request_count += 1;
        entry.prompt_tokens += input.prompt_tokens;
        entry.completion_tokens += input.completion_tokens;
        entry.total_tokens += input.total_tokens;
        entry.cached_tokens += input.cached_tokens;
        entry.reasoning_tokens += input.reasoning_tokens;
        entry.total_request_time_ms += input.request_time_ms;

        Ok(())
    }

    /// Get a snapshot of cumulative statistics.
    pub fn summary(&self) -> super::usage_stats::TokenUsageSummary {
        self.stats.to_summary()
    }

    /// Get per-model aggregations from the bin file.
    pub fn per_model(&self) -> Vec<BinModelEntry> {
        let mut entries: Vec<BinModelEntry> = self.per_model.values().cloned().collect();
        entries.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        entries
    }

    /// Get the most recent `limit` records.
    pub fn recent(&self, limit: usize) -> anyhow::Result<Vec<TokenUsageRecord>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let data = fs::read(&self.path)?;
        let total_records = data.len() / RECORD_SIZE;
        let skip = total_records.saturating_sub(limit);

        let mut records = Vec::with_capacity(total_records - skip);
        for i in skip..total_records {
            let offset = i * RECORD_SIZE;
            let mut buf = [0u8; RECORD_SIZE];
            buf.copy_from_slice(&data[offset..offset + RECORD_SIZE]);
            if let Some(record) = TokenUsageRecord::decode(&buf) {
                records.push(record);
            }
        }
        Ok(records)
    }
}

/// Scan the bin file, verify MACs, and accumulate stats + per-model data.
/// Uses multi-key verification: tries official per-version keys, then
/// self-build keyring key.
fn scan_and_verify(
    path: &Path,
) -> anyhow::Result<(TokenUsageStats, HashMap<String, BinModelEntry>)> {
    let mut stats = TokenUsageStats::default();
    stats.ledger_status = "ok".to_string();
    let mut per_model: HashMap<String, BinModelEntry> = HashMap::new();

    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len() as usize;
    let total_records = file_len / RECORD_SIZE;

    if total_records == 0 {
        return Ok((stats, per_model));
    }

    let mut buf = [0u8; RECORD_SIZE];
    let mut expected_seq: u64 = 1;

    for i in 0..total_records {
        file.seek(SeekFrom::Start((i * RECORD_SIZE) as u64))?;
        file.read_exact(&mut buf)?;

        let record = match TokenUsageRecord::decode(&buf) {
            Some(r) => r,
            None => {
                warn!(
                    record_index = i,
                    "Ledger corrupted at record; stopping scan"
                );
                stats.ledger_status = "corrupted".to_string();
                break;
            }
        };

        // Verify sequential ordering.
        if record.seq != expected_seq {
            warn!(
                record_index = i,
                expected = expected_seq,
                actual = record.seq,
                "Sequence gap; stopping scan"
            );
            stats.ledger_status = "corrupted".to_string();
            break;
        }

        expected_seq = record.seq + 1;

        // Multi-key MAC verification.
        let payload = TokenUsageRecord::mac_payload(&buf);
        let verify_status =
            verify_record_mac(payload, &record.record_mac, &record.build_version_str());

        match verify_status {
            VerifyStatus::Official | VerifyStatus::SelfBuild => {
                // Accumulate verified records.
                stats.request_count += 1;
                stats.prompt_tokens += record.prompt_tokens;
                stats.completion_tokens += record.completion_tokens;
                stats.total_tokens += record.total_tokens;
                stats.total_request_time_ms += record.request_time_ms;
                stats.total_cached_tokens += record.cached_tokens;
                stats.total_reasoning_tokens += record.reasoning_tokens;
                stats.last_request_time_ms = record.request_time_ms;
                stats.last_created_at_ms = record.created_at_ms;
                stats.last_seq = record.seq;

                // Accumulate per-model stats.
                if !record.provider_model.is_empty() {
                    let key = record.provider_model.clone();
                    let entry = per_model.entry(key).or_insert_with(|| BinModelEntry {
                        provider_model: record.provider_model.clone(),
                        model_route: record.model_route.clone(),
                        ..Default::default()
                    });
                    if !record.model_route.is_empty() {
                        entry.model_route = record.model_route.clone();
                    }
                    entry.request_count += 1;
                    entry.prompt_tokens += record.prompt_tokens;
                    entry.completion_tokens += record.completion_tokens;
                    entry.total_tokens += record.total_tokens;
                    entry.cached_tokens += record.cached_tokens;
                    entry.reasoning_tokens += record.reasoning_tokens;
                    entry.total_request_time_ms += record.request_time_ms;
                }
            }
            VerifyStatus::Unverified => {
                stats.unverified_count += 1;
                stats.last_seq = record.seq;
                stats.last_created_at_ms = stats.last_created_at_ms.max(record.created_at_ms);
                warn!(
                    record_index = i,
                    seq = record.seq,
                    build_version = %record.build_version_str(),
                    "Record MAC unverified; skipping from aggregation"
                );
            }
        }
    }

    // Truncate file if there's a partial record at the end.
    let valid_len = total_records * RECORD_SIZE;
    if valid_len < file_len {
        warn!(
            file_len,
            valid_len, "Truncating partial trailing record from ledger"
        );
        drop(file);
        let f = OpenOptions::new().write(true).open(path)?;
        f.set_len(valid_len as u64)?;
        f.sync_all()?;
    }

    Ok((stats, per_model))
}
