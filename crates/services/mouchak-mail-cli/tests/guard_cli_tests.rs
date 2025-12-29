use assert_cmd::Command;
// use predicates::prelude::*; -> Removed
use predicates::str::contains;
use tempfile::TempDir;

#[test]
#[allow(clippy::unwrap_used, clippy::expect_used, deprecated)]
fn test_guard_status_command() {
    // Setup temp git repo structure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let hooks_dir = temp_dir.path().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks_dir).expect("Failed to create hooks dir");

    // Run command (Expect failure as subcommand doesn't exist yet)
    // We expect it to FAIL initially (RED)
    // But once implemented, it should succeed.
    // To verify RED, we check for failure OR check output doesn't match.
    // Current mcp-cli has no "guard" command. It will return exit code != 0.

    let mut cmd = Command::cargo_bin("mouchak-mail-cli").expect("Binary not found");
    cmd.current_dir(&temp_dir)
        .arg("guard")
        .arg("status")
        .assert()
        .success() // Now Green
        .stdout(contains("Installed hooks:"));
}
