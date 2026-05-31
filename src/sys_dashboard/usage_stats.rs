/// Cumulative statistics computed from token_usage.bin.
#[derive(Debug, Clone, Default)]
pub struct TokenUsageStats {
    pub request_count: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub total_request_time_ms: u64,
    pub total_cached_tokens: u64,
    pub total_reasoning_tokens: u64,
    pub last_request_time_ms: u64,
    pub last_created_at_ms: i64,
    pub last_seq: u64,
    /// "ok" or "corrupted" if ledger integrity check failed partway.
    pub ledger_status: String,
    /// Number of records that failed MAC verification (non-official or tampered).
    pub unverified_count: u64,
}

/// JSON-serializable summary for the API.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenUsageSummary {
    pub request_count: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub total_request_time_ms: u64,
    pub total_cached_tokens: u64,
    pub total_reasoning_tokens: u64,
    pub avg_request_time_ms: f64,
    pub avg_prompt_tokens: f64,
    pub avg_completion_tokens: f64,
    pub last_request_time_ms: u64,
    pub last_updated_at_ms: i64,
    pub ledger_status: String,
    pub unverified_count: u64,
}

impl TokenUsageStats {
    pub fn to_summary(&self) -> TokenUsageSummary {
        let count = self.request_count;
        TokenUsageSummary {
            request_count: count,
            prompt_tokens: self.prompt_tokens,
            completion_tokens: self.completion_tokens,
            total_tokens: self.total_tokens,
            total_request_time_ms: self.total_request_time_ms,
            total_cached_tokens: self.total_cached_tokens,
            total_reasoning_tokens: self.total_reasoning_tokens,
            avg_request_time_ms: if count > 0 {
                self.total_request_time_ms as f64 / count as f64
            } else {
                0.0
            },
            avg_prompt_tokens: if count > 0 {
                self.prompt_tokens as f64 / count as f64
            } else {
                0.0
            },
            avg_completion_tokens: if count > 0 {
                self.completion_tokens as f64 / count as f64
            } else {
                0.0
            },
            last_request_time_ms: self.last_request_time_ms,
            last_updated_at_ms: self.last_created_at_ms,
            ledger_status: self.ledger_status.clone(),
            unverified_count: self.unverified_count,
        }
    }
}
