// Allow expect in this module: regex patterns are compile-time verified
#![allow(clippy::expect_used)]

use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;

lazy_static! {
    static ref AGENT_NAME_RE: Regex =
        Regex::new(r"^[a-zA-Z0-9_]{1,64}$").expect("valid regex pattern");
    static ref HUMAN_KEY_RE: Regex =
        Regex::new(r"^[a-zA-Z0-9_-]{1,64}$").expect("valid regex pattern");
}

/// Validation error with actionable suggestion
#[derive(Debug, Clone, Serialize)]
pub struct ValidationFailure {
    pub field: String,
    pub provided: String,
    pub reason: String,
    pub suggestion: Option<String>,
    pub pattern: Option<String>,
}

/// Input validation errors with recovery hints
#[derive(Debug, thiserror::Error, Serialize)]
pub enum ValidationError {
    #[error("Invalid {field}: {reason}")]
    InvalidField {
        field: String,
        provided: String,
        reason: String,
        suggestion: Option<String>,
    },

    #[error("Project key must be absolute path or human_key, got: {provided}")]
    InvalidProjectKey {
        provided: String,
        suggestion: String,
    },

    #[error("Agent name must match ^[a-zA-Z0-9_]{{1,64}}$, got: {provided}")]
    InvalidAgentName {
        provided: String,
        suggestion: String,
    },

    #[error("File path must be relative (no leading /), got: {provided}")]
    AbsolutePathNotAllowed {
        provided: String,
        suggestion: String,
    },

    #[error("TTL must be between {min}s and {max}s, got: {provided}s")]
    InvalidTtl {
        provided: u64,
        min: u64,
        max: u64,
        suggestion: u64,
    },

    #[error("Entity not found: {entity_type} with {identifier}")]
    NotFound {
        entity_type: String,
        identifier: String,
        similar: Vec<String>,
    },
}

impl ValidationError {
    // Note: ToolError might not be available here depending on dependencies.
    // The plan suggests to_tool_error method returning ToolError.
    // But ToolError is usually in lib-mcp or lib-common.
    // If lib-core depends on lib-common, we might use it.
    // I'll check imports later. For now I keep the struct/enum.

    pub fn context(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// Validate and potentially sanitize agent name
pub fn validate_agent_name(name: &str) -> Result<(), ValidationError> {
    if AGENT_NAME_RE.is_match(name) {
        return Ok(());
    }

    Err(ValidationError::InvalidAgentName {
        provided: name.to_string(),
        suggestion: sanitize_agent_name(name),
    })
}

/// Sanitize agent name for suggestion
pub fn sanitize_agent_name(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(64)
        .collect::<String>()
        .to_lowercase()
}

/// Validate project key (absolute path or human_key)
pub fn validate_project_key(key: &str) -> Result<(), ValidationError> {
    // Check if it's an absolute path
    if key.starts_with('/') {
        if std::path::Path::new(key).exists() {
            return Ok(());
        }
        // Path format but doesn't exist - might be valid
        return Ok(());
    }

    // Check if it's a valid human_key
    if HUMAN_KEY_RE.is_match(key) {
        return Ok(());
    }

    // Invalid - provide suggestion
    let suggestion = if key.contains('/') && !key.starts_with('/') {
        format!("/{}", key) // Suggest making it absolute
    } else {
        sanitize_agent_name(key) // Suggest as human_key
    };

    Err(ValidationError::InvalidProjectKey {
        provided: key.to_string(),
        suggestion,
    })
}

/// Validate file reservation path (must be relative)
pub fn validate_reservation_path(path: &str) -> Result<(), ValidationError> {
    if path.starts_with('/') {
        let suggestion = path.trim_start_matches('/').to_string();
        return Err(ValidationError::AbsolutePathNotAllowed {
            provided: path.to_string(),
            suggestion,
        });
    }
    Ok(())
}

/// Validate TTL within bounds
pub fn validate_ttl(ttl_seconds: u64) -> Result<(), ValidationError> {
    const MIN_TTL: u64 = 60; // 1 minute
    const MAX_TTL: u64 = 604_800; // 7 days

    if (MIN_TTL..=MAX_TTL).contains(&ttl_seconds) {
        return Ok(());
    }

    let suggestion = ttl_seconds.clamp(MIN_TTL, MAX_TTL);

    Err(ValidationError::InvalidTtl {
        provided: ttl_seconds,
        min: MIN_TTL,
        max: MAX_TTL,
        suggestion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_agent_names() {
        assert!(validate_agent_name("claude_1").is_ok());
        assert!(validate_agent_name("AGENT123").is_ok());
        assert!(validate_agent_name("a").is_ok());
    }

    #[test]
    fn test_invalid_agent_names_with_suggestions() {
        let err = validate_agent_name("claude-1").unwrap_err();
        if let ValidationError::InvalidAgentName { suggestion, .. } = err {
            assert_eq!(suggestion, "claude1");
        }

        let err = validate_agent_name("my agent!").unwrap_err();
        if let ValidationError::InvalidAgentName { suggestion, .. } = err {
            assert_eq!(suggestion, "myagent");
        }
    }

    #[test]
    fn test_absolute_path_rejection() {
        let err = validate_reservation_path("/src/lib.rs").unwrap_err();
        if let ValidationError::AbsolutePathNotAllowed { suggestion, .. } = err {
            assert_eq!(suggestion, "src/lib.rs");
        }
    }

    #[test]
    fn test_ttl_clamping() {
        assert!(validate_ttl(3600).is_ok()); // Valid

        let err = validate_ttl(30).unwrap_err(); // Too short
        if let ValidationError::InvalidTtl { suggestion, .. } = err {
            assert_eq!(suggestion, 60);
        }
    }
}
