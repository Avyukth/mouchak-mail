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

#[cfg(test)]
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
}
