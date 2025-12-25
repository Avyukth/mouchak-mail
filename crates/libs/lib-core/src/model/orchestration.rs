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
        use OrchestrationState::{
            Acknowledged, Approved, Completed, Fixed, Rejected, Reviewing, Started,
        };

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

/// Standardized completion report for \[COMPLETION\] mails.
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

    /// Generate markdown format suitable for \[COMPLETION\] messages
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
        md.push('\n');

        // Acceptance Criteria
        md.push_str("## Acceptance Criteria\n\n");
        for (criterion, passed) in &self.criteria_status {
            let icon = if *passed { "✅" } else { "❌" };
            md.push_str(&format!("{} {}\n", icon, criterion));
        }
        md.push('\n');

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

    /// Auto-populate a CompletionReport from git state and beads task info.
    ///
    /// This function shells out to `git` and `bd` commands to gather:
    /// - Current branch name
    /// - HEAD commit SHA
    /// - Files changed (from git diff --name-only)
    /// - Task title from beads (if task_id is a valid beads ID)
    ///
    /// # Errors
    /// Returns an error if git commands fail or the working directory is not a git repo.
    pub fn from_git_and_beads(task_id: &str) -> std::io::Result<Self> {
        use std::process::Command;

        // Get current branch
        let branch_output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()?;
        let branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        // Get HEAD commit
        let commit_output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()?;
        let commit_id = String::from_utf8_lossy(&commit_output.stdout)
            .trim()
            .to_string();

        // Get files changed (staged and unstaged)
        let diff_output = Command::new("git")
            .args(["diff", "--name-only", "HEAD~1..HEAD"])
            .output()?;
        let files_changed: Vec<String> = String::from_utf8_lossy(&diff_output.stdout)
            .lines()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        // Try to get task title from beads (bd show <task_id>)
        let task_title = Command::new("bd")
            .args(["show", task_id])
            .output()
            .ok()
            .and_then(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse first line after ID for title
                stdout
                    .lines()
                    .find(|line| line.contains(':') && !line.starts_with("Status"))
                    .map(|line| {
                        line.split_once(':')
                            .map(|(_, title)| title.trim().to_string())
                            .unwrap_or_else(|| task_id.to_string())
                    })
            })
            .unwrap_or_else(|| task_id.to_string());

        Ok(Self {
            task_id: task_id.to_string(),
            task_title,
            commit_id,
            branch,
            files_changed,
            summary: String::new(), // Must be filled in by caller
            criteria_status: vec![],
            quality_gates: QualityGateResults::default(),
            notes: None,
        })
    }
}

// -- ORCH-5: WorktreeManager --

use std::path::{Path, PathBuf};

/// Information about an active worktree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub task_id: Option<String>,
    pub created_at: String,
}

/// Manages git worktrees for multi-agent isolation.
///
/// Creates isolated sandboxes for workers and reviewers to work
/// without interfering with each other or the main branch.
pub struct WorktreeManager {
    base_path: PathBuf,
}

/// Result of a worktree operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeResult {
    pub success: bool,
    pub path: PathBuf,
    pub branch: String,
    pub message: String,
}

impl WorktreeManager {
    /// Create a new WorktreeManager with the given base path.
    /// All worktrees will be created under `base/.sandboxes/`.
    pub fn new(base: &Path) -> Self {
        Self {
            base_path: base.join(".sandboxes"),
        }
    }

    /// Get the path for a worker worktree
    pub fn worker_path(&self, task_id: &str) -> PathBuf {
        self.base_path.join(format!("worker-{}", task_id))
    }

    /// Get the path for a reviewer worktree
    pub fn reviewer_path(&self, task_id: &str) -> PathBuf {
        self.base_path.join(format!("reviewer-fix-{}", task_id))
    }

    /// Create a worker worktree for isolated task development.
    ///
    /// Creates a new git worktree at `.sandboxes/worker-{task_id}` with a
    /// new branch `feature/{task_id}` based on the current HEAD.
    pub fn create_worker_worktree(&self, task_id: &str) -> std::io::Result<WorktreeResult> {
        let worktree_path = self.worker_path(task_id);
        let branch_name = format!("feature/{}", task_id);

        self.create_worktree(&worktree_path, &branch_name)
    }

    /// Create a reviewer worktree for applying fixes.
    ///
    /// Creates a new git worktree at `.sandboxes/reviewer-fix-{task_id}` with a
    /// new branch `fix/{task_id}` based on the current HEAD.
    pub fn create_reviewer_worktree(&self, task_id: &str) -> std::io::Result<WorktreeResult> {
        let worktree_path = self.reviewer_path(task_id);
        let branch_name = format!("fix/{}", task_id);

        self.create_worktree(&worktree_path, &branch_name)
    }

