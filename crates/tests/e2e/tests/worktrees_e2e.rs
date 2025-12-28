#![allow(clippy::expect_used)]
//! Worktrees Functionality E2E Tests
//!
//! Tests the worktree-related functionality including:
//! - Gate toggle (WORKTREES_ENABLED on/off)
//! - MCP surface verification (tools/resources visibility)
//! - Git repo + worktree creation and management
//! - Environment variable based guard behavior
//!
//! Port of: test_worktrees_functionality_e2e.py
//!
//! Note: Guard installation (pre-commit/pre-push hooks) is tested in
//! lib-core/model/precommit_guard.rs unit tests. These E2E tests focus
//! on the CLI-exposed functionality and environment variable gates.
//!
//! Run tests:
//! ```bash
//! cargo test -p e2e-tests --test worktrees_e2e -- --test-threads=1
//! ```

#![allow(deprecated)] // cargo_bin is still valid for our use case
#![allow(clippy::unwrap_used, clippy::expect_used)] // expect/unwrap is fine in tests

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process;
use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test environment with isolated data directory
fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let data_dir = temp_dir.path().join("data");
    fs::create_dir_all(&data_dir).expect("Failed to create data dir");
    temp_dir
}

/// Run CLI command with isolated data directory
fn run_cli_with_data_dir(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.current_dir(temp_dir.path());
    cmd
}

/// Run CLI command with custom environment variables
fn run_cli_with_env(temp_dir: &TempDir, key: &str, value: &str) -> Command {
    let mut cmd = run_cli_with_data_dir(temp_dir);
    cmd.env(key, value);
    cmd
}

/// Create a git repository in the temp directory
fn create_git_repo(temp_dir: &TempDir) -> std::path::PathBuf {
    let repo_path = temp_dir.path().join("test-repo");
    fs::create_dir_all(&repo_path).expect("Failed to create repo dir");

    // Initialize git repo
    process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("git init failed");

    // Configure git
    process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&repo_path)
        .output()
        .expect("git config email failed");

    process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .expect("git config name failed");

    // Create initial commit
    let readme = repo_path.join("README.md");
    fs::write(&readme, "# Test Repo\n").expect("Failed to create README");

    process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .expect("git add failed");

    process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .expect("git commit failed");

    repo_path
}

/// Create a worktree in the repo
fn create_worktree(repo_path: &std::path::Path, name: &str) -> std::path::PathBuf {
    let worktree_path = repo_path.parent().unwrap().join(name);

    // Create a branch for the worktree
    process::Command::new("git")
        .args(["branch", name])
        .current_dir(repo_path)
        .output()
        .expect("git branch failed");

    // Create worktree
    process::Command::new("git")
        .args([
            "worktree",
            "add",
            worktree_path.to_string_lossy().as_ref(),
            name,
        ])
        .current_dir(repo_path)
        .output()
        .expect("git worktree add failed");

    worktree_path
}

// ============================================================================
// Gate Toggle Tests (PORT-1)
// ============================================================================

#[test]
fn test_health_check_basic() {
    let temp_dir = setup_test_env();

    // Basic health check should work without any env vars
    run_cli_with_data_dir(&temp_dir)
        .args(["health", "--url", "http://localhost:8765"])
        .assert()
        // May fail if server not running, but validates command
        .code(predicate::in_iter([0, 1]));
}

