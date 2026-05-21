use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShimError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("response not found: {0}")]
    ResponseNotFound(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("provider protocol error: {0}")]
    ProviderProtocol(String),

    #[error("unsupported feature: {message}")]
    UnsupportedFeature { code: &'static str, message: String },

    #[error("upstream transport error: {0}")]
    Transport(String),
}

impl ShimError {
    pub fn status(&self) -> StatusCode {
        match self {
            ShimError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            ShimError::ResponseNotFound(_) => StatusCode::NOT_FOUND,
            ShimError::Provider(_) => StatusCode::BAD_GATEWAY,
            ShimError::ProviderProtocol(_) => StatusCode::BAD_GATEWAY,
            ShimError::UnsupportedFeature { .. } => StatusCode::NOT_IMPLEMENTED,
            ShimError::Transport(_) => StatusCode::BAD_GATEWAY,
        }
    }

    pub fn error_type(&self) -> &'static str {
        match self {
            ShimError::InvalidRequest(_) => "invalid_request_error",
            ShimError::ResponseNotFound(_) => "not_found_error",
            ShimError::Provider(_) => "provider_error",
            ShimError::ProviderProtocol(_) => "provider_protocol_error",
            ShimError::UnsupportedFeature { .. } => "unsupported_feature",
            ShimError::Transport(_) => "gateway_error",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            ShimError::InvalidRequest(_) => "invalid_request",
            ShimError::ResponseNotFound(_) => "response_not_found",
            ShimError::Provider(_) => "provider_error",
            ShimError::ProviderProtocol(_) => "provider_protocol_error",
            ShimError::UnsupportedFeature { code, .. } => *code,
            ShimError::Transport(_) => "transport_error",
        }
    }

    pub fn payload(&self) -> Value {
        json!({
            "error": {
                "message": self.to_string(),
                "type": self.error_type(),
                "param": null,
                "code": self.code()
            }
        })
    }
}

impl IntoResponse for ShimError {
    fn into_response(self) -> Response {
        (self.status(), Json(self.payload())).into_response()
    }
}
