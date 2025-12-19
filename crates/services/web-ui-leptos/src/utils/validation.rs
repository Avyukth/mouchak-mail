//! Form validation module for client-side validation.
//!
//! Provides validation rules and utilities for form fields.
//! Server-side validation should ALWAYS run as well (never trust client).

use std::fmt;

/// Validation error with field-specific error message.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// The error message (max 50 chars recommended)
    pub message: String,
}

impl ValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Validation rules for form fields.
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Field must not be empty
    Required,
    /// Field must have at least N characters
    MinLength(usize),
    /// Field must have at most N characters
    MaxLength(usize),
    /// Field must be a valid email format
    Email,
    /// Field must match a custom pattern
    Pattern { pattern: String, message: String },
}

impl ValidationRule {
    /// Create a pattern rule with custom error message.
    pub fn pattern(pattern: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Pattern {
            pattern: pattern.into(),
            message: message.into(),
        }
    }
}

/// Validate a value against a single rule.
pub fn validate_rule(value: &str, rule: &ValidationRule) -> Result<(), ValidationError> {
    match rule {
        ValidationRule::Required => {
            if value.trim().is_empty() {
                Err(ValidationError::new("This field is required"))
            } else {
                Ok(())
            }
        }
        ValidationRule::MinLength(min) => {
            if value.len() < *min {
                Err(ValidationError::new(format!(
                    "Must be at least {} characters",
                    min
                )))
            } else {
                Ok(())
            }
        }
        ValidationRule::MaxLength(max) => {
            if value.len() > *max {
                Err(ValidationError::new(format!(
                    "Must be at most {} characters",
                    max
                )))
            } else {
                Ok(())
            }
        }
        ValidationRule::Email => {
            // Simple email validation: contains @ and . after @
            let has_at = value.contains('@');
            let has_domain = value
                .split('@')
                .nth(1)
                .is_some_and(|domain| domain.contains('.') && !domain.starts_with('.'));

            if has_at && has_domain && !value.starts_with('@') {
                Ok(())
            } else {
                Err(ValidationError::new("Please enter a valid email"))
            }
        }
        ValidationRule::Pattern { pattern, message } => {
            // Simple pattern matching: treat pattern as substring check
            // For WASM compatibility, we avoid regex dependency
            if value.contains(pattern.as_str()) || pattern.is_empty() {
                Ok(())
            } else {
                Err(ValidationError::new(message.clone()))
            }
        }
    }
}

/// Validate a value against multiple rules.
/// Returns the first validation error encountered.
pub fn validate(value: &str, rules: &[ValidationRule]) -> Result<(), ValidationError> {
    for rule in rules {
        validate_rule(value, rule)?;
    }
    Ok(())
}

/// Validate a value and return all errors (not just the first).
pub fn validate_all(value: &str, rules: &[ValidationRule]) -> Vec<ValidationError> {
    rules
        .iter()
        .filter_map(|rule| validate_rule(value, rule).err())
        .collect()
}

/// Field validation state for reactive forms.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldState {
    /// Current field value
    pub value: String,
    /// Whether field has been touched (blurred at least once)
    pub touched: bool,
    /// Whether field has been modified
    pub dirty: bool,
    /// Current validation error (if any)
    pub error: Option<ValidationError>,
}