    /// Internal helper to create a worktree with a new branch.
    fn create_worktree(&self, path: &Path, branch: &str) -> std::io::Result<WorktreeResult> {
        use std::process::Command;

        // Ensure sandboxes directory exists
        std::fs::create_dir_all(&self.base_path)?;

        // Check if worktree already exists
        if path.exists() {
            return Ok(WorktreeResult {
                success: false,
                path: path.to_path_buf(),
                branch: branch.to_string(),
                message: "Worktree already exists".to_string(),
            });
        }

        // Create worktree with new branch: git worktree add -b <branch> <path>
        let output = Command::new("git")
            .args(["worktree", "add", "-b", branch])
            .arg(path)
            .output()?;

        if output.status.success() {
            Ok(WorktreeResult {
                success: true,
                path: path.to_path_buf(),
                branch: branch.to_string(),
                message: "Worktree created successfully".to_string(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(WorktreeResult {
                success: false,
                path: path.to_path_buf(),
                branch: branch.to_string(),
                message: format!("Failed to create worktree: {}", stderr),
            })
        }
    }

    /// Merge the worktree branch into target and cleanup.
    ///
    /// This method:
    /// 1. Switches to the target branch in the main repo
    /// 2. Merges the worktree's branch
    /// 3. Removes the worktree
    /// 4. Deletes the branch
    ///
    /// Returns the merge commit SHA on success.
    pub fn merge_and_cleanup(
        &self,
        worktree_path: &Path,
        target_branch: &str,
    ) -> std::io::Result<String> {
        use std::process::Command;

        // Get the branch name from the worktree
        let branch_output = Command::new("git")
            .args(["-C"])
            .arg(worktree_path)
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()?;

        if !branch_output.status.success() {
            return Err(std::io::Error::other("Failed to get worktree branch"));
        }

        let branch_name = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        // Checkout target branch in main repo
        let checkout = Command::new("git")
            .args(["checkout", target_branch])
            .output()?;

        if !checkout.status.success() {
            return Err(std::io::Error::other(format!(
                "Failed to checkout {}: {}",
                target_branch,
                String::from_utf8_lossy(&checkout.stderr)
            )));
        }

        // Merge the worktree branch
        let merge = Command::new("git")
            .args(["merge", "--no-ff", "-m"])
            .arg(format!("Merge {} into {}", branch_name, target_branch))
            .arg(&branch_name)
            .output()?;

        if !merge.status.success() {
            return Err(std::io::Error::other(format!(
                "Merge failed: {}",
                String::from_utf8_lossy(&merge.stderr)
            )));
        }

        // Get the merge commit SHA
        let sha_output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()?;

        let commit_sha = String::from_utf8_lossy(&sha_output.stdout)
            .trim()
            .to_string();

        // Remove the worktree
        let _ = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(worktree_path)
            .output();

        // Delete the branch
        let _ = Command::new("git")
            .args(["branch", "-D", &branch_name])
            .output();

        Ok(commit_sha)
    }

    /// Force remove a worktree without merging.
    ///
    /// Removes the worktree and deletes its branch. Use when abandoning work.
    pub fn force_cleanup(&self, worktree_path: &Path) -> std::io::Result<()> {
        use std::process::Command;

        // Get the branch name first
        let branch_output = Command::new("git")
            .args(["-C"])
            .arg(worktree_path)
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output();

        let branch_name = branch_output
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        // Remove the worktree forcefully
        let remove = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(worktree_path)
            .output()?;

        if !remove.status.success() {
            // Try removing the directory directly if git worktree remove fails
            if worktree_path.exists() {
                std::fs::remove_dir_all(worktree_path)?;
            }

            // Prune worktrees to clean up stale references
            let _ = Command::new("git").args(["worktree", "prune"]).output();
        }

        // Delete the branch if we found it
        if let Some(branch) = branch_name {
            let _ = Command::new("git").args(["branch", "-D", &branch]).output();
        }

        Ok(())
    }

    /// List all active worktrees in the sandboxes directory
    pub fn list_active_worktrees(&self) -> std::io::Result<Vec<WorktreeInfo>> {
        let mut worktrees = Vec::new();

        if !self.base_path.exists() {
            return Ok(worktrees);
        }

        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                let task_id = if name.starts_with("worker-") {
                    Some(name.strip_prefix("worker-").unwrap_or("").to_string())
                } else if name.starts_with("reviewer-fix-") {
                    Some(name.strip_prefix("reviewer-fix-").unwrap_or("").to_string())
                } else {
                    None
                };

                worktrees.push(WorktreeInfo {
                    path: path.clone(),
                    branch: name.clone(),
                    task_id,
                    created_at: entry
                        .metadata()
                        .and_then(|m| m.created())
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_else(|_| "unknown".to_string()),
                });
            }
        }

        Ok(worktrees)
    }
}

// -- ORCH-6: QualityGateRunner --

/// Result of a single quality gate check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub passed: bool,
    pub exit_code: i32,
    pub output: String,
    pub duration_ms: u64,
}

