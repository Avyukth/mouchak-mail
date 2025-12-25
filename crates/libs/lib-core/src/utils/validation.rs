//! Input validation utilities for MCP Agent Mail.
//!
//! This module provides validation functions for common input types:
//!
//! - **Agent names**: Alphanumeric + underscore, 1-64 characters
//! - **Project keys**: Absolute paths or human-readable keys
//! - **File paths**: Must be relative (no leading `/`)
//! - **TTL values**: Between 60 seconds and 7 days
//!
//! All validation functions return actionable error messages with
//! suggestions for fixing invalid input.
//!
//! # Example
//!
//! ```
//! use lib_core::utils::validation::{validate_agent_name, validate_ttl};
//!
//! // Valid agent name
//! assert!(validate_agent_name("claude_1").is_ok());
//!
//! // Invalid agent name with suggestion
//! let err = validate_agent_name("claude-1").unwrap_err();
//! // Error contains suggestion: "claude1"
//!
//! // TTL validation
//! assert!(validate_ttl(3600).is_ok());  // 1 hour - valid
//! assert!(validate_ttl(30).is_err());   // 30 seconds - too short
//! ```

// Allow expect in this module: regex patterns are compile-time verified
#![allow(clippy::expect_used)]

use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;

lazy_static! {
    /// Regex pattern for valid agent names: alphanumeric + underscore, 1-64 chars.
    static ref AGENT_NAME_RE: Regex =
        Regex::new(r"^[a-zA-Z0-9_]{1,64}$").expect("valid regex pattern");
    /// Regex pattern for valid human keys: alphanumeric + underscore + hyphen, 1-64 chars.
    static ref HUMAN_KEY_RE: Regex =
        Regex::new(r"^[a-zA-Z0-9_-]{1,64}$").expect("valid regex pattern");
}

/// Detailed validation failure information.
///
/// Contains context about what failed and how to fix it.
///
/// # Fields
///
/// - `field` - Name of the field that failed validation
/// - `provided` - The value that was provided
/// - `reason` - Human-readable explanation of why validation failed
/// - `suggestion` - Optional corrected value the user could use
/// - `pattern` - Optional regex pattern the input should match
#[derive(Debug, Clone, Serialize)]
pub struct ValidationFailure {
    /// Name of the field that failed validation.
    pub field: String,
    /// The value that was provided.
    pub provided: String,
    /// Human-readable explanation of why validation failed.
    pub reason: String,
    /// Optional corrected value the user could use.
    pub suggestion: Option<String>,
    /// Optional regex pattern the input should match.
    pub pattern: Option<String>,
}

/// Input validation errors with recovery hints.
///
/// Each variant includes the invalid input and a suggestion for correction.
/// This enables agents to self-correct without human intervention.
///
/// # Example
///
/// ```
/// use lib_core::utils::validation::{ValidationError, validate_agent_name};
///
/// match validate_agent_name("invalid-name") {
///     Ok(()) => println!("Valid"),
///     Err(ValidationError::InvalidAgentName { provided, suggestion }) => {
///         println!("Invalid: {}, try: {}", provided, suggestion);
///     }
///     Err(e) => println!("Other error: {}", e),
/// }
/// ```
#[derive(Debug, thiserror::Error, Serialize)]
pub enum ValidationError {
    /// Generic field validation failure.
    #[error("Invalid {field}: {reason}")]
    InvalidField {
        /// Field name that failed validation.
        field: String,
        /// Value that was provided.
        provided: String,
        /// Why validation failed.
        reason: String,
        /// Suggested correction.
        suggestion: Option<String>,
    },

    /// Project key is neither an absolute path nor a valid human_key.
    #[error("Project key must be absolute path or human_key, got: {provided}")]
    InvalidProjectKey {
        /// The invalid project key.
        provided: String,
        /// Suggested correction.
        suggestion: String,
    },

    /// Agent name doesn't match the required pattern.
    #[error("Agent name must match ^[a-zA-Z0-9_]{{1,64}}$, got: {provided}")]
    InvalidAgentName {
        /// The invalid agent name.
        provided: String,
        /// Sanitized version as suggestion.
        suggestion: String,
    },

    /// File path is absolute when it should be relative.
    #[error("File path must be relative (no leading /), got: {provided}")]
    AbsolutePathNotAllowed {
        /// The invalid absolute path.
        provided: String,
        /// Suggested relative path.
        suggestion: String,
    },

    /// TTL value is outside allowed range (60s to 7 days).
    #[error("TTL must be between {min}s and {max}s, got: {provided}s")]
    InvalidTtl {
        /// The invalid TTL value.
        provided: u64,
        /// Minimum allowed TTL (60 seconds).
        min: u64,
        /// Maximum allowed TTL (604800 seconds = 7 days).
        max: u64,
        /// Clamped value as suggestion.
        suggestion: u64,
    },

