use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::ShimError;
use crate::responses::tools::{
    chat_tool_calls_to_responses_items, make_assistant_tool_calls_message,
};
use crate::xiaomi::reasoning::{
    ReasoningPolicy, apply_reasoning_policy_to_assistant_message, extract_reasoning_content,
};
use crate::xiaomi::schema::{ChatResult, ProviderTokenStats};

#[derive(Debug, Clone)]
pub struct BuiltResponse {
    pub response_object: Value,
    pub updated_chat_messages: Vec<Value>,
}

pub fn build_response_object(
    response_id: &str,
    client_model: &str,
    base_chat_messages: &[Value],
    chat_result: ChatResult,
    parallel_tool_calls: bool,
    store: bool,
    reasoning_policy: ReasoningPolicy,
) -> Result<BuiltResponse, ShimError> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let message = chat_result.message;
    let tool_calls = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut updated = base_chat_messages.to_vec();
    let reasoning_content = extract_reasoning_content(&message);
    let (output, output_text) = if !tool_calls.is_empty() {
        let mut assistant_message = make_assistant_tool_calls_message(tool_calls.clone());
        apply_reasoning_policy_to_assistant_message(
            &mut assistant_message,
            reasoning_content.as_deref(),
            reasoning_policy,
        );
        updated.push(assistant_message);
        (
            chat_tool_calls_to_responses_items(&tool_calls),
            String::new(),
        )
    } else {
        let text = message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let mut assistant_message = json!({"role":"assistant", "content": text});
        apply_reasoning_policy_to_assistant_message(
            &mut assistant_message,
            reasoning_content.as_deref(),
            reasoning_policy,
        );
        updated.push(assistant_message);
        let item = json!({
            "id": format!("msg_local_{}", Uuid::new_v4().simple()),
            "type": "message",
            "status": "completed",
            "role": "assistant",
            "content": [{
                "type": "output_text",
                "text": text,
                "annotations": []
            }]
        });
        (vec![item], text)
    };

    let provider_usage = chat_result.usage.clone();
    let observed_token_stats = ProviderTokenStats::from_usage(provider_usage.as_ref());
    let responses_usage = normalize_responses_usage(provider_usage.as_ref())?;

    let mut response = json!({
        "id": response_id,
        "object": "response",
        "created_at": now,
        "status": "completed",
        "model": client_model,
        "output": output,
        "output_text": output_text,
        "error": null,
        "incomplete_details": null,
        "parallel_tool_calls": parallel_tool_calls,
        "store": store,
        // Codex parses response.completed as a Responses API object. The shim
        // converts provider token accounting only when all required counts are
        // present. It never fabricates input/output/total token counts.
        "usage": responses_usage
    });

    response["local_gateway"] = json!({
        "name": "codex-mimo-shim",
        "backend": "xiaomimimo",
        "endpoint_used": "chat.completions",
        "finish_reason": chat_result.finish_reason,
        "provider_raw_id": chat_result.raw.get("id").cloned().unwrap_or(Value::Null),
        "provider_usage": provider_usage.unwrap_or(Value::Null),
        "observed_token_stats": observed_token_stats.as_json()
    });

    Ok(BuiltResponse {
        response_object: response,
        updated_chat_messages: updated,
    })
}