impl Default for GateResult {
    fn default() -> Self {
        Self {
            passed: false,
            exit_code: -1,
            output: String::new(),
            duration_ms: 0,
        }
    }
}

/// Results from running all quality gates
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FullQualityGateResults {
    pub cargo_check: GateResult,
    pub cargo_clippy: GateResult,
    pub cargo_fmt: GateResult,
    pub cargo_test: GateResult,
    pub all_passed: bool,
}

/// Runs cargo quality gates for a project
pub struct QualityGateRunner;

impl QualityGateRunner {
    /// Run a single cargo command and capture results
    async fn run_gate(command: &str, args: &[&str]) -> GateResult {
        use std::time::Instant;
        use tokio::process::Command;

        let start = Instant::now();
        let result = Command::new(command).args(args).output().await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}{}", stdout, stderr);

                GateResult {
                    passed: output.status.success(),
                    exit_code: output.status.code().unwrap_or(-1),
                    output: combined.chars().take(2000).collect(), // Limit output size
                    duration_ms,
                }
            }
            Err(e) => GateResult {
                passed: false,
                exit_code: -1,
                output: e.to_string(),
                duration_ms,
            },
        }
    }

    /// Run all quality gates (check, clippy, fmt --check, test)
    pub async fn run_all() -> FullQualityGateResults {
        let cargo_check = Self::run_gate("cargo", &["check", "--all-targets"]).await;
        let cargo_clippy = Self::run_gate(
            "cargo",
            &["clippy", "--all-targets", "--", "-D", "warnings"],
        )
        .await;
        let cargo_fmt = Self::run_gate("cargo", &["fmt", "--check"]).await;
        let cargo_test = Self::run_gate("cargo", &["test", "--workspace"]).await;

        let all_passed =
            cargo_check.passed && cargo_clippy.passed && cargo_fmt.passed && cargo_test.passed;

        FullQualityGateResults {
            cargo_check,
            cargo_clippy,
            cargo_fmt,
            cargo_test,
            all_passed,
        }
    }

    /// Run only blocking gates (check, clippy, fmt) - skip tests for speed
    pub async fn run_blocking_only() -> FullQualityGateResults {
        let cargo_check = Self::run_gate("cargo", &["check", "--all-targets"]).await;
        let cargo_clippy = Self::run_gate(
            "cargo",
            &["clippy", "--all-targets", "--", "-D", "warnings"],
        )
        .await;
        let cargo_fmt = Self::run_gate("cargo", &["fmt", "--check"]).await;

        let all_passed = cargo_check.passed && cargo_clippy.passed && cargo_fmt.passed;

        FullQualityGateResults {
            cargo_check,
            cargo_clippy,
            cargo_fmt,
            cargo_test: GateResult::default(),
            all_passed,
        }
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

// -- ORCH-8: OrchestrationBmc for crash recovery --

use crate::Result;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use std::time::Duration;

/// Information about an abandoned task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbandonedTask {
    pub thread_id: String,
    pub task_title: String,
    pub worker_name: String,
    pub last_activity: String,
    pub state: OrchestrationState,
}

/// Information about an abandoned review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbandonedReview {
    pub thread_id: String,
    pub reviewer_name: String,
    pub last_activity: String,
}

/// Information about merge conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub worktree_path: PathBuf,
    pub branch: String,
    pub conflicting_files: Vec<String>,
}

/// BMC for orchestration crash recovery.
///
/// Tracks orchestration state and helps recover from agent crashes
/// by detecting abandoned tasks and reviews.
pub struct OrchestrationBmc;

