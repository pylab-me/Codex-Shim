use std::collections::HashMap;

use serde::Serialize;

/// Per-model aggregated runtime statistics.
/// On startup, populated from `token_usage.bin`; at runtime, updated incrementally.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ModelStatsEntry {
    pub provider_model: String,
    pub model_route: String,
    pub request_count: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_provider_ms: u64,
    pub total_total_ms: u64,
    pub last_provider_ms: u64,
    pub last_total_ms: u64,
    pub last_status: u16,
    pub last_error: String,
}

/// Aggregated summary across all models.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ModelStatsSummary {
    pub model_count: usize,
    pub total_requests: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_tokens: u64,
    pub total_cached_tokens: u64,
    pub total_reasoning_tokens: u64,
    pub total_provider_ms: u64,
    pub total_total_ms: u64,
}

/// In-memory tracker for per-model stats.
pub struct ModelStatsStore {
    models: HashMap<String, ModelStatsEntry>,
}

impl ModelStatsStore {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Seed from the bin file's per-model aggregation (called once at startup).
    pub fn seed_from_bin(&mut self, entries: &[crate::sys_dashboard::usage_store::BinModelEntry]) {
        for e in entries {
            self.models.insert(
                e.provider_model.clone(),
                ModelStatsEntry {
                    provider_model: e.provider_model.clone(),
                    model_route: e.model_route.clone(),
                    request_count: e.request_count,
                    prompt_tokens: e.prompt_tokens,
                    completion_tokens: e.completion_tokens,
                    total_tokens: e.total_tokens,
                    cached_tokens: e.cached_tokens,
                    reasoning_tokens: e.reasoning_tokens,
                    total_provider_ms: e.total_request_time_ms,
                    ..Default::default()
                },
            );
        }
    }

    /// Record a request result, updating per-model stats.
    pub fn record(
        &mut self,
        provider_model: &str,
        model_route: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
        total_tokens: u64,
        cached_tokens: u64,
        reasoning_tokens: u64,
        provider_ms: u64,
        total_ms: u64,
        status_code: u16,
        error: &str,
    ) {
        let key = provider_model.to_string();
        let entry = self.models.entry(key).or_insert_with(|| ModelStatsEntry {
            provider_model: provider_model.to_string(),
            model_route: model_route.to_string(),
            ..Default::default()
        });

        // Update route if it changed (shouldn't normally, but be safe).
        if !model_route.is_empty() {
            entry.model_route = model_route.to_string();
        }

        entry.request_count += 1;
        if status_code == 200 {
            entry.success_count += 1;
        } else {
            entry.error_count += 1;
        }
        entry.prompt_tokens += prompt_tokens;
        entry.completion_tokens += completion_tokens;
        entry.total_tokens += total_tokens;
        entry.cached_tokens += cached_tokens;
        entry.reasoning_tokens += reasoning_tokens;
        entry.total_provider_ms += provider_ms;
        entry.total_total_ms += total_ms;
        entry.last_provider_ms = provider_ms;
        entry.last_total_ms = total_ms;
        entry.last_status = status_code;
        entry.last_error = error.to_string();
    }

    /// Get per-model stats as a Vec (sorted by request count descending).
    pub fn per_model(&self) -> Vec<ModelStatsEntry> {
        let mut entries: Vec<ModelStatsEntry> = self.models.values().cloned().collect();
        entries.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        entries
    }

    /// Get aggregate summary across all models.
    pub fn summary(&self) -> ModelStatsSummary {
        let mut sum = ModelStatsSummary::default();
        sum.model_count = self.models.len();
        for entry in self.models.values() {
            sum.total_requests += entry.request_count;
            sum.total_prompt_tokens += entry.prompt_tokens;
            sum.total_completion_tokens += entry.completion_tokens;
            sum.total_tokens += entry.total_tokens;
            sum.total_cached_tokens += entry.cached_tokens;
            sum.total_reasoning_tokens += entry.reasoning_tokens;
            sum.total_provider_ms += entry.total_provider_ms;
            sum.total_total_ms += entry.total_total_ms;
        }
        sum
    }
}