    /// Entity not found with similar name suggestions.
    #[error("Entity not found: {entity_type} with {identifier}")]
    NotFound {
        /// Type of entity (e.g., "agent", "project").
        entity_type: String,
        /// The identifier that was not found.
        identifier: String,
        /// Similar identifiers that do exist.
        similar: Vec<String>,
    },
}

impl ValidationError {
    /// Converts the error to a JSON value for structured error responses.
    ///
    /// Useful for MCP tool responses that need structured error context.
    ///
    /// # Returns
    ///
    /// A JSON value containing all error fields, or an empty object on
    /// serialization failure.
    pub fn context(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// Validates an agent name against the allowed pattern.
///
/// Agent names must:
/// - Be 1-64 characters long
/// - Contain only alphanumeric characters and underscores
///
/// # Arguments
///
/// * `name` - The agent name to validate
///
/// # Returns
///
/// `Ok(())` if valid, or `Err(ValidationError::InvalidAgentName)` with
/// a sanitized suggestion.
///
/// # Examples
///
/// ```
/// use lib_core::utils::validation::validate_agent_name;
///
/// assert!(validate_agent_name("claude_1").is_ok());
/// assert!(validate_agent_name("AGENT123").is_ok());
/// assert!(validate_agent_name("a").is_ok());
///
/// // Invalid names return suggestions
/// let err = validate_agent_name("claude-1").unwrap_err();
/// ```
pub fn validate_agent_name(name: &str) -> Result<(), ValidationError> {
    if AGENT_NAME_RE.is_match(name) {
        return Ok(());
    }

    Err(ValidationError::InvalidAgentName {
        provided: name.to_string(),
        suggestion: sanitize_agent_name(name),
    })
}

/// Sanitizes an agent name by removing invalid characters.
///
/// This is used to generate suggestions when validation fails.
/// The sanitized name:
/// - Contains only alphanumeric characters and underscores
/// - Is truncated to 64 characters
/// - Is lowercased for consistency
///
/// # Arguments
///
/// * `input` - The input string to sanitize
///
/// # Returns
///
/// A sanitized string suitable for use as an agent name.
///
/// # Examples
///
/// ```
/// use lib_core::utils::validation::sanitize_agent_name;
///
/// assert_eq!(sanitize_agent_name("my-agent!"), "myagent");
/// assert_eq!(sanitize_agent_name("Claude_1"), "claude_1");
/// ```
pub fn sanitize_agent_name(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(64)
        .collect::<String>()
        .to_lowercase()
}

/// Validates a project key.
///
/// Project keys can be either:
/// - **Absolute paths**: Starting with `/` (e.g., `/Users/me/myproject`)
/// - **Human keys**: Alphanumeric with underscores/hyphens (e.g., `my-project`)
///
/// # Arguments
///
/// * `key` - The project key to validate
///
/// # Returns
///
/// `Ok(())` if valid, or `Err(ValidationError::InvalidProjectKey)` with suggestion.
///
/// # Examples
///
/// ```
/// use lib_core::utils::validation::validate_project_key;
///
/// assert!(validate_project_key("/Users/me/project").is_ok());
/// assert!(validate_project_key("my-project").is_ok());
/// assert!(validate_project_key("relative/path").is_err()); // Neither absolute nor human_key
/// ```
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

/// Validates a file reservation path.
///
/// File reservation paths must be relative (not starting with `/`).
/// This prevents reserving files outside the project directory.
///
/// # Arguments
///
/// * `path` - The file path to validate
///
/// # Returns
///
/// `Ok(())` if valid, or `Err(ValidationError::AbsolutePathNotAllowed)` with
/// the path stripped of leading slashes as a suggestion.
///
/// # Examples
///
/// ```
/// use lib_core::utils::validation::validate_reservation_path;
///
/// assert!(validate_reservation_path("src/main.rs").is_ok());
/// assert!(validate_reservation_path("**/*.rs").is_ok());
/// assert!(validate_reservation_path("/src/main.rs").is_err()); // Absolute path
/// ```
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

/// Validates a TTL (Time To Live) value.
///
/// TTL must be between 60 seconds (1 minute) and 604,800 seconds (7 days).
/// This prevents both too-short locks that expire immediately and too-long
/// locks that could block resources indefinitely.
///
/// # Arguments
///
/// * `ttl_seconds` - The TTL value in seconds
///
/// # Returns
///
/// `Ok(())` if valid, or `Err(ValidationError::InvalidTtl)` with a clamped
/// value as suggestion.
///
/// # Examples
///
/// ```
/// use lib_core::utils::validation::validate_ttl;
///
/// assert!(validate_ttl(3600).is_ok());     // 1 hour - valid
/// assert!(validate_ttl(86400).is_ok());    // 1 day - valid
/// assert!(validate_ttl(30).is_err());      // 30 seconds - too short
/// assert!(validate_ttl(1000000).is_err()); // ~11 days - too long
/// ```
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]
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
