#![allow(clippy::unwrap_used)]
#![allow(deprecated)] // cargo_bin is still valid for our use case

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_products_subcommand_help() {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("products")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Ensure a product exists"));
}

#[test]
fn test_products_ensure() {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("products")
        .arg("ensure")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage:")); // Capital U
}

#[test]
fn test_products_link() {
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("products")
        .arg("link")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage:")); // Capital U
}
