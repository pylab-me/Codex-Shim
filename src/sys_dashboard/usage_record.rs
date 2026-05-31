use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{TimeZone, Utc};

/// Magic bytes: "TOK3" in little-endian.
pub const MAGIC: u32 = 0x544F4B33;
/// Version 3: adds build_version (major/minor/patch) in the former reserved field.
pub const VERSION: u16 = 3;
/// Fixed record size in bytes.
pub const RECORD_SIZE: usize = 256;

// ── Field offsets (all little-endian) ──
// [0..4)    magic: u32
// [4..6)    version: u16
// [6..8)    record_len: u16
// [8..16)   seq: u64
// [16..24)  created_at_ms: i64
// [24..32)  prompt_tokens: u64
// [32..40)  completion_tokens: u64
// [40..48)  total_tokens: u64
// [48..56)  request_time_ms: u64
// [56..64)  cached_tokens: u64
// [64..72)  reasoning_tokens: u64
// [72..74)  status_code: u16
// [74..76)  build_major: u16
// [76..78)  build_minor: u16
// [78..80)  build_patch: u16
// [80..144) provider_model: [u8; 64] (null-padded UTF-8)
// [144..192) model_route: [u8; 48] (null-padded UTF-8)
// [192..224) record_mac: [u8; 32]
// [224..256) padding: [u8; 32]

/// Size of the data portion before the MAC (192 bytes).
const PAYLOAD_SIZE: usize = 192;

const PROVIDER_MODEL_LEN: usize = 64;
const MODEL_ROUTE_LEN: usize = 48;

/// A single token usage record, stored as a fixed-length binary blob.
#[derive(Debug, Clone)]
pub struct TokenUsageRecord {
    pub magic: u32,
    pub version: u16,
    pub record_len: u16,
    pub seq: u64,
    pub created_at_ms: i64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub request_time_ms: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub status_code: u16,
    /// Build version that wrote this record.
    pub build_major: u16,
    pub build_minor: u16,
    pub build_patch: u16,
    pub provider_model: String,
    pub model_route: String,
    pub record_mac: [u8; 32],
}

/// Minimal view for API responses.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenUsageView {
    pub seq: u64,
    pub created_at: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub request_time_ms: u64,
    pub provider_model: String,
    pub model_route: String,
    pub status_code: u16,
}

/// Input for creating a new record.
pub struct TokenUsageInput {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub request_time_ms: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub model: String,
    pub provider_model: String,
    pub model_route: String,
    pub success: bool,
}

