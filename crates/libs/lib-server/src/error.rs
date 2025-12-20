//! Production-grade error handling for MCP Agent Mail Server.
//!
//! Follows NIST SP 800-53 error handling guidelines:
//! - Fail-secure: Never expose internal implementation details
//! - Structured responses: Machine-readable error codes
//! - Proper HTTP status mapping: Semantic status codes for API clients
//! - Defense in depth: Multiple layers of error sanitization

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

/// Error codes for machine-readable error classification.
/// These codes are stable and can be used for client-side error handling.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // 4xx Client Errors
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    ValidationError,

    // 5xx Server Errors
    InternalError,
    DatabaseError,
    #[allow(dead_code)]
    ServiceUnavailable,
    ConfigError,
}

impl ErrorCode {
    /// Returns the string representation of the error code.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::BadRequest => "BAD_REQUEST",
            ErrorCode::Unauthorized => "UNAUTHORIZED",
            ErrorCode::Forbidden => "FORBIDDEN",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::ValidationError => "VALIDATION_ERROR",
            ErrorCode::InternalError => "INTERNAL_ERROR",
            ErrorCode::DatabaseError => "DATABASE_ERROR",
            ErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            ErrorCode::ConfigError => "CONFIG_ERROR",
        }
    }
}

/// Structured error response following RFC 7807 Problem Details pattern.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Machine-readable error code for client-side handling.
    pub code: &'static str,
    /// Human-readable error message (safe for display).
    pub error: String,
    /// Optional details for debugging (only in non-production or for safe errors).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Optional suggestions for similar entities (for NotFound errors).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
}

impl ErrorResponse {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str(),
            error: message.into(),
            details: None,
            suggestions: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }
}