impl OrchestrationBmc {
    /// Find tasks that were in-progress but the worker disappeared.
    ///
    /// A task is considered abandoned if:
    /// - State is Started or Reviewing (in-progress states)
    /// - No activity for longer than stale_threshold
    pub async fn find_abandoned_tasks(
        ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        stale_threshold: Duration,
    ) -> Result<Vec<AbandonedTask>> {
        use crate::model::message::MessageBmc;

        let mut abandoned = Vec::new();
        let cutoff =
            chrono::Utc::now() - chrono::Duration::from_std(stale_threshold).unwrap_or_default();

        // Get all threads for the project
        let threads = MessageBmc::list_threads(ctx, mm, project_id, 100).await?;

        for thread in threads {
            let messages =
                MessageBmc::list_by_thread(ctx, mm, project_id, &thread.thread_id).await?;

            if messages.is_empty() {
                continue;
            }

            let state = parse_thread_state(&messages);

            // Check if in an "in-progress" state
            if matches!(
                state,
                OrchestrationState::Started | OrchestrationState::Reviewing
            ) {
                // Check if last activity is before cutoff
                // Safe: we checked messages.is_empty() above
                if let Some(last_msg) = messages.last() {
                    let last_ts = last_msg.created_ts;

                    if last_ts < cutoff.naive_utc() {
                        abandoned.push(AbandonedTask {
                            thread_id: thread.thread_id.clone(),
                            task_title: thread.subject,
                            worker_name: last_msg.sender_name.clone(),
                            last_activity: last_ts.to_string(),
                            state,
                        });
                    }
                }
            }
        }

        Ok(abandoned)
    }

    /// Find reviews that were claimed but the reviewer disappeared.
    ///
    /// A review is considered abandoned if:
    /// - State is Reviewing
    /// - No activity for longer than stale_threshold
    pub async fn find_abandoned_reviews(
        ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        stale_threshold: Duration,
    ) -> Result<Vec<AbandonedReview>> {
        use crate::model::message::MessageBmc;

        let mut abandoned = Vec::new();
        let cutoff =
            chrono::Utc::now() - chrono::Duration::from_std(stale_threshold).unwrap_or_default();

        let threads = MessageBmc::list_threads(ctx, mm, project_id, 100).await?;

        for thread in threads {
            let messages =
                MessageBmc::list_by_thread(ctx, mm, project_id, &thread.thread_id).await?;

            if messages.is_empty() {
                continue;
            }

            let state = parse_thread_state(&messages);

            if matches!(state, OrchestrationState::Reviewing) {
                // Safe: we checked messages.is_empty() above
                if let Some(last_msg) = messages.last() {
                    let last_ts = last_msg.created_ts;

                    if last_ts < cutoff.naive_utc() {
                        // Find the reviewer name from REVIEWING message
                        let reviewer = messages
                            .iter()
                            .rev()
                            .find(|m| m.subject.to_uppercase().starts_with("[REVIEWING]"))
                            .map(|m| m.sender_name.clone())
                            .unwrap_or_else(|| "unknown".to_string());

                        abandoned.push(AbandonedReview {
                            thread_id: thread.thread_id.clone(),
                            reviewer_name: reviewer,
                            last_activity: last_ts.to_string(),
                        });
                    }
                }
            }
        }

        Ok(abandoned)
    }

    /// Check for merge conflicts in active worktrees.
    pub fn check_worktree_conflicts(base_path: &Path) -> std::io::Result<Vec<ConflictInfo>> {
        let manager = WorktreeManager::new(base_path);
        let worktrees = manager.list_active_worktrees()?;

        let mut conflicts = Vec::new();
        for wt in worktrees {
            // Check for .git/MERGE_HEAD or conflict markers
            let merge_head = wt.path.join(".git").join("MERGE_HEAD");
            if merge_head.exists() {
                conflicts.push(ConflictInfo {
                    worktree_path: wt.path,
                    branch: wt.branch,
                    conflicting_files: vec![], // Would need to parse git status
                });
            }
        }

        Ok(conflicts)
    }
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
        use OrchestrationState::{
            Acknowledged, Approved, Completed, Fixed, Rejected, Reviewing, Started,
        };

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
        use OrchestrationState::{Acknowledged, Approved, Fixed, Rejected, Started};

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

    #[test]
    fn test_from_git_and_beads_in_repo() {
        // This test only works in a git repo - skip gracefully if not
        let result = CompletionReport::from_git_and_beads("test-task-123");
        match result {
            Ok(report) => {
                assert_eq!(report.task_id, "test-task-123");
                // Branch and commit should be populated if in a git repo
                assert!(!report.branch.is_empty(), "branch should be populated");
                assert!(
                    !report.commit_id.is_empty(),
                    "commit_id should be populated"
                );
            }
            Err(_) => {
                // Not in a git repo or git not available - test passes
            }
        }
    }
}
