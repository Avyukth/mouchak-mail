//! Structured error codes for MCP tool responses

use rmcp::ErrorData as McpError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    AgentNotFound,
    AgentAlreadyExists,
    CapabilityDenied,

    ProjectNotFound,
    ProjectAlreadyExists,

    MessageNotFound,
    ThreadNotFound,
    InvalidRecipient,

    ReservationConflict,
    ReservationNotFound,
    ReservationExpired,

    ProductNotFound,
    ProductAlreadyExists,

    MacroNotFound,

    InvalidInput,
    InvalidAgentName,
    InvalidProjectKey,
    InvalidTtl,

    DatabaseError,
    InternalError,
}

impl ErrorCode {
    pub fn to_mcp_error(self, message: &str, context: Option<serde_json::Value>) -> McpError {
        let mut data = context.unwrap_or_else(|| serde_json::json!({}));

        if let Some(obj) = data.as_object_mut() {
            obj.insert(
                "error_code".to_string(),
                serde_json::to_value(self).unwrap_or_default(),
            );
        }

        match self {
            Self::AgentNotFound
            | Self::ProjectNotFound
            | Self::MessageNotFound
            | Self::ThreadNotFound
            | Self::ProductNotFound
            | Self::MacroNotFound
            | Self::ReservationNotFound => {
                McpError::invalid_params(message.to_string(), Some(data))
            }

            Self::AgentAlreadyExists
            | Self::ProjectAlreadyExists
            | Self::ProductAlreadyExists
            | Self::ReservationConflict => {
                McpError::invalid_params(message.to_string(), Some(data))
            }

            Self::CapabilityDenied
            | Self::InvalidRecipient
            | Self::InvalidInput
            | Self::InvalidAgentName
            | Self::InvalidProjectKey
            | Self::InvalidTtl
            | Self::ReservationExpired => McpError::invalid_params(message.to_string(), Some(data)),

            Self::DatabaseError | Self::InternalError => {
                McpError::internal_error(message.to_string(), Some(data))
            }
        }
    }

    pub fn with_suggestion(self, message: &str, suggestion: &str) -> McpError {
        self.to_mcp_error(
            message,
            Some(serde_json::json!({
                "suggestion": suggestion
            })),
        )
    }
}

#[macro_export]
macro_rules! mcp_err {
    ($code:expr, $msg:expr) => {
        $code.to_mcp_error($msg, None)
    };
    ($code:expr, $msg:expr, $ctx:tt) => {
        $code.to_mcp_error($msg, Some(serde_json::json!($ctx)))
    };
}

pub use mcp_err;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_serialization() {
        let code = ErrorCode::AgentNotFound;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "\"AGENT_NOT_FOUND\"");
    }

    #[test]
    fn test_to_mcp_error_includes_error_code() {
        let err = ErrorCode::AgentNotFound.to_mcp_error("Agent 'test' not found", None);

        let data = err.data.expect("should have data");
        let error_code = data.get("error_code").expect("should have error_code");
        assert_eq!(error_code, "AGENT_NOT_FOUND");
    }

    #[test]
    fn test_to_mcp_error_preserves_context() {
        let ctx = serde_json::json!({
            "agent_name": "claude_1",
            "project_id": 42
        });
        let err = ErrorCode::AgentNotFound.to_mcp_error("Agent not found", Some(ctx));

        let data = err.data.expect("should have data");
        assert_eq!(data.get("agent_name").unwrap(), "claude_1");
        assert_eq!(data.get("project_id").unwrap(), 42);
        assert_eq!(data.get("error_code").unwrap(), "AGENT_NOT_FOUND");
    }

    #[test]
    fn test_with_suggestion() {
        let err = ErrorCode::AgentAlreadyExists
            .with_suggestion("Agent 'test' already exists", "Use a different agent name");

        let data = err.data.expect("should have data");
        assert_eq!(data.get("error_code").unwrap(), "AGENT_ALREADY_EXISTS");
        assert_eq!(
            data.get("suggestion").unwrap(),
            "Use a different agent name"
        );
    }

    #[test]
    fn test_mcp_err_macro_simple() {
        let err = mcp_err!(ErrorCode::ProjectNotFound, "Project not found");
        let data = err.data.expect("should have data");
        assert_eq!(data.get("error_code").unwrap(), "PROJECT_NOT_FOUND");
    }

    #[test]
    fn test_mcp_err_macro_with_context() {
        let err = mcp_err!(
            ErrorCode::CapabilityDenied,
            "Capability denied",
            { "capability": "send_message", "agent_name": "test" }
        );

        let data = err.data.expect("should have data");
        assert_eq!(data.get("error_code").unwrap(), "CAPABILITY_DENIED");
        assert_eq!(data.get("capability").unwrap(), "send_message");
        assert_eq!(data.get("agent_name").unwrap(), "test");
    }
}
