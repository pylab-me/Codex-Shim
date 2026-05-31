use std::collections::VecDeque;

use chrono::Utc;
use serde::Serialize;
use serde_json::{Value, json};

/// Maximum number of in-memory TRC records.
const MAX_TRC_RECORDS: usize = 10_000;

/// A single in-memory TRC record (lost on restart).
/// Matches the fields from access_out in main.rs.
#[derive(Debug, Clone, Serialize)]
pub struct TrcRecord {
    pub seq: u64,
    pub trace_id: String,
    pub created_at: String,
    pub client_model: String,
    pub provider_model: String,
    pub model_route: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub provider_ms: u64,
    pub request_time_ms: u64,
    pub status_code: u16,
    pub error: String,
}

/// In-memory ring buffer for TRC records.
pub struct TrcStore {
    records: VecDeque<TrcRecord>,
    next_seq: u64,
}

impl TrcStore {
    pub fn new() -> Self {
        Self {
            records: VecDeque::with_capacity(MAX_TRC_RECORDS),
            next_seq: 1,
        }
    }

    pub fn push(&mut self, record: TrcRecord) {
        if self.records.len() >= MAX_TRC_RECORDS {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    /// Get the most recent `limit` records (newest first).
    pub fn recent(&self, limit: usize) -> Vec<&TrcRecord> {
        let len = self.records.len();
        let skip = len.saturating_sub(limit);
        self.records.iter().skip(skip).rev().collect()
    }

    /// Create and push a new record, returning the assigned seq.
    pub fn record(
        &mut self,
        trace_id: String,
        client_model: String,
        provider_model: String,
        model_route: String,
        prompt_tokens: u64,
        completion_tokens: u64,
        total_tokens: u64,
        cached_tokens: u64,
        reasoning_tokens: u64,
        provider_ms: u64,
        request_time_ms: u64,
        status_code: u16,
        error: String,
        created_at: String,
    ) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.push(TrcRecord {
            seq,
            trace_id,
            created_at,
            client_model,
            provider_model,
            model_route,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            cached_tokens,
            reasoning_tokens,
            provider_ms,
            request_time_ms,
            status_code,
            error,
        });
        seq
    }
}

/// Format the current time as ISO 8601 with millisecond precision.
pub fn now_iso8601() -> String {
    // 1. 直接获取当前 UTC 时间
    let now = Utc::now();

    // 2. 使用 chrono 的内置占位符直接格式化
    // %Y-%m-%dT%H:%M:%S 是年月日时分秒
    // %.3f 代表保留 3 位小数的毫秒（带点，即 .123）
    // Z 代表 UTC 时区
    now.format("%Y-%m-%dT%H:%M:%S.%.3fZ").to_string()
}

/// Serialize a list of TRC records to JSON.
pub fn trc_to_json(records: &[&TrcRecord]) -> Value {
    let arr: Vec<Value> = records
        .iter()
        .map(|r| {
            json!({
                "seq": r.seq,
                "trace_id": r.trace_id,
                "created_at": r.created_at,
                "client_model": r.client_model,
                "provider_model": r.provider_model,
                "model_route": r.model_route,
                "prompt_tokens": r.prompt_tokens,
                "completion_tokens": r.completion_tokens,
                "total_tokens": r.total_tokens,
                "cached_tokens": r.cached_tokens,
                "reasoning_tokens": r.reasoning_tokens,
                "provider_ms": r.provider_ms,
                "request_time_ms": r.request_time_ms,
                "status_code": r.status_code,
                "error": r.error,
            })
        })
        .collect();
    json!({ "records": arr, "total": records.len() })
}
