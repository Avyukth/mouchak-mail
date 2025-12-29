#![allow(clippy::unwrap_used, clippy::expect_used)] // expect/unwrap is fine in tests
#![allow(deprecated)] // cargo_bin is still valid for our use case
#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("mouchak-mail").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("mouchak-mail"));
}

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("mouchak-mail").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Unified Server/CLI for Mouchak Mail",
        ));
}

#[test]
fn test_serve_http_command_dry_run() {
    // We can't easily dry-run a server without it blocking, unless we use a timeout or specific verify mode.
    // Clap parsing check is decent enough for "dry run" of args.
    // Let's check bad arguments fail.
    let mut cmd = Command::cargo_bin("mouchak-mail").unwrap();
    cmd.args(["serve", "http", "--port", "invalid"])
        .assert()
        .failure();
}

#[test]
fn test_schema_command() {
    let mut cmd = Command::cargo_bin("mouchak-mail").unwrap();
    cmd.args(["schema", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("send_message")); // Expect at least one known tool
}

#[test]
fn test_tools_command() {
    let mut cmd = Command::cargo_bin("mouchak-mail").unwrap();
    cmd.arg("tools")
        .assert()
        .success()
        .stdout(predicate::str::contains("Mouchak Mail Tools"));
}