/// Server error type with production-hardened error handling.
#[derive(Debug, Error)]
pub enum ServerError {
    // -- External Errors (wrapped)
    #[error("Database error")]
    Database(#[from] lib_core::Error),

    #[error("IO error")]
    Io(#[from] std::io::Error),

    // -- API-Specific Errors
    #[allow(dead_code)]
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[allow(dead_code)]
    #[error("Resource conflict: {0}")]
    Conflict(String),

    #[allow(dead_code)]
    #[error("Validation error: {0}")]
    Validation(String),

    #[allow(dead_code)]
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[allow(dead_code)]
    #[error("Unauthorized")]
    Unauthorized,

    #[allow(dead_code)]
    #[error("Forbidden")]
    Forbidden,

    #[allow(dead_code)]
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[allow(dead_code)]
    #[error("Internal server error: {0}")]
    Internal(String),
}

#[allow(dead_code)]
impl ServerError {
    /// Creates a NotFound error for a specific resource.
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound(resource.into())
    }

    /// Creates a Conflict error (e.g., duplicate entry).
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    /// Creates a Validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    /// Creates a BadRequest error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }
}

/// Determines if an error message indicates a unique constraint violation.
fn is_unique_constraint_error(msg: &str) -> bool {
    let msg_lower = msg.to_lowercase();
    msg_lower.contains("unique constraint failed")
        || msg_lower.contains("duplicate key")
        || msg_lower.contains("already exists")
}

/// Extracts a user-friendly message from a constraint error.
fn extract_conflict_message(msg: &str) -> String {
    // Parse common patterns like "UNIQUE constraint failed: agents.project_id, agents.name"
    if let Some(idx) = msg.find("UNIQUE constraint failed:") {
        let rest = &msg[idx + 25..];
        // Get the last field name (e.g., "name" from "agents.name")
        if let Some(table_field) = rest.split('.').next_back() {
            let field = table_field
                .split(',')
                .next()
                .unwrap_or(table_field)
                .trim()
                .trim_end_matches('`'); // Remove trailing backtick if present
            return format!("A record with this {} already exists", field);
        }
    }
    "A record with these values already exists".to_string()
}

/// Sanitizes error messages to prevent information leakage.
/// Never expose internal SQL errors, file paths, or stack traces.
fn sanitize_error_message(error: &lib_core::Error) -> String {
    match error {
        lib_core::Error::ProjectNotFound { identifier, .. } => {
            format!("Project not found: {}", identifier)
        }
        lib_core::Error::AgentNotFound { name, .. } => format!("Agent not found: {}", name),
        lib_core::Error::MessageNotFound(id) => format!("Message not found: {}", id),
        lib_core::Error::FileReservationNotFound(id) => {
            format!("File reservation not found: {}", id)
        }
        lib_core::Error::ProductNotFound(id) => format!("Product not found: {}", id),
        lib_core::Error::MacroNotFound(name) => format!("Macro not found: {}", name),
        lib_core::Error::BuildSlotNotFound(id) => format!("Build slot not found: {}", id),
        lib_core::Error::NotFound => "Resource not found".to_string(),
        lib_core::Error::InvalidInput(msg) => format!("Invalid input: {}", msg),
        lib_core::Error::AuthError => "Authentication failed".to_string(),
        // For database errors, check if it's a unique constraint
        lib_core::Error::Libsql(e) => {
            let msg = e.to_string();
            if is_unique_constraint_error(&msg) {
                extract_conflict_message(&msg)
            } else {
                // Don't expose raw SQL errors
                "Database operation failed".to_string()
            }
        }
        lib_core::Error::Git2(_) => "Version control operation failed".to_string(),
        lib_core::Error::SerdeJson(_) => "Invalid JSON format".to_string(),
        lib_core::Error::Io(_) => "File operation failed".to_string(),
        lib_core::Error::LockTimeout { .. } => "Lock acquisition timed out".to_string(),
        lib_core::Error::Validation(ve) => ve.to_string(),
        lib_core::Error::Image(_) => "Image processing failed".to_string(),
        lib_core::Error::EncryptionError(_) => "Encryption operation failed".to_string(),
        lib_core::Error::DecryptionError(_) => "Decryption operation failed".to_string(),
    }
}

/// Maps lib_core::Error to appropriate HTTP status code.
fn map_core_error_to_status(error: &lib_core::Error) -> StatusCode {
    match error {
        lib_core::Error::ProjectNotFound { .. }
        | lib_core::Error::AgentNotFound { .. }
        | lib_core::Error::MessageNotFound(_)
        | lib_core::Error::FileReservationNotFound(_)
        | lib_core::Error::ProductNotFound(_)
        | lib_core::Error::MacroNotFound(_)
        | lib_core::Error::BuildSlotNotFound(_)
        | lib_core::Error::NotFound => StatusCode::NOT_FOUND,

        lib_core::Error::InvalidInput(_) => StatusCode::BAD_REQUEST,
        lib_core::Error::AuthError => StatusCode::UNAUTHORIZED,
        lib_core::Error::SerdeJson(_) => StatusCode::BAD_REQUEST,

        lib_core::Error::Libsql(e) => {
            if is_unique_constraint_error(&e.to_string()) {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }

        lib_core::Error::Git2(_) | lib_core::Error::Io(_) | lib_core::Error::LockTimeout { .. } => {
            StatusCode::INTERNAL_SERVER_ERROR
        }

        lib_core::Error::Validation(_) => StatusCode::BAD_REQUEST,
        lib_core::Error::Image(_) => StatusCode::BAD_REQUEST,
        lib_core::Error::EncryptionError(_) | lib_core::Error::DecryptionError(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

/// Maps lib_core::Error to appropriate ErrorCode.
fn map_core_error_to_code(error: &lib_core::Error) -> ErrorCode {
    match error {
        lib_core::Error::ProjectNotFound { .. }
        | lib_core::Error::AgentNotFound { .. }
        | lib_core::Error::MessageNotFound(_)
        | lib_core::Error::FileReservationNotFound(_)
        | lib_core::Error::ProductNotFound(_)
        | lib_core::Error::MacroNotFound(_)
        | lib_core::Error::BuildSlotNotFound(_)
        | lib_core::Error::NotFound => ErrorCode::NotFound,

        lib_core::Error::InvalidInput(_)
        | lib_core::Error::SerdeJson(_)
        | lib_core::Error::Validation(_) => ErrorCode::ValidationError,

        lib_core::Error::AuthError => ErrorCode::Unauthorized,

        lib_core::Error::Libsql(e) => {
            if is_unique_constraint_error(&e.to_string()) {
                ErrorCode::Conflict
            } else {
                ErrorCode::DatabaseError
            }
        }

        lib_core::Error::Git2(_) | lib_core::Error::Io(_) | lib_core::Error::LockTimeout { .. } => {
            ErrorCode::InternalError
        }

        lib_core::Error::Image(_) => ErrorCode::ValidationError,
        lib_core::Error::EncryptionError(_) | lib_core::Error::DecryptionError(_) => {
            ErrorCode::InternalError
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        // Log the full error for debugging (server-side only)
        tracing::error!(error = ?self, "Request error");

        let (status, response) = match self {
            ServerError::Database(ref e) => {
                let status = map_core_error_to_status(e);
                let code = map_core_error_to_code(e);
                let message = sanitize_error_message(e);
                let suggestions = e.suggestions().to_vec();
                (
                    status,
                    ErrorResponse::new(code, message).with_suggestions(suggestions),
                )
            }

            ServerError::NotFound(ref msg) => (
                StatusCode::NOT_FOUND,
                ErrorResponse::new(ErrorCode::NotFound, msg.clone()),
            ),

            ServerError::Conflict(ref msg) => (
                StatusCode::CONFLICT,
                ErrorResponse::new(ErrorCode::Conflict, msg.clone()),
            ),

            ServerError::Validation(ref msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse::new(ErrorCode::ValidationError, msg.clone()),
            ),

            ServerError::BadRequest(ref msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse::new(ErrorCode::BadRequest, msg.clone()),
            ),

            ServerError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                ErrorResponse::new(ErrorCode::Unauthorized, "Authentication required"),
            ),

            ServerError::Forbidden => (
                StatusCode::FORBIDDEN,
                ErrorResponse::new(ErrorCode::Forbidden, "Access denied"),
            ),

            ServerError::ConfigError(ref msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new(ErrorCode::ConfigError, msg.clone()),
            ),

            ServerError::Io(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new(ErrorCode::InternalError, "File operation failed"),
            ),

            ServerError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new(ErrorCode::InternalError, "An internal error occurred"),
            ),
        };

        (status, Json(response)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_constraint_detection() {
        assert!(is_unique_constraint_error(
            "UNIQUE constraint failed: agents.project_id, agents.name"
        ));
        assert!(is_unique_constraint_error("duplicate key value"));
        assert!(is_unique_constraint_error("Record already exists"));
        assert!(!is_unique_constraint_error("Some other error"));
    }

    #[test]
    fn test_conflict_message_extraction() {
        let msg = "UNIQUE constraint failed: agents.project_id, agents.name";
        let result = extract_conflict_message(msg);
        assert!(result.contains("name"));
    }

    #[test]
    fn test_error_response_serialization() {
        let resp = ErrorResponse::new(ErrorCode::NotFound, "Agent not found");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("Agent not found"));
        // Empty suggestions should not be serialized
        assert!(!json.contains("suggestions"));
    }

    #[test]
    fn test_error_response_with_suggestions() {
        let resp = ErrorResponse::new(ErrorCode::NotFound, "Agent not found")
            .with_suggestions(vec!["claude_1".to_string(), "claude_2".to_string()]);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("suggestions"));
        assert!(json.contains("claude_1"));
        assert!(json.contains("claude_2"));
    }
}
