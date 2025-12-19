//! Orchestration state machine for review workflows.
//!
//! This module provides state parsing for review threads based on subject prefixes.
//! Used by multi-agent orchestration to track task lifecycle.

use crate::model::message::Message;
use serde::{Deserialize, Serialize};

/// State machine for orchestration workflows.
/// States are parsed from message subject prefixes like `[TASK_STARTED]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrchestrationState {
    /// Task has been started - `[TASK_STARTED]`
    Started,
    /// Task is complete, awaiting review - `[COMPLETION]`
    Completed,
    /// Reviewer is actively reviewing - `[REVIEWING]`
    Reviewing,
    /// Review approved - `[APPROVED]`
    Approved,
    /// Review rejected, needs fixes - `[REJECTED]`
    Rejected,
    /// Fixes applied after rejection - `[FIXED]`
    Fixed,
    /// Message acknowledged - `[ACK]`
    Acknowledged,
}

impl OrchestrationState {
    /// Parse state from message subject prefix.
    /// Returns `None` if no recognized prefix is found.
    pub fn from_subject(subject: &str) -> Option<Self> {
        let subject_upper = subject.to_uppercase();

        if subject_upper.starts_with("[TASK_STARTED]") {
            Some(Self::Started)
        } else if subject_upper.starts_with("[COMPLETION]") {
            Some(Self::Completed)
        } else if subject_upper.starts_with("[REVIEWING]") {
            Some(Self::Reviewing)
        } else if subject_upper.starts_with("[APPROVED]") {
            Some(Self::Approved)
        } else if subject_upper.starts_with("[REJECTED]") {
            Some(Self::Rejected)
        } else if subject_upper.starts_with("[FIXED]") {
            Some(Self::Fixed)
        } else if subject_upper.starts_with("[ACK]") {
            Some(Self::Acknowledged)
        } else {
            None
        }
    }

    /// Check if transitioning to the given state is valid.
    pub fn can_transition_to(&self, next: &Self) -> bool {
        use OrchestrationState::*;

        match (self, next) {
            // From Started: can move to Completed
            (Started, Completed) => true,

            // From Completed: can be Reviewing, Approved, Rejected, or Acknowledged
            (Completed, Reviewing) => true,
            (Completed, Approved) => true,
            (Completed, Rejected) => true,
            (Completed, Acknowledged) => true,

            // From Reviewing: can be Approved or Rejected
            (Reviewing, Approved) => true,
            (Reviewing, Rejected) => true,

            // From Rejected: can be Fixed
            (Rejected, Fixed) => true,

            // From Fixed: goes back to review cycle
            (Fixed, Reviewing) => true,
            (Fixed, Approved) => true,
            (Fixed, Rejected) => true,

            // From Approved: can be Acknowledged
            (Approved, Acknowledged) => true,

            // All other transitions are invalid
            _ => false,
        }
    }

    /// Get the prefix string for this state
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Started => "[TASK_STARTED]",
            Self::Completed => "[COMPLETION]",
            Self::Reviewing => "[REVIEWING]",
            Self::Approved => "[APPROVED]",
            Self::Rejected => "[REJECTED]",
            Self::Fixed => "[FIXED]",
            Self::Acknowledged => "[ACK]",
        }
    }
}

impl std::fmt::Display for OrchestrationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.prefix())
    }
}

// -- ORCH-2: CompletionReport --

/// Quality gate check results
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityGateResults {
    pub tests_passed: Option<bool>,
    pub lint_passed: Option<bool>,
    pub build_passed: Option<bool>,
    pub coverage_met: Option<bool>,
}

impl QualityGateResults {
    pub fn all_passed(&self) -> bool {
        self.tests_passed.unwrap_or(true)
            && self.lint_passed.unwrap_or(true)
            && self.build_passed.unwrap_or(true)
            && self.coverage_met.unwrap_or(true)
    }
}

/// Standardized completion report for [COMPLETION] mails.
/// Used by agents to report task completion in a structured format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionReport {
    /// Beads task ID (e.g., "mcp-agent-mail-rs-abc")
    pub task_id: String,
    /// Human-readable task title
    pub task_title: String,
    /// Git commit SHA of the final commit
    pub commit_id: String,
    /// Branch where the work was done
    pub branch: String,
    /// List of files changed
    pub files_changed: Vec<String>,
    /// Summary of changes made
    pub summary: String,
    /// Acceptance criteria status: (criterion, passed)
    pub criteria_status: Vec<(String, bool)>,
    /// Quality gate results
    pub quality_gates: QualityGateResults,
    /// Optional additional notes
    pub notes: Option<String>,
}

