use serde_json::{Value, json};

/// Convert Codex/Responses function tool declarations into Chat Completions tools.
/// Non-function/built-in tools are intentionally dropped: Codex owns tools, this shim
/// only forwards client-owned function schemas to Xiaomi MiMo.
pub fn responses_tools_to_chat_tools(tools: Option<&Value>) -> Vec<Value> {
    let Some(items) = tools.and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|tool| {
            let obj = tool.as_object()?;
            if obj.get("type").and_then(Value::as_str) != Some("function") {
                return None;
            }
            let name = obj.get("name").and_then(Value::as_str)?.to_string();
            let description = obj.get("description").cloned().unwrap_or_else(|| json!(""));
            let parameters = obj
                .get("parameters")
                .cloned()
                .unwrap_or_else(|| json!({"type":"object","properties":{}}));
            Some(json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": description,
                    "parameters": parameters
                }
            }))
        })
        .collect()
}

pub fn chat_tool_calls_to_responses_items(tool_calls: &[Value]) -> Vec<Value> {
    tool_calls
        .iter()
        .filter_map(|call| {
            let obj = call.as_object()?;
            let call_id = obj.get("id").and_then(Value::as_str).unwrap_or_else(|| {
                obj.get("call_id")
                    .and_then(Value::as_str)
                    .unwrap_or("call_local_unknown")
            });
            let function = obj.get("function").and_then(Value::as_object)?;
            let name = function.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = normalize_tool_arguments(function.get("arguments"));
            Some(json!({
                "id": format!("fc_{}", call_id),
                "type": "function_call",
                "status": "completed",
                "call_id": call_id,
                "name": name,
                "arguments": arguments
            }))
        })
        .collect()
}

pub fn response_function_call_item_to_chat_tool_call(item: &Value) -> Value {
    let call_id = item
        .get("call_id")
        .and_then(Value::as_str)
        .or_else(|| item.get("id").and_then(Value::as_str))
        .unwrap_or("call_local_unknown");
    let name = item.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = normalize_tool_arguments(item.get("arguments"));
    json!({
        "id": call_id,
        "type": "function",
        "function": {
            "name": name,
            "arguments": arguments
        }
    })
}

fn normalize_tool_arguments(arguments: Option<&Value>) -> String {
    match arguments {
        Some(Value::String(text)) => text.clone(),
        Some(other) => serde_json::to_string(other).unwrap_or_else(|_| "{}".to_string()),
        None => "{}".to_string(),
    }
}

pub fn make_assistant_tool_calls_message(tool_calls: Vec<Value>) -> Value {
    let tool_calls = tool_calls
        .into_iter()
        .map(normalize_chat_tool_call)
        .collect::<Vec<_>>();
    json!({
        "role": "assistant",
        "content": null,
        "tool_calls": tool_calls
    })
}

fn normalize_chat_tool_call(call: Value) -> Value {
    let Some(obj) = call.as_object() else {
        return call;
    };

    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| obj.get("call_id").and_then(Value::as_str))
        .unwrap_or("call_local_unknown");
    let function = obj.get("function").and_then(Value::as_object);
    let name = function
        .and_then(|function| function.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let arguments =
        normalize_tool_arguments(function.and_then(|function| function.get("arguments")));

    json!({
        "id": id,
        "type": "function",
        "function": {
            "name": name,
            "arguments": arguments
        }
    })
}

pub fn make_tool_message(call_id: &str, output: &Value) -> Value {
    let content = match output {
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    };
    json!({
        "role": "tool",
        "tool_call_id": call_id,
        "content": content
    })
}