/// Parse the CARGO_PKG_VERSION ("x.y.z") into (major, minor, patch).
fn parse_pkg_version() -> (u16, u16, u16) {
    let v = env!("CARGO_PKG_VERSION");
    let parts: Vec<&str> = v.split('.').collect();
    let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

impl TokenUsageRecord {
    /// Create a new record (without MAC). Caller must compute and set `record_mac`.
    pub fn new(seq: u64, input: &TokenUsageInput) -> Self {
        let now_ms =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
        let (build_major, build_minor, build_patch) = parse_pkg_version();

        Self {
            magic: MAGIC,
            version: VERSION,
            record_len: RECORD_SIZE as u16,
            seq,
            created_at_ms: now_ms,
            prompt_tokens: input.prompt_tokens,
            completion_tokens: input.completion_tokens,
            total_tokens: input.total_tokens,
            request_time_ms: input.request_time_ms,
            cached_tokens: input.cached_tokens,
            reasoning_tokens: input.reasoning_tokens,
            status_code: if input.success { 0 } else { 1 },
            build_major,
            build_minor,
            build_patch,
            provider_model: input.provider_model.clone(),
            model_route: input.model_route.clone(),
            record_mac: [0u8; 32],
        }
    }

    /// Encode the record to a fixed-size byte buffer.
    pub fn encode(&self) -> [u8; RECORD_SIZE] {
        let mut buf = [0u8; RECORD_SIZE];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4..6].copy_from_slice(&self.version.to_le_bytes());
        buf[6..8].copy_from_slice(&self.record_len.to_le_bytes());
        buf[8..16].copy_from_slice(&self.seq.to_le_bytes());
        buf[16..24].copy_from_slice(&self.created_at_ms.to_le_bytes());
        buf[24..32].copy_from_slice(&self.prompt_tokens.to_le_bytes());
        buf[32..40].copy_from_slice(&self.completion_tokens.to_le_bytes());
        buf[40..48].copy_from_slice(&self.total_tokens.to_le_bytes());
        buf[48..56].copy_from_slice(&self.request_time_ms.to_le_bytes());
        buf[56..64].copy_from_slice(&self.cached_tokens.to_le_bytes());
        buf[64..72].copy_from_slice(&self.reasoning_tokens.to_le_bytes());
        buf[72..74].copy_from_slice(&self.status_code.to_le_bytes());
        buf[74..76].copy_from_slice(&self.build_major.to_le_bytes());
        buf[76..78].copy_from_slice(&self.build_minor.to_le_bytes());
        buf[78..80].copy_from_slice(&self.build_patch.to_le_bytes());
        write_padded_str(&mut buf[80..80 + PROVIDER_MODEL_LEN], &self.provider_model);
        write_padded_str(&mut buf[144..144 + MODEL_ROUTE_LEN], &self.model_route);
        buf[192..224].copy_from_slice(&self.record_mac);
        buf
    }

    /// Compute the MAC payload: bytes [0..PAYLOAD_SIZE) of the encoded buffer.
    pub fn mac_payload(buf: &[u8; RECORD_SIZE]) -> &[u8] {
        &buf[..PAYLOAD_SIZE]
    }

    /// Decode a record from a byte buffer. Accepts V2 and V3 formats.
    pub fn decode(buf: &[u8; RECORD_SIZE]) -> Option<Self> {
        let magic = u32::from_le_bytes(buf[0..4].try_into().ok()?);
        if magic != MAGIC {
            // Also accept V2 magic "TOK2"
            if magic != 0x544F4B32 {
                return None;
            }
        }
        let version = u16::from_le_bytes(buf[4..6].try_into().ok()?);
        let record_len = u16::from_le_bytes(buf[6..8].try_into().ok()?);
        if record_len as usize != RECORD_SIZE {
            return None;
        }

        let (build_major, build_minor, build_patch) = if version >= 3 {
            (
                u16::from_le_bytes(buf[74..76].try_into().ok()?),
                u16::from_le_bytes(buf[76..78].try_into().ok()?),
                u16::from_le_bytes(buf[78..80].try_into().ok()?),
            )
        } else {
            // V2 records: reserved field is all zeros, treat as version 0.0.0
            (0, 0, 0)
        };

        Some(Self {
            magic,
            version,
            record_len,
            seq: u64::from_le_bytes(buf[8..16].try_into().ok()?),
            created_at_ms: i64::from_le_bytes(buf[16..24].try_into().ok()?),
            prompt_tokens: u64::from_le_bytes(buf[24..32].try_into().ok()?),
            completion_tokens: u64::from_le_bytes(buf[32..40].try_into().ok()?),
            total_tokens: u64::from_le_bytes(buf[40..48].try_into().ok()?),
            request_time_ms: u64::from_le_bytes(buf[48..56].try_into().ok()?),
            cached_tokens: u64::from_le_bytes(buf[56..64].try_into().ok()?),
            reasoning_tokens: u64::from_le_bytes(buf[64..72].try_into().ok()?),
            status_code: u16::from_le_bytes(buf[72..74].try_into().ok()?),
            build_major,
            build_minor,
            build_patch,
            provider_model: read_padded_str(&buf[80..80 + PROVIDER_MODEL_LEN]),
            model_route: read_padded_str(&buf[144..144 + MODEL_ROUTE_LEN]),
            record_mac: buf[192..224].try_into().ok()?,
        })
    }

    /// Convert to a view for API responses.
    pub fn to_view(&self) -> TokenUsageView {
        TokenUsageView {
            seq: self.seq,
            created_at: format_timestamp_ms(self.created_at_ms),
            prompt_tokens: self.prompt_tokens,
            completion_tokens: self.completion_tokens,
            total_tokens: self.total_tokens,
            cached_tokens: self.cached_tokens,
            reasoning_tokens: self.reasoning_tokens,
            request_time_ms: self.request_time_ms,
            provider_model: self.provider_model.clone(),
            model_route: self.model_route.clone(),
            status_code: self.status_code,
        }
    }

    /// Format the build version as "major.minor.patch".
    pub fn build_version_str(&self) -> String {
        format!(
            "{}.{}.{}",
            self.build_major, self.build_minor, self.build_patch
        )
    }
}