fn normalize_responses_usage(provider_usage: Option<&Value>) -> Result<Value, ShimError> {
    let Some(usage) = provider_usage else {
        return Err(ShimError::ProviderProtocol(
            "missing_provider_usage: provider response is missing usage".to_string(),
        ));
    };

    let input_tokens =
        number_field(usage, &["input_tokens", "prompt_tokens"]).ok_or_else(|| {
            ShimError::ProviderProtocol(
                "missing_provider_usage: usage is missing input_tokens/prompt_tokens".to_string(),
            )
        })?;
    let output_tokens =
        number_field(usage, &["output_tokens", "completion_tokens"]).ok_or_else(|| {
            ShimError::ProviderProtocol(
                "missing_provider_usage: usage is missing output_tokens/completion_tokens"
                    .to_string(),
            )
        })?;
    let total_tokens = number_field(usage, &["total_tokens"]).ok_or_else(|| {
        ShimError::ProviderProtocol(
            "missing_provider_usage: usage is missing total_tokens".to_string(),
        )
    })?;

    let cached_tokens = nested_number_field(
        usage,
        &[
            ("input_tokens_details", "cached_tokens"),
            ("prompt_tokens_details", "cached_tokens"),
        ],
    )
    .unwrap_or(0);

    let reasoning_tokens = nested_number_field(
        usage,
        &[
            ("output_tokens_details", "reasoning_tokens"),
            ("completion_tokens_details", "reasoning_tokens"),
        ],
    )
    .unwrap_or(0);

    Ok(json!({
        "input_tokens": input_tokens,
        "input_tokens_details": {
            "cached_tokens": cached_tokens
        },
        "output_tokens": output_tokens,
        "output_tokens_details": {
            "reasoning_tokens": reasoning_tokens
        },
        "total_tokens": total_tokens
    }))
}

fn number_field(value: &Value, names: &[&str]) -> Option<u64> {
    names
        .iter()
        .find_map(|name| value.get(*name).and_then(Value::as_u64))
}

fn nested_number_field(value: &Value, names: &[(&str, &str)]) -> Option<u64> {
    names.iter().find_map(|(outer, inner)| {
        value
            .get(*outer)
            .and_then(|obj| obj.get(*inner))
            .and_then(Value::as_u64)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage() -> Value {
        json!({
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        })
    }

    #[test]
    fn stores_assistant_reasoning_content_for_text_response_when_enabled() {
        let chat_result = ChatResult {
            raw: json!({"id":"chatcmpl_test"}),
            message: json!({
                "role": "assistant",
                "content": "final answer",
                "reasoning_content": "provider reasoning"
            }),
            finish_reason: Some("stop".to_string()),
            usage: Some(usage()),
        };
        let policy = ReasoningPolicy {
            thinking_enabled: true,
        };

        let built = build_response_object(
            "resp_test",
            "mimo-v2.5-pro",
            &[],
            chat_result,
            false,
            true,
            policy,
        )
        .expect("response should build");

        assert_eq!(
            built.updated_chat_messages[0]["reasoning_content"],
            json!("provider reasoning")
        );
    }

    #[test]
    fn stores_assistant_reasoning_content_for_tool_calls_when_enabled() {
        let chat_result = ChatResult {
            raw: json!({"id":"chatcmpl_test"}),
            message: json!({
                "role": "assistant",
                "content": null,
                "reasoning_content": "provider reasoning",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "lookup", "arguments": "{}"}
                }]
            }),
            finish_reason: Some("tool_calls".to_string()),
            usage: Some(usage()),
        };
        let policy = ReasoningPolicy {
            thinking_enabled: true,
        };

        let built = build_response_object(
            "resp_test",
            "mimo-v2.5-pro",
            &[],
            chat_result,
            true,
            true,
            policy,
        )
        .expect("response should build");

        assert_eq!(
            built.updated_chat_messages[0]["reasoning_content"],
            json!("provider reasoning")
        );
        assert!(built.updated_chat_messages[0]["tool_calls"].is_array());
    }

    #[test]
    fn preserves_provider_reasoning_content_even_when_explicit_thinking_is_disabled() {
        let chat_result = ChatResult {
            raw: json!({"id":"chatcmpl_test"}),
            message: json!({
                "role": "assistant",
                "content": "final answer",
                "reasoning_content": "provider reasoning"
            }),
            finish_reason: Some("stop".to_string()),
            usage: Some(usage()),
        };
        let policy = ReasoningPolicy {
            thinking_enabled: false,
        };

        let built = build_response_object(
            "resp_test",
            "mimo-v2.5-pro",
            &[],
            chat_result,
            false,
            true,
            policy,
        )
        .expect("response should build");

        assert_eq!(
            built.updated_chat_messages[0]["reasoning_content"],
            json!("provider reasoning")
        );
    }
}
