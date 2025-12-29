//! Agent Mistake Detection Helpers (PORT-1.3)
//!
//! Proactive detection of common AI agent input mistakes.

use serde::Serialize;
use strsim::levenshtein;

/// Suggestion for detected mistake
#[derive(Debug, Clone, Serialize)]
pub struct MistakeSuggestion {
    pub detected_issue: String,
    pub suggestion: String,
    pub confidence: f64, // 0.0 - 1.0
}

/// Detect if input looks like an absolute path used as project_key
pub fn detect_path_as_project_key(input: &str) -> Option<MistakeSuggestion> {
    if input.contains('/') && !input.starts_with('/') {
        // Relative path used where absolute expected
        return Some(MistakeSuggestion {
            detected_issue: "Relative path provided, expected absolute path or human_key".into(),
            suggestion: format!("Use absolute path: /{}", input),
            confidence: 0.8,
        });
    }
    None
}

/// Detect if agent_name looks like a file path
pub fn detect_path_as_agent_name(input: &str) -> Option<MistakeSuggestion> {
    if input.contains('/') || input.contains('.') {
        let sanitized = input
            .split(&['/', '.'][..])
            .next_back()
            .unwrap_or(input)
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();

        return Some(MistakeSuggestion {
            detected_issue: "Agent name contains path characters".into(),
            suggestion: format!("Use agent name: {}", sanitized),
            confidence: 0.9,
        });
    }
    None
}

/// Detect thread_id vs message_id confusion
#[derive(Debug, Clone, Copy)]
pub enum IdType {
    ThreadId,
    MessageId,
}

pub fn detect_id_confusion(input: &str, expected: IdType) -> Option<MistakeSuggestion> {
    let is_numeric = input.parse::<i64>().is_ok();

    match (expected, is_numeric) {
        (IdType::ThreadId, true) => Some(MistakeSuggestion {
            detected_issue: "Numeric ID provided where thread_id expected".into(),
            suggestion:
                "thread_id is a user-defined string (e.g., 'FEAT-123'), not a numeric message_id"
                    .into(),
            confidence: 0.7,
        }),
        (IdType::MessageId, false) => Some(MistakeSuggestion {
            detected_issue: "Non-numeric value provided where message_id expected".into(),
            suggestion: "message_id must be numeric (e.g., 42)".into(),
            confidence: 0.7,
        }),
        _ => None,
    }
}

/// Find similar strings using Levenshtein distance
pub fn suggest_similar<'a>(
    input: &str,
    candidates: &'a [&str],
    max_distance: usize,
) -> Vec<&'a str> {
    let mut matches: Vec<_> = candidates
        .iter()
        .map(|c| (*c, levenshtein(input, c)))
        .filter(|(_, d)| *d <= max_distance)
        .collect();

    matches.sort_by_key(|(_, d)| *d);
    matches.into_iter().map(|(c, _)| c).take(3).collect()
}

pub fn looks_like_unix_username(value: &str) -> bool {
    let v = value.trim();
    if v.is_empty() {
        return false;
    }
    let is_lowercase_alnum = v
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let len_valid = (2..=16).contains(&v.len());
    is_lowercase_alnum && len_valid
}

/// Detect if an agent name looks like a Unix username ($USER).
///
/// This is a **hint**, not an error. Lowercase alphanumeric names are valid,
/// but often indicate the user passed their shell $USER instead of registering
/// an agent identity first.
///
/// Returns a suggestion if the input matches common Unix username patterns.
pub fn detect_unix_username_as_agent(input: &str) -> Option<MistakeSuggestion> {
    if looks_like_unix_username(input) {
        Some(MistakeSuggestion {
            detected_issue: format!("'{}' looks like a Unix username (possibly $USER)", input),
            suggestion: format!(
                "If '{}' is your shell username, consider using a descriptive agent name instead. \
                 Use create_agent_identity for name suggestions, or register_agent with a name like 'Claude_Backend'.",
                input
            ),
            confidence: 0.75, // Lower confidence - valid names can match this pattern
        })
    } else {
        None
    }
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
    fn test_detect_path_as_agent_name() {
        let result = detect_path_as_agent_name("src/main.rs");
        assert!(result.is_some());
        assert!(result.unwrap().suggestion.contains("rs"));
    }

    #[test]
    fn test_similar_suggestions() {
        let candidates = &["claude_1", "claude_2", "gemini_1"];
        let similar = suggest_similar("claued_1", candidates, 3);
        assert_eq!(similar.first(), Some(&"claude_1"));
    }

    #[test]
    fn test_detect_id_confusion_thread() {
        let result = detect_id_confusion("12345", IdType::ThreadId);
        assert!(result.is_some());
        assert!(result.unwrap().detected_issue.contains("Numeric"));
    }

    #[test]
    fn test_detect_id_confusion_message() {
        let result = detect_id_confusion("FEAT-123", IdType::MessageId);
        assert!(result.is_some());
        assert!(result.unwrap().detected_issue.contains("Non-numeric"));
    }

    #[test]
    fn test_looks_like_unix_username_valid() {
        assert!(looks_like_unix_username("ubuntu"));
        assert!(looks_like_unix_username("amrit"));
        assert!(looks_like_unix_username("user1"));
        assert!(looks_like_unix_username("root"));
        assert!(looks_like_unix_username("ec2user"));
    }

    #[test]
    fn test_looks_like_unix_username_invalid() {
        assert!(!looks_like_unix_username("BlueLake"));
        assert!(!looks_like_unix_username("UPPERCASE"));
        assert!(!looks_like_unix_username("a"));
        assert!(!looks_like_unix_username(""));
        assert!(!looks_like_unix_username("has-dash"));
        assert!(!looks_like_unix_username("has_underscore"));
    }

    #[test]
    fn test_looks_like_unix_username_boundaries() {
        assert!(looks_like_unix_username("ab"));
        assert!(looks_like_unix_username("abcdefghijklmnop"));
        assert!(!looks_like_unix_username("abcdefghijklmnopq"));
        assert!(!looks_like_unix_username("a"));
        assert!(looks_like_unix_username("u1"));
        assert!(looks_like_unix_username("user123456789012"));
    }

    #[test]
    fn test_detect_unix_username_as_agent() {
        let result = detect_unix_username_as_agent("ubuntu");
        assert!(result.is_some());
        let suggestion = result.unwrap();
        assert!(suggestion.suggestion.contains("create_agent_identity"));
        assert!(suggestion.detected_issue.contains("Unix username"));

        let result = detect_unix_username_as_agent("BlueLake");
        assert!(result.is_none());
    }
}