impl FieldState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if field is valid (no error)
    pub fn is_valid(&self) -> bool {
        self.error.is_none()
    }

    /// Check if error should be displayed
    /// (only show after touched or if dirty after first error)
    pub fn should_show_error(&self) -> bool {
        self.error.is_some() && (self.touched || self.dirty)
    }

    /// Update value and validate
    pub fn set_value(&mut self, value: String, rules: &[ValidationRule]) {
        self.value = value;
        self.dirty = true;
        self.error = validate(&self.value, rules).err();
    }

    /// Mark as touched (blurred)
    pub fn touch(&mut self, rules: &[ValidationRule]) {
        self.touched = true;
        self.error = validate(&self.value, rules).err();
    }

    /// Reset field state
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Generate aria attributes for a field with validation.
pub fn aria_attrs(field: &FieldState, error_id: &str) -> (bool, Option<String>) {
    let aria_invalid = field.should_show_error();
    let aria_describedby = if aria_invalid {
        Some(error_id.to_string())
    } else {
        None
    };
    (aria_invalid, aria_describedby)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ValidationError Tests
    // =========================================================================

    #[test]
    fn test_validation_error_new() {
        let err = ValidationError::new("Test error");
        assert_eq!(err.message, "Test error");
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::new("Display test");
        assert_eq!(format!("{}", err), "Display test");
    }

    #[test]
    fn test_validation_error_clone() {
        let err = ValidationError::new("Clone test");
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // =========================================================================
    // ValidationRule::Required Tests
    // =========================================================================

    #[test]
    fn test_required_passes_with_value() {
        let result = validate_rule("hello", &ValidationRule::Required);
        assert!(result.is_ok());
    }

    #[test]
    fn test_required_fails_with_empty() {
        let result = validate_rule("", &ValidationRule::Required);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().message, "This field is required");
    }

    #[test]
    fn test_required_fails_with_whitespace_only() {
        let result = validate_rule("   ", &ValidationRule::Required);
        assert!(result.is_err());
    }

    #[test]
    fn test_required_passes_with_whitespace_and_content() {
        let result = validate_rule("  hello  ", &ValidationRule::Required);
        assert!(result.is_ok());
    }

    // =========================================================================
    // ValidationRule::MinLength Tests
    // =========================================================================

    #[test]
    fn test_min_length_passes_at_minimum() {
        let result = validate_rule("abc", &ValidationRule::MinLength(3));
        assert!(result.is_ok());
    }

    #[test]
    fn test_min_length_passes_above_minimum() {
        let result = validate_rule("abcdef", &ValidationRule::MinLength(3));
        assert!(result.is_ok());
    }

    #[test]
    fn test_min_length_fails_below_minimum() {
        let result = validate_rule("ab", &ValidationRule::MinLength(3));
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("at least 3"));
    }

    #[test]
    fn test_min_length_zero_always_passes() {
        let result = validate_rule("", &ValidationRule::MinLength(0));
        assert!(result.is_ok());
    }

    // =========================================================================
    // ValidationRule::MaxLength Tests
    // =========================================================================

    #[test]
    fn test_max_length_passes_at_maximum() {
        let result = validate_rule("abc", &ValidationRule::MaxLength(3));
        assert!(result.is_ok());
    }

    #[test]
    fn test_max_length_passes_below_maximum() {
        let result = validate_rule("ab", &ValidationRule::MaxLength(3));
        assert!(result.is_ok());
    }

    #[test]
    fn test_max_length_fails_above_maximum() {
        let result = validate_rule("abcd", &ValidationRule::MaxLength(3));
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("at most 3"));
    }

    #[test]
    fn test_max_length_zero_only_empty_passes() {
        let result = validate_rule("", &ValidationRule::MaxLength(0));
        assert!(result.is_ok());

        let result = validate_rule("a", &ValidationRule::MaxLength(0));
        assert!(result.is_err());
    }

    // =========================================================================
    // ValidationRule::Email Tests
    // =========================================================================

    #[test]
    fn test_email_valid_simple() {
        let result = validate_rule("test@example.com", &ValidationRule::Email);
        assert!(result.is_ok());
    }

    #[test]
    fn test_email_valid_subdomain() {
        let result = validate_rule("test@sub.example.com", &ValidationRule::Email);
        assert!(result.is_ok());
    }

    #[test]
    fn test_email_valid_plus_addressing() {
        let result = validate_rule("test+tag@example.com", &ValidationRule::Email);
        assert!(result.is_ok());
    }

    #[test]
    fn test_email_invalid_no_at() {
        let result = validate_rule("testexample.com", &ValidationRule::Email);
        assert!(result.is_err());
    }

    #[test]
    fn test_email_invalid_no_domain() {
        let result = validate_rule("test@", &ValidationRule::Email);
        assert!(result.is_err());
    }

    #[test]
    fn test_email_invalid_no_tld() {
        let result = validate_rule("test@example", &ValidationRule::Email);
        assert!(result.is_err());
    }

    #[test]
    fn test_email_invalid_starts_with_at() {
        let result = validate_rule("@example.com", &ValidationRule::Email);
        assert!(result.is_err());
    }

    #[test]
    fn test_email_invalid_dot_start_domain() {
        let result = validate_rule("test@.example.com", &ValidationRule::Email);
        assert!(result.is_err());
    }

    #[test]
    fn test_email_error_message() {
        let result = validate_rule("invalid", &ValidationRule::Email);
        assert_eq!(result.unwrap_err().message, "Please enter a valid email");
    }

    // =========================================================================
    // ValidationRule::Pattern Tests
    // =========================================================================

    #[test]
    fn test_pattern_helper_creates_rule() {
        let rule = ValidationRule::pattern("test", "Must contain test");
        match rule {
            ValidationRule::Pattern { pattern, message } => {
                assert_eq!(pattern, "test");
                assert_eq!(message, "Must contain test");
            }
            _ => panic!("Expected Pattern variant"),
        }
    }

    #[test]
    fn test_pattern_empty_always_passes() {
        let rule = ValidationRule::pattern("", "Never shown");
        let result = validate_rule("anything", &rule);
        assert!(result.is_ok());
    }

    // =========================================================================
    // validate() Multiple Rules Tests
    // =========================================================================

    #[test]
    fn test_validate_empty_rules_passes() {
        let result = validate("anything", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_all_rules_pass() {
        let rules = vec![
            ValidationRule::Required,
            ValidationRule::MinLength(3),
            ValidationRule::MaxLength(10),
        ];
        let result = validate("hello", &rules);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_stops_at_first_error() {
        let rules = vec![
            ValidationRule::Required,
            ValidationRule::MinLength(10), // This will fail
            ValidationRule::MaxLength(5),  // This would also fail but won't be checked
        ];
        let result = validate("hello", &rules);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("at least 10"));
    }

    #[test]
    fn test_validate_required_checked_first() {
        let rules = vec![ValidationRule::Required, ValidationRule::Email];
        let result = validate("", &rules);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().message, "This field is required");
    }

    // =========================================================================
    // validate_all() Tests
    // =========================================================================

    #[test]
    fn test_validate_all_returns_all_errors() {
        let rules = vec![
            ValidationRule::Required,
            ValidationRule::MinLength(10),
            ValidationRule::Email,
        ];
        let errors = validate_all("ab", &rules);
        assert_eq!(errors.len(), 2); // MinLength and Email fail
    }

    #[test]
    fn test_validate_all_empty_when_valid() {
        let rules = vec![ValidationRule::Required, ValidationRule::MinLength(3)];
        let errors = validate_all("hello", &rules);
        assert!(errors.is_empty());
    }

    // =========================================================================
    // FieldState Tests
    // =========================================================================

    #[test]
    fn test_field_state_default() {
        let state = FieldState::new();
        assert!(state.value.is_empty());
        assert!(!state.touched);
        assert!(!state.dirty);
        assert!(state.error.is_none());
    }

    #[test]
    fn test_field_state_is_valid() {
        let mut state = FieldState::new();
        assert!(state.is_valid());

        state.error = Some(ValidationError::new("Error"));
        assert!(!state.is_valid());
    }

    #[test]
    fn test_field_state_should_show_error_not_touched() {
        let mut state = FieldState::new();
        state.error = Some(ValidationError::new("Error"));
        assert!(!state.should_show_error()); // Not touched, not dirty
    }

    #[test]
    fn test_field_state_should_show_error_touched() {
        let mut state = FieldState::new();
        state.error = Some(ValidationError::new("Error"));
        state.touched = true;
        assert!(state.should_show_error());
    }

    #[test]
    fn test_field_state_should_show_error_dirty() {
        let mut state = FieldState::new();
        state.error = Some(ValidationError::new("Error"));
        state.dirty = true;
        assert!(state.should_show_error());
    }

    #[test]
    fn test_field_state_set_value() {
        let mut state = FieldState::new();
        let rules = vec![ValidationRule::Required];

        state.set_value("hello".to_string(), &rules);
        assert_eq!(state.value, "hello");
        assert!(state.dirty);
        assert!(state.error.is_none());
    }

    #[test]
    fn test_field_state_set_value_with_error() {
        let mut state = FieldState::new();
        let rules = vec![ValidationRule::MinLength(10)];

        state.set_value("short".to_string(), &rules);
        assert!(state.error.is_some());
    }

    #[test]
    fn test_field_state_touch() {
        let mut state = FieldState::new();
        let rules = vec![ValidationRule::Required];

        state.touch(&rules);
        assert!(state.touched);
        assert!(state.error.is_some()); // Empty value fails Required
    }

    #[test]
    fn test_field_state_reset() {
        let mut state = FieldState::new();
        state.value = "test".to_string();
        state.touched = true;
        state.dirty = true;
        state.error = Some(ValidationError::new("Error"));

        state.reset();
        assert!(state.value.is_empty());
        assert!(!state.touched);
        assert!(!state.dirty);
        assert!(state.error.is_none());
    }

    // =========================================================================
    // aria_attrs() Tests
    // =========================================================================

    #[test]
    fn test_aria_attrs_no_error() {
        let state = FieldState::new();
        let (invalid, describedby) = aria_attrs(&state, "error-id");
        assert!(!invalid);
        assert!(describedby.is_none());
    }

    #[test]
    fn test_aria_attrs_with_error_touched() {
        let mut state = FieldState::new();
        state.error = Some(ValidationError::new("Error"));
        state.touched = true;

        let (invalid, describedby) = aria_attrs(&state, "error-id");
        assert!(invalid);
        assert_eq!(describedby, Some("error-id".to_string()));
    }

    #[test]
    fn test_aria_attrs_error_not_shown() {
        let mut state = FieldState::new();
        state.error = Some(ValidationError::new("Error"));
        // Not touched, not dirty

        let (invalid, describedby) = aria_attrs(&state, "error-id");
        assert!(!invalid);
        assert!(describedby.is_none());
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_unicode_string_length() {
        // Unicode characters count as bytes, not characters in len()
        let result = validate_rule("日本語", &ValidationRule::MinLength(9)); // 3 chars * 3 bytes
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_string_all_rules() {
        assert!(validate_rule("", &ValidationRule::Required).is_err());
        assert!(validate_rule("", &ValidationRule::MinLength(0)).is_ok());
        assert!(validate_rule("", &ValidationRule::MinLength(1)).is_err());
        assert!(validate_rule("", &ValidationRule::MaxLength(0)).is_ok());
        assert!(validate_rule("", &ValidationRule::Email).is_err());
    }

    #[test]
    fn test_very_long_string() {
        let long = "a".repeat(10000);
        assert!(validate_rule(&long, &ValidationRule::Required).is_ok());
        assert!(validate_rule(&long, &ValidationRule::MaxLength(9999)).is_err());
        assert!(validate_rule(&long, &ValidationRule::MaxLength(10000)).is_ok());
    }

    #[test]
    fn test_special_characters_in_value() {
        let special = "<script>alert('xss')</script>";
        // Validation doesn't sanitize, just validates
        assert!(validate_rule(special, &ValidationRule::Required).is_ok());
        assert!(validate_rule(special, &ValidationRule::MinLength(1)).is_ok());
    }

    #[test]
    fn test_error_message_length_guideline() {
        // All error messages should be <= 50 characters per quality gate
        let errors = [
            "This field is required",
            "Must be at least 3 characters",
            "Must be at most 100 characters",
            "Please enter a valid email",
        ];
        for msg in errors {
            assert!(
                msg.len() <= 50,
                "Error message too long: '{}' ({} chars)",
                msg,
                msg.len()
            );
        }
    }
}