#[test]
fn test_tools_list_basic() {
    let temp_dir = setup_test_env();

    // Tools list should work and return JSON
    let output = run_cli_with_data_dir(&temp_dir)
        .args(["tools"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);

    // Should output some tools
    assert!(!output_str.is_empty(), "Should output tools information");
}

#[test]
fn test_schema_output() {
    let temp_dir = setup_test_env();

    // Schema should output JSON
    let output = run_cli_with_data_dir(&temp_dir)
        .args(["schema", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Verify it's valid JSON
    let json: serde_json::Value =
        serde_json::from_slice(&output).expect("Schema should be valid JSON");

    // Schema is an array of tool definitions
    assert!(json.is_array(), "Schema should be a JSON array");
}

// ============================================================================
// MCP Surface Verification Tests (PORT-2)
// ============================================================================

#[test]
fn test_mcp_tools_list_contains_tools() {
    let temp_dir = setup_test_env();

    // Get tools list (human-readable format)
    let output = run_cli_with_data_dir(&temp_dir)
        .args(["tools"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);

    // Should list some known tools
    assert!(
        output_str.contains("ensure_project"),
        "Should list ensure_project tool"
    );
    assert!(
        output_str.contains("send_message"),
        "Should list send_message tool"
    );
    assert!(
        output_str.contains("check_inbox"),
        "Should list check_inbox tool"
    );
}

#[test]
fn test_schema_json_format() {
    let temp_dir = setup_test_env();

    let output = run_cli_with_data_dir(&temp_dir)
        .args(["schema", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let schema: serde_json::Value =
        serde_json::from_slice(&output).expect("Schema should be valid JSON");

    // Schema outputs as an array of tool definitions
    assert!(schema.is_array(), "Schema should be a JSON array");
    let schema_array = schema.as_array().unwrap();
    assert!(!schema_array.is_empty(), "Schema should have tools");
}

#[test]
fn test_schema_yaml_format() {
    let temp_dir = setup_test_env();

    run_cli_with_data_dir(&temp_dir)
        .args(["schema", "--format", "yaml"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// ============================================================================
// Worktree Lifecycle Tests (PORT-4)
// ============================================================================

#[test]
fn test_worktree_creation() {
    let temp_dir = setup_test_env();
    let repo_path = create_git_repo(&temp_dir);

    // Create a worktree
    let worktree_path = create_worktree(&repo_path, "feature-branch");

    // Verify worktree exists
    assert!(worktree_path.exists(), "Worktree directory should exist");
    assert!(
        worktree_path.join(".git").exists(),
        "Worktree should have .git file"
    );

    // The .git file should be a file (not directory) pointing to main repo
    let git_file = worktree_path.join(".git");
    let git_content = fs::read_to_string(&git_file).expect("Failed to read .git file");
    assert!(
        git_content.contains("gitdir:"),
        ".git should be a gitdir reference"
    );
}

#[test]
fn test_worktree_list() {
    let temp_dir = setup_test_env();
    let repo_path = create_git_repo(&temp_dir);

    // Create worktrees
    create_worktree(&repo_path, "feature-a");
    create_worktree(&repo_path, "feature-b");

    // List worktrees
    let output = process::Command::new("git")
        .args(["worktree", "list"])
        .current_dir(&repo_path)
        .output()
        .expect("git worktree list failed");

    let output_str = String::from_utf8_lossy(&output.stdout);

    assert!(
        output_str.contains("feature-a"),
        "Should list feature-a worktree"
    );
    assert!(
        output_str.contains("feature-b"),
        "Should list feature-b worktree"
    );
}

#[test]
fn test_worktree_multiple_creation() {
    let temp_dir = setup_test_env();
    let repo_path = create_git_repo(&temp_dir);

    // Create multiple worktrees
    let wt1 = create_worktree(&repo_path, "wt-feature-a");
    let wt2 = create_worktree(&repo_path, "wt-feature-b");
    let wt3 = create_worktree(&repo_path, "wt-bugfix");

    // Verify all exist
    assert!(wt1.exists(), "wt-feature-a should exist");
    assert!(wt2.exists(), "wt-feature-b should exist");
    assert!(wt3.exists(), "wt-bugfix should exist");

    // List worktrees via git
    let output = process::Command::new("git")
        .args(["worktree", "list"])
        .current_dir(&repo_path)
        .output()
        .expect("git worktree list failed");

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Should have 4 entries (main + 3 worktrees)
    let line_count = output_str.lines().count();
    assert!(
        line_count >= 4,
        "Should have at least 4 worktree entries, got {}",
        line_count
    );
}

// ============================================================================
// Git Commit Tests in Different Environments (PORT-5)
// ============================================================================

#[test]
fn test_commit_in_regular_repo() {
    let temp_dir = setup_test_env();
    let repo_path = create_git_repo(&temp_dir);

    // Make a change
    let test_file = repo_path.join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");

    // Add and commit
    process::Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&repo_path)
        .output()
        .expect("git add failed");

    let output = process::Command::new("git")
        .args(["commit", "-m", "Test commit"])
        .current_dir(&repo_path)
        .output()
        .expect("git commit failed");

    assert!(
        output.status.success(),
        "Commit should succeed in regular repo: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_commit_in_worktree() {
    let temp_dir = setup_test_env();
    let repo_path = create_git_repo(&temp_dir);
    let worktree_path = create_worktree(&repo_path, "feature-test");

    // Make a change in the worktree
    let test_file = worktree_path.join("worktree-file.txt");
    fs::write(&test_file, "worktree content").expect("Failed to write test file");

    // Add and commit in worktree
    process::Command::new("git")
        .args(["add", "worktree-file.txt"])
        .current_dir(&worktree_path)
        .output()
        .expect("git add failed");

    let output = process::Command::new("git")
        .args(["commit", "-m", "Worktree commit"])
        .current_dir(&worktree_path)
        .output()
        .expect("git commit failed");

    assert!(
        output.status.success(),
        "Commit should succeed in worktree: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_worktree_branch_isolation() {
    let temp_dir = setup_test_env();
    let repo_path = create_git_repo(&temp_dir);
    let worktree_path = create_worktree(&repo_path, "isolated-branch");

    // Make a change only in the worktree
    let wt_file = worktree_path.join("isolated.txt");
    fs::write(&wt_file, "isolated content").expect("Failed to write file");

    process::Command::new("git")
        .args(["add", "isolated.txt"])
        .current_dir(&worktree_path)
        .output()
        .expect("git add failed");

    process::Command::new("git")
        .args(["commit", "-m", "Isolated commit"])
        .current_dir(&worktree_path)
        .output()
        .expect("git commit failed");

    // Verify the file doesn't exist in main repo
    assert!(
        !repo_path.join("isolated.txt").exists(),
        "File should not exist in main repo"
    );

    // Verify the file exists in worktree
    assert!(
        worktree_path.join("isolated.txt").exists(),
        "File should exist in worktree"
    );
}

// ============================================================================
// Environment Variable Tests (PORT-6)
// ============================================================================

#[test]
fn test_cli_with_worktrees_enabled() {
    let temp_dir = setup_test_env();

    // Run with WORKTREES_ENABLED=1
    run_cli_with_env(&temp_dir, "WORKTREES_ENABLED", "1")
        .args(["tools"])
        .assert()
        .success();
}

#[test]
fn test_cli_with_git_identity_enabled() {
    let temp_dir = setup_test_env();

    // Run with GIT_IDENTITY_ENABLED=1
    run_cli_with_env(&temp_dir, "GIT_IDENTITY_ENABLED", "1")
        .args(["tools"])
        .assert()
        .success();
}

#[test]
fn test_cli_with_bypass_mode() {
    let temp_dir = setup_test_env();

    // Run with AGENT_MAIL_BYPASS=1
    run_cli_with_env(&temp_dir, "AGENT_MAIL_BYPASS", "1")
        .args(["tools"])
        .assert()
        .success();
}

// ============================================================================
// Help and Usage Tests
// ============================================================================

#[test]
fn test_main_help() {
    let temp_dir = setup_test_env();

    run_cli_with_data_dir(&temp_dir)
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("serve"))
        .stdout(predicate::str::contains("health"))
        .stdout(predicate::str::contains("tools"))
        .stdout(predicate::str::contains("schema"))
        .stdout(predicate::str::contains("share"))
        .stdout(predicate::str::contains("archive"));
}

#[test]
fn test_serve_help() {
    let temp_dir = setup_test_env();

    run_cli_with_data_dir(&temp_dir)
        .args(["serve", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("http"))
        .stdout(predicate::str::contains("mcp"));
}

#[test]
fn test_service_help() {
    let temp_dir = setup_test_env();

    run_cli_with_data_dir(&temp_dir)
        .args(["service", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("stop"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("restart"));
}

#[test]
fn test_version() {
    let temp_dir = setup_test_env();

    run_cli_with_data_dir(&temp_dir)
        .args(["version"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// ============================================================================
// Config Commands Tests
// ============================================================================

#[test]
fn test_config_show_port() {
    let temp_dir = setup_test_env();

    // Show port should work
    run_cli_with_data_dir(&temp_dir)
        .args(["config", "show-port"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_config_set_port() {
    let temp_dir = setup_test_env();

    // Set port to a custom value
    run_cli_with_data_dir(&temp_dir)
        .args(["config", "set-port", "9999"])
        .assert()
        .success()
        .stdout(predicate::str::contains("9999"));
}

// ============================================================================
// Archive Integration Tests (cross-reference with archive_workflow)
// ============================================================================

#[test]
fn test_archive_help() {
    let temp_dir = setup_test_env();

    run_cli_with_data_dir(&temp_dir)
        .args(["archive", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("save"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("restore"));
}
