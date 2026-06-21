use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct ChatResult {
    pub raw: Value,
    pub message: Value,
    pub finish_reason: Option<String>,
    pub usage: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderTokenStats {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cached_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
}

impl ProviderTokenStats {
    pub fn from_usage(usage: Option<&Value>) -> Self {
        let Some(usage) = usage else {
            return Self::default();
        };
        Self {
            input_tokens: preferred_nonzero_number_field(usage, "input_tokens", "prompt_tokens"),
            output_tokens: preferred_nonzero_number_field(
                usage,
                "output_tokens",
                "completion_tokens",
            ),
            total_tokens: number_field(usage, &["total_tokens"]),
            cached_tokens: nested_number_field(
                usage,
                &[
                    ("input_tokens_details", "cached_tokens"),
                    ("prompt_tokens_details", "cached_tokens"),
                ],
            ),
            reasoning_tokens: nested_number_field(
                usage,
                &[
                    ("output_tokens_details", "reasoning_tokens"),
                    ("completion_tokens_details", "reasoning_tokens"),
                ],
            ),
        }
    }

    pub fn has_any(&self) -> bool {
        self.input_tokens.is_some()
            || self.output_tokens.is_some()
            || self.total_tokens.is_some()
            || self.cached_tokens.is_some()
            || self.reasoning_tokens.is_some()
    }

    pub fn as_json(&self) -> Value {
        json!({
            "input_tokens": self.input_tokens,
            "output_tokens": self.output_tokens,
            "total_tokens": self.total_tokens,
            "cached_tokens": self.cached_tokens,
            "reasoning_tokens": self.reasoning_tokens,
            "source": "provider_usage_nonstandard_observation"
        })
    }
}

fn number_field(value: &Value, names: &[&str]) -> Option<u64> {
    names
        .iter()
        .find_map(|name| value.get(*name).and_then(Value::as_u64))
}

fn preferred_nonzero_number_field(value: &Value, primary: &str, fallback: &str) -> Option<u64> {
    let primary_value = value.get(primary).and_then(Value::as_u64);
    let fallback_value = value.get(fallback).and_then(Value::as_u64);
    match (primary_value, fallback_value) {
        (Some(0), Some(other)) if other > 0 => Some(other),
        (Some(current), _) => Some(current),
        (None, Some(other)) => Some(other),
        (None, None) => None,
    }
}

fn nested_number_field(value: &Value, names: &[(&str, &str)]) -> Option<u64> {
    names.iter().find_map(|(outer, inner)| {
        value
            .get(*outer)
            .and_then(|obj| obj.get(*inner))
            .and_then(Value::as_u64)
    })
}
