//! Errors returned to clients, OpenAI style: `{"error":{"code":..,"message":..}}`.
//! Codes are stable for clients to match on; messages are not.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Error codes used in the body (docs/m0/gateway.md).
pub mod codes {
    /// Unknown or revoked virtual key.
    pub const INVALID_API_KEY: &str = "invalid_api_key";
    /// Peer UID may not call the admin API.
    pub const FORBIDDEN: &str = "forbidden";
    /// M0 supports only `stream: false`.
    pub const STREAM_NOT_SUPPORTED: &str = "stream_not_supported";
    /// No route is allowed for the request's privacy label.
    pub const NO_ROUTE_PRIVACY: &str = "no_route_privacy";
    /// Every candidate was tried and failed.
    pub const UPSTREAM_UNAVAILABLE: &str = "upstream_unavailable";
    /// Logical model name not configured.
    pub const MODEL_NOT_FOUND: &str = "model_not_found";
    /// Session does not exist.
    pub const SESSION_NOT_FOUND: &str = "session_not_found";
    /// Session already exists for another uid.
    pub const SESSION_UID_MISMATCH: &str = "session_uid_mismatch";
    /// Malformed request.
    pub const INVALID_REQUEST: &str = "invalid_request";
    /// Internal error.
    pub const INTERNAL_ERROR: &str = "internal_error";
}

/// The `error` part of an error body.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

/// Error envelope `{"error":{..}}`.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

/// Error returned to a client.
#[derive(Debug, Clone)]
pub struct GatewayError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl GatewayError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    /// 401: unknown or revoked virtual key.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, codes::INVALID_API_KEY, message)
    }

    /// 403: peer may not call the admin API.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, codes::FORBIDDEN, message)
    }

    /// 400: malformed request.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, codes::INVALID_REQUEST, message)
    }

    /// 400: streaming is not supported in M0.
    pub fn stream_not_supported() -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            codes::STREAM_NOT_SUPPORTED,
            "streaming is not supported in M0; use stream=false",
        )
    }

    /// 409: no route for the request's privacy label.
    pub fn no_route(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, codes::NO_ROUTE_PRIVACY, message)
    }

    /// 503: every candidate failed.
    pub fn upstream_unavailable(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            codes::UPSTREAM_UNAVAILABLE,
            message,
        )
    }

    /// 404: logical model not configured.
    pub fn model_not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, codes::MODEL_NOT_FOUND, message)
    }

    /// 404: session does not exist.
    pub fn session_not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, codes::SESSION_NOT_FOUND, message)
    }

    /// 409: session already exists for another uid.
    pub fn session_uid_mismatch(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, codes::SESSION_UID_MISMATCH, message)
    }

    /// 500: internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            codes::INTERNAL_ERROR,
            message,
        )
    }

    /// Envelope for serialization.
    pub fn envelope(&self) -> ErrorEnvelope {
        ErrorEnvelope {
            error: ErrorBody {
                code: self.code.to_string(),
                message: self.message.clone(),
            },
        }
    }
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}): {}",
            self.code,
            self.status.as_u16(),
            self.message
        )
    }
}

impl std::error::Error for GatewayError {}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        (self.status, Json(self.envelope())).into_response()
    }
}