impl CompletionReport {
    /// Create a new blank report with the given task ID
    pub fn new(task_id: impl Into<String>, task_title: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            task_title: task_title.into(),
            commit_id: String::new(),
            branch: String::new(),
            files_changed: vec![],
            summary: String::new(),
            criteria_status: vec![],
            quality_gates: QualityGateResults::default(),
            notes: None,
        }
    }

    /// Generate markdown format suitable for [COMPLETION] messages
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str(&format!("# Completion Report: {}\n\n", self.task_title));
        md.push_str(&format!("**Task ID**: `{}`\n", self.task_id));
        md.push_str(&format!("**Commit**: `{}`\n", self.commit_id));
        md.push_str(&format!("**Branch**: `{}`\n\n", self.branch));

        // Summary
        md.push_str("## Summary\n\n");
        md.push_str(&self.summary);
        md.push_str("\n\n");

        // Files Changed
        md.push_str("## Files Changed\n\n");
        for file in &self.files_changed {
            md.push_str(&format!("- `{}`\n", file));
        }
        md.push_str("\n");

        // Acceptance Criteria
        md.push_str("## Acceptance Criteria\n\n");
        for (criterion, passed) in &self.criteria_status {
            let icon = if *passed { "✅" } else { "❌" };
            md.push_str(&format!("{} {}\n", icon, criterion));
        }
        md.push_str("\n");

        // Quality Gates
        md.push_str("## Quality Gates\n\n");
        let gate_icon = |opt: Option<bool>| match opt {
            Some(true) => "✅",
            Some(false) => "❌",
            None => "⏭️",
        };
        md.push_str(&format!(
            "{} Tests\n",
            gate_icon(self.quality_gates.tests_passed)
        ));
        md.push_str(&format!(
            "{} Lint\n",
            gate_icon(self.quality_gates.lint_passed)
        ));
        md.push_str(&format!(
            "{} Build\n",
            gate_icon(self.quality_gates.build_passed)
        ));
        md.push_str(&format!(
            "{} Coverage\n",
            gate_icon(self.quality_gates.coverage_met)
        ));

        // Notes
        if let Some(notes) = &self.notes {
            md.push_str("\n## Notes\n\n");
            md.push_str(notes);
        }

        md
    }
}