/// Write a string into a fixed-size byte slice, null-padded.
fn write_padded_str(dst: &mut [u8], s: &str) {
    let bytes = s.as_bytes();
    let copy_len = bytes.len().min(dst.len());
    dst[..copy_len].copy_from_slice(&bytes[..copy_len]);
    for b in dst.iter_mut().skip(copy_len) {
        *b = 0;
    }
}

/// Read a null-terminated string from a fixed-size byte slice.
fn read_padded_str(src: &[u8]) -> String {
    let end = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    String::from_utf8_lossy(&src[..end]).to_string()
}

/// Format a millisecond timestamp to ISO 8601 string.
fn format_timestamp_ms(ms: i64) -> String {
    let secs = ms / 1000;
    let nsecs = ((ms % 1000).abs() as u32) * 1_000_000;

    match Utc.timestamp_opt(secs, nsecs) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%dT%H:%M:%S.%.3fZ").to_string(),
        _ => {
            let safe_millis = (ms % 1000).abs();
            format!("{}.{:03}Z", secs, safe_millis)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_encode_decode_roundtrip() {
        let input = TokenUsageInput {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            request_time_ms: 1234,
            cached_tokens: 80,
            reasoning_tokens: 10,
            model: "mimo-v2.5".to_string(),
            provider_model: "mimo-v2.5".to_string(),
            model_route: "direct_provider_model".to_string(),
            success: true,
        };
        let mut record = TokenUsageRecord::new(42, &input);
        let buf = record.encode();
        let mac = blake3::keyed_hash(&[0u8; 32], TokenUsageRecord::mac_payload(&buf));
        record.record_mac = mac.into();
        let buf = record.encode();

        let decoded = TokenUsageRecord::decode(&buf).unwrap();
        assert_eq!(decoded.seq, 42);
        assert_eq!(decoded.prompt_tokens, 100);
        assert_eq!(decoded.completion_tokens, 50);
        assert_eq!(decoded.total_tokens, 150);
        assert_eq!(decoded.cached_tokens, 80);
        assert_eq!(decoded.reasoning_tokens, 10);
        assert_eq!(decoded.request_time_ms, 1234);
        assert_eq!(decoded.status_code, 0);
        assert_eq!(decoded.provider_model, "mimo-v2.5");
        assert_eq!(decoded.model_route, "direct_provider_model");
        // Build version should be the current package version.
        let (expected_major, expected_minor, expected_patch) = parse_pkg_version();
        assert_eq!(decoded.build_major, expected_major);
        assert_eq!(decoded.build_minor, expected_minor);
        assert_eq!(decoded.build_patch, expected_patch);
    }

    #[test]
    fn long_strings_are_truncated() {
        let input = TokenUsageInput {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            request_time_ms: 0,
            cached_tokens: 0,
            reasoning_tokens: 0,
            model: String::new(),
            provider_model: "x".repeat(100),
            model_route: "y".repeat(100),
            success: true,
        };
        let record = TokenUsageRecord::new(1, &input);
        let buf = record.encode();
        let decoded = TokenUsageRecord::decode(&buf).unwrap();
        // Truncated to field capacity.
        assert_eq!(decoded.provider_model.len(), PROVIDER_MODEL_LEN);
        assert_eq!(decoded.model_route.len(), MODEL_ROUTE_LEN);
    }
}
