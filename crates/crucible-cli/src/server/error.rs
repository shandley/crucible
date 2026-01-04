//! API error types and handling.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// API error type.
#[derive(Debug)]
#[allow(dead_code)] // Variants kept for API completeness
pub enum ApiError {
    /// Resource not found.
    NotFound(String),
    /// Bad request from client.
    BadRequest(String),
    /// Conflict (e.g., duplicate decision).
    Conflict(String),
    /// Internal server error.
    Internal(String),
    /// Error from crucible library.
    Crucible(crucible::CrucibleError),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", msg),
            ApiError::Crucible(e) => (StatusCode::BAD_REQUEST, "crucible_error", e.to_string()),
        };

        (
            status,
            Json(ErrorResponse {
                error: error.to_string(),
                message,
            }),
        )
            .into_response()
    }
}

impl From<crucible::CrucibleError> for ApiError {
    fn from(err: crucible::CrucibleError) -> Self {
        ApiError::Crucible(err)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            ApiError::Internal(msg) => write!(f, "Internal error: {}", msg),
            ApiError::Crucible(e) => write!(f, "Crucible error: {}", e),
        }
    }
}

impl std::error::Error for ApiError {}