/// Parse the current orchestration state from a thread's message history.
/// Returns the state from the most recent message with a recognized prefix,
/// or `Started` if no state messages are found.
pub fn parse_thread_state(messages: &[Message]) -> OrchestrationState {
    // Iterate in reverse (most recent first)
    for msg in messages.iter().rev() {
        if let Some(state) = OrchestrationState::from_subject(&msg.subject) {
            return state;
        }
    }

    // Default to Started if no state found
    OrchestrationState::Started
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn make_message(subject: &str) -> Message {
        Message {
            id: 1,
            project_id: 1,
            sender_id: 1,
            thread_id: Some("test-thread".to_string()),
            subject: subject.to_string(),
            body_md: "Test body".to_string(),
            importance: "normal".to_string(),
            ack_required: false,
            created_ts: NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
            attachments: vec![],
            sender_name: "test-sender".to_string(),
        }
    }

    #[test]
    fn test_from_subject_parsing() {
        assert_eq!(
            OrchestrationState::from_subject("[TASK_STARTED] Begin work"),
            Some(OrchestrationState::Started)
        );
        assert_eq!(
            OrchestrationState::from_subject("[COMPLETION] Work done"),
            Some(OrchestrationState::Completed)
        );
        assert_eq!(
            OrchestrationState::from_subject("[REVIEWING] Looking at code"),
            Some(OrchestrationState::Reviewing)
        );
        assert_eq!(
            OrchestrationState::from_subject("[APPROVED] LGTM"),
            Some(OrchestrationState::Approved)
        );
        assert_eq!(
            OrchestrationState::from_subject("[REJECTED] Needs fixes"),
            Some(OrchestrationState::Rejected)
        );
        assert_eq!(
            OrchestrationState::from_subject("[FIXED] Applied changes"),
            Some(OrchestrationState::Fixed)
        );
        assert_eq!(
            OrchestrationState::from_subject("[ACK] Understood"),
            Some(OrchestrationState::Acknowledged)
        );
        assert_eq!(OrchestrationState::from_subject("Regular message"), None);
    }

    #[test]
    fn test_from_subject_case_insensitive() {
        assert_eq!(
            OrchestrationState::from_subject("[task_started] Begin"),
            Some(OrchestrationState::Started)
        );
        assert_eq!(
            OrchestrationState::from_subject("[Task_Started] Begin"),
            Some(OrchestrationState::Started)
        );
        assert_eq!(
            OrchestrationState::from_subject("[COMPLETION] Done"),
            Some(OrchestrationState::Completed)
        );
    }

    #[test]
    fn test_valid_transitions() {
        use OrchestrationState::*;

        // Valid paths
        assert!(Started.can_transition_to(&Completed));
        assert!(Completed.can_transition_to(&Reviewing));
        assert!(Completed.can_transition_to(&Approved));
        assert!(Completed.can_transition_to(&Rejected));
        assert!(Reviewing.can_transition_to(&Approved));
        assert!(Reviewing.can_transition_to(&Rejected));
        assert!(Rejected.can_transition_to(&Fixed));
        assert!(Fixed.can_transition_to(&Reviewing));
        assert!(Fixed.can_transition_to(&Approved));
        assert!(Approved.can_transition_to(&Acknowledged));
    }

    #[test]
    fn test_invalid_transitions() {
        use OrchestrationState::*;

        // Invalid paths
        assert!(!Started.can_transition_to(&Approved));
        assert!(!Started.can_transition_to(&Rejected));
        assert!(!Approved.can_transition_to(&Rejected));
        assert!(!Acknowledged.can_transition_to(&Started));
        assert!(!Fixed.can_transition_to(&Started));
    }

    #[test]
    fn test_parse_thread_state_empty() {
        let messages: Vec<Message> = vec![];
        assert_eq!(parse_thread_state(&messages), OrchestrationState::Started);
    }

    #[test]
    fn test_parse_thread_state_with_messages() {
        let messages = vec![
            make_message("[TASK_STARTED] Begin work"),
            make_message("Some discussion"),
            make_message("[COMPLETION] Work done"),
        ];
        // Most recent state-bearing message is [COMPLETION]
        assert_eq!(parse_thread_state(&messages), OrchestrationState::Completed);
    }

    #[test]
    fn test_parse_thread_state_full_cycle() {
        let messages = vec![
            make_message("[TASK_STARTED] Begin"),
            make_message("[COMPLETION] Done"),
            make_message("[REJECTED] Needs fixes"),
            make_message("[FIXED] Applied"),
            make_message("[APPROVED] LGTM"),
        ];
        assert_eq!(parse_thread_state(&messages), OrchestrationState::Approved);
    }

    #[test]
    fn test_prefix() {
        assert_eq!(OrchestrationState::Started.prefix(), "[TASK_STARTED]");
        assert_eq!(OrchestrationState::Completed.prefix(), "[COMPLETION]");
        assert_eq!(OrchestrationState::Approved.prefix(), "[APPROVED]");
    }

    // -- ORCH-2 Tests --

    #[test]
    fn test_quality_gate_results_all_passed() {
        let gates = QualityGateResults {
            tests_passed: Some(true),
            lint_passed: Some(true),
            build_passed: Some(true),
            coverage_met: Some(true),
        };
        assert!(gates.all_passed());
    }

    #[test]
    fn test_quality_gate_results_one_failed() {
        let gates = QualityGateResults {
            tests_passed: Some(true),
            lint_passed: Some(false),
            build_passed: Some(true),
            coverage_met: Some(true),
        };
        assert!(!gates.all_passed());
    }

    #[test]
    fn test_quality_gate_results_defaults() {
        let gates = QualityGateResults::default();
        assert!(gates.all_passed()); // None defaults to true
    }

    #[test]
    fn test_completion_report_new() {
        let report = CompletionReport::new("task-123", "My Task Title");
        assert_eq!(report.task_id, "task-123");
        assert_eq!(report.task_title, "My Task Title");
        assert!(report.commit_id.is_empty());
    }

    #[test]
    fn test_completion_report_to_markdown() {
        let mut report = CompletionReport::new("mcp-agent-mail-rs-xyz", "Add Feature X");
        report.commit_id = "abc123def".to_string();
        report.branch = "feature/x".to_string();
        report.files_changed = vec!["src/lib.rs".to_string(), "Cargo.toml".to_string()];
        report.summary = "Implemented feature X with tests.".to_string();
        report.criteria_status = vec![
            ("Unit tests pass".to_string(), true),
            ("Docs updated".to_string(), false),
        ];
        report.quality_gates = QualityGateResults {
            tests_passed: Some(true),
            lint_passed: Some(true),
            build_passed: Some(true),
            coverage_met: None,
        };

        let md = report.to_markdown();

        assert!(md.contains("# Completion Report: Add Feature X"));
        assert!(md.contains("**Task ID**: `mcp-agent-mail-rs-xyz`"));
        assert!(md.contains("**Commit**: `abc123def`"));
        assert!(md.contains("**Branch**: `feature/x`"));
        assert!(md.contains("- `src/lib.rs`"));
        assert!(md.contains("Implemented feature X"));
        assert!(md.contains("✅ Unit tests pass"));
        assert!(md.contains("❌ Docs updated"));
        assert!(md.contains("✅ Tests"));
        assert!(md.contains("⏭️ Coverage")); // None case
    }
}
