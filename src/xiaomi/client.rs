use reqwest::{Client, StatusCode};
use serde_json::Value;

use super::schema::ChatResult;
use crate::config::AppConfig;
use crate::error::ShimError;

#[derive(Clone)]
pub struct XiaomiClient {
    client: Client,
    config: AppConfig,
}

impl XiaomiClient {
    pub fn new(config: AppConfig) -> anyhow::Result<Self> {
        let mut builder = Client::builder()
            .timeout(config.request_timeout)
            .user_agent(config.user_agent.clone())
            .no_proxy();
        if config.trust_env {
            // reqwest trusts env proxies by default. The explicit branch documents intent.
            builder = Client::builder()
                .timeout(config.request_timeout)
                .user_agent(config.user_agent.clone());
        }
        if config.http2_prior_knowledge {
            builder = builder.http2_prior_knowledge();
        }
        Ok(Self {
            client: builder.build()?,
            config,
        })
    }

    pub async fn chat_completions(&self, payload: Value) -> Result<ChatResult, ShimError> {
        let url = format!(
            "{}/chat/completions",
            self.config.mimo_base_url.trim_end_matches('/')
        );
        let request = self
            .client
            .post(&url)
            .bearer_auth(&self.config.mimo_api_key)
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .json(&payload);

        let response = request
            .send()
            .await
            .map_err(|err| ShimError::Transport(err.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|err| ShimError::Transport(err.to_string()))?;
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
        let finish_reason = choice
            .get("finish_reason")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let usage = raw.get("usage").cloned();
        Ok(ChatResult {
            raw,
            message,
            finish_reason,
            usage,
        })
    }
}
