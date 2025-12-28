#![allow(clippy::unwrap_used)]
#![allow(deprecated)] // cargo_bin is still valid for our use case

use assert_cmd::Command;
use predicates::prelude::*;

/// Test guard check help output
#[test]
fn test_guard_check_help() {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("guard")
        .arg("check")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--stdin-nul"))
        .stdout(predicate::str::contains("--advisory"))
        .stdout(predicate::str::contains("--project"));
}

/// Test guard check with empty stdin (advisory mode)
#[test]
fn test_guard_check_empty_stdin_advisory() {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("guard")
        .arg("check")
        .arg("--advisory")
        .write_stdin("")
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning: No paths provided"));
}

/// Test guard check with empty stdin (strict mode - should fail)
#[test]
fn test_guard_check_empty_stdin_strict() {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("guard")
        .arg("check")
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error: No paths provided"));
}

/// Test guard check with project flag
#[test]
fn test_guard_check_with_project_flag() {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("guard")
        .arg("check")
        .arg("--project")
        .arg("test-project")
        .arg("--advisory")
        .write_stdin("src/main.rs\n")
        .assert()
        .success();
    // In advisory mode without server, should warn but succeed
}

/// Test guard check with null-separated paths
#[test]
fn test_guard_check_stdin_nul() {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("guard")
        .arg("check")
        .arg("--stdin-nul")
        .arg("--advisory")
        .write_stdin("src/main.rs\0src/lib.rs\0")
        .assert()
        .success();
}

/// Test guard status command exists
#[test]
fn test_guard_status_help() {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("guard")
        .arg("status")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("guard status"));
}
