use serde_json::{Map, Value, json};

/// Controls Xiaomi MiMo thinking-mode compatibility.
///
/// Xiaomi MiMo's OpenAI-compatible Chat Completions endpoint can require the
/// assistant `reasoning_content` returned by a thinking-mode response to be
/// passed back on later turns.
///
/// Important distinction:
/// - `thinking_enabled=true` means the shim explicitly sends Xiaomi's
///   non-standard top-level `thinking: {"type":"enabled"}` option and fills
///   missing assistant `reasoning_content` with an empty string.
/// - Existing/provider-returned assistant `reasoning_content` is always
///   preserved and replayed. Some Xiaomi models/endpoints may return or require
///   this field even when the shim did not explicitly send the top-level
///   thinking option.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReasoningPolicy {
    pub thinking_enabled: bool,
}

impl ReasoningPolicy {
    pub fn should_send_thinking_option(self) -> bool {
        self.thinking_enabled
    }

    pub fn should_fill_missing_reasoning(self) -> bool {
        self.thinking_enabled
    }
}

/// Add Xiaomi's non-standard thinking flag to the upstream Chat Completions
/// payload when the shim is explicitly configured to request thinking mode.
pub fn insert_thinking_option(payload: &mut Map<String, Value>, policy: ReasoningPolicy) {
    if policy.should_send_thinking_option() {
        payload.insert("thinking".to_string(), json!({"type": "enabled"}));
    }
}

/// Apply the current policy to outbound chat messages before sending them to
/// Xiaomi and before using them as the base continuation state.
pub fn apply_reasoning_policy_to_chat_messages(messages: &mut [Value], policy: ReasoningPolicy) {
    for message in messages {
        apply_reasoning_policy_to_assistant_message(message, None, policy);
    }
}

/// Preserve `reasoning_content` from incoming assistant messages whenever it is
/// present. Do not gate this on `thinking_enabled`: the flag only controls
/// whether the shim explicitly asks Xiaomi to enter thinking mode, while
/// pass-back is a compatibility requirement once Xiaomi has emitted the field.
pub fn copy_incoming_reasoning_content(
    source: &Map<String, Value>,
    target: &mut Map<String, Value>,
    role: &str,
    _policy: ReasoningPolicy,
) {
    if role != "assistant" {
        return;
    }
    if let Some(reasoning_content) =
        source.get("reasoning_content").and_then(Value::as_str).map(ToOwned::to_owned)
    {
        target.insert("reasoning_content".to_string(), json!(reasoning_content));
    }
}

pub fn apply_reasoning_policy_to_assistant_message(
    message: &mut Value,
    reasoning_content: Option<&str>,
    policy: ReasoningPolicy,
) {
    let Some(obj) = message.as_object_mut() else {
        return;
    };
    if obj.get("role").and_then(Value::as_str) != Some("assistant") {
        return;
    }

    let existing = obj.get("reasoning_content").and_then(Value::as_str);
    if let Some(value) = reasoning_content.or(existing) {
        obj.insert("reasoning_content".to_string(), json!(value));
        return;
    }

    if policy.should_fill_missing_reasoning() {
        obj.insert("reasoning_content".to_string(), json!(""));
    }
}

pub fn extract_reasoning_content(message: &Value) -> Option<String> {
    message.get("reasoning_content").and_then(Value::as_str).map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_empty_reasoning_content_for_assistant_in_explicit_thinking_mode() {
        let policy = ReasoningPolicy {
            thinking_enabled: true,
        };
        let mut messages = vec![json!({"role":"assistant", "content":"hello"})];

        apply_reasoning_policy_to_chat_messages(&mut messages, policy);

        assert_eq!(messages[0]["reasoning_content"], json!(""));
    }

    #[test]
    fn does_not_fabricate_missing_reasoning_content_when_explicit_thinking_is_disabled() {
        let policy = ReasoningPolicy {
            thinking_enabled: false,
        };
        let mut messages = vec![json!({"role":"assistant", "content":"hello"})];

        apply_reasoning_policy_to_chat_messages(&mut messages, policy);

        assert!(messages[0].get("reasoning_content").is_none());
    }

    #[test]
    fn preserves_existing_reasoning_content_even_when_explicit_thinking_is_disabled() {
        let policy = ReasoningPolicy {
            thinking_enabled: false,
        };
        let mut messages = vec![json!({
            "role":"assistant",
            "content":"hello",
            "reasoning_content":"provider reasoning"
        })];

        apply_reasoning_policy_to_chat_messages(&mut messages, policy);

        assert_eq!(
            messages[0]["reasoning_content"],
            json!("provider reasoning")
        );
    }
}
