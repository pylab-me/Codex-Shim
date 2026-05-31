use reqwest::{Client, StatusCode};
use serde_json::Value;
use tracing::debug;

use crate::error::ShimError;

/// A generic OpenAI-compatible API client.
/// Works with any provider that implements `/chat/completions`.
#[derive(Clone)]
pub struct ProviderClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ProviderClient {
    /// Create a new client with a properly configured reqwest backend.
    pub fn new(
        base_url: &str,
        timeout_secs: u64,
        user_agent: &str,
        trust_env: bool,
        http2_prior_knowledge: bool,
    ) -> anyhow::Result<Self> {
        let mut builder = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .user_agent(user_agent);
        if !trust_env {
            builder = builder.no_proxy();
        }
        if http2_prior_knowledge {
            builder = builder.http2_prior_knowledge();
        }
        Ok(Self {
            client: builder.build()?,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: String::new(),
        })
    }

    /// Set or update the API key.
    pub fn set_api_key(&mut self, key: String) {
        self.api_key = key;
    }

    /// Send a chat completions request to the provider.
    pub async fn chat_completions(&self, payload: Value) -> Result<ChatResult, ShimError> {
        let url = format!("{}/chat/completions", self.base_url);
        let request = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .json(&payload);

        let response = request.send().await.map_err(|err| ShimError::Transport(err.to_string()))?;
        let status = response.status();
        let body = response.text().await.map_err(|err| ShimError::Transport(err.to_string()))?;
        debug!(
            status = status.as_u16(),
            body = %body,
            "provider upstream raw response"
        );
        if !status.is_success() {
            return Err(ShimError::Provider(format!("{} {}", status.as_u16(), body)));
        }

        let raw: Value = serde_json::from_str(&body).map_err(|err| {
            ShimError::Provider(format!("invalid provider JSON: {err}; body={body}"))
        })?;
        Self::parse_chat_result(raw, status)
    }

    fn parse_chat_result(raw: Value, _status: StatusCode) -> Result<ChatResult, ShimError> {
        let choice = raw
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .ok_or_else(|| ShimError::Provider("missing choices[0]".to_string()))?;
        let message = choice
            .get("message")
            .cloned()
            .ok_or_else(|| ShimError::Provider("missing choices[0].message".to_string()))?;
        let finish_reason =
            choice.get("finish_reason").and_then(Value::as_str).map(ToOwned::to_owned);
        let usage = raw.get("usage").cloned();
        Ok(ChatResult {
            raw,
            message,
            finish_reason,
            usage,
        })
    }
}

/// Parsed chat completion result from any OpenAI-compatible provider.
#[derive(Debug)]
pub struct ChatResult {
    pub raw: Value,
    pub message: Value,
    pub finish_reason: Option<String>,
    pub usage: Option<Value>,
}
