use std::fmt;

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

#[derive(Debug)]
pub enum ShimError {
    InvalidRequest(String),
    ResponseNotFound(String),
    Provider(String),
    ProviderProtocol(String),
    UnsupportedFeature { code: &'static str, message: String },
    Transport(String),
}

impl fmt::Display for ShimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShimError::InvalidRequest(msg) => write!(f, "invalid request: {}", msg),
            ShimError::ResponseNotFound(id) => write!(f, "response not found: {}", id),
            ShimError::Provider(msg) => write!(f, "provider error: {}", msg),
            ShimError::ProviderProtocol(msg) => write!(f, "provider protocol error: {}", msg),
            ShimError::UnsupportedFeature { message, .. } => {
                write!(f, "unsupported feature: {}", message)
            }
            ShimError::Transport(msg) => write!(f, "upstream transport error: {}", msg),
        }
    }
}

impl std::error::Error for ShimError {}

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
