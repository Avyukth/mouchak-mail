use assert_cmd::Command;
use predicates::prelude::*;
use std::time::Duration;

#[test]
fn test_service_lifecycle() {
    let port = 9099; // Use a high port unlikely to be used

    // 1. Ensure clean state (stop any leftover)
    let _ = Command::cargo_bin("mcp-agent-mail")
        .unwrap()
        .arg("service")
        .arg("stop")
        .arg("--port")
        .arg(port.to_string())
        .ok();

    // 2. Start server using SERVICE START (background)
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("service")
        .arg("start")
        .arg("--port")
        .arg(port.to_string())
        .arg("--background")
        .assert()
        .success()
        .stdout(predicate::str::contains("Server started"));

    // Give it time to start
    std::thread::sleep(Duration::from_secs(2));

    // 3. Check Status
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("service")
        .arg("status")
        .arg("--port")
        .arg(port.to_string())
        .assert()
        .success()
        .stdout(predicate::str::contains("Server RUNNING"));

    // 4. Restart Server (should succeed and keep running)
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("service")
        .arg("restart")
        .arg("--port")
        .arg(port.to_string())
        .assert()
        .success()
        .stdout(predicate::str::contains("Restarting server"));

    // Give it time to restart
    std::thread::sleep(Duration::from_secs(2));

    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("service")
        .arg("status")
        .arg("--port")
        .arg(port.to_string())
        .assert()
        .success()
        .stdout(predicate::str::contains("Server RUNNING"));

    // 5. Stop Server
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("service")
        .arg("stop")
        .arg("--port")
        .arg(port.to_string())
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped server"));

    // 6. Verify Stopped
    std::thread::sleep(Duration::from_secs(1));
    let mut cmd = Command::cargo_bin("mcp-agent-mail").unwrap();
    cmd.arg("service")
        .arg("status")
        .arg("--port")
        .arg(port.to_string())
        .assert()
        .success()
        .stdout(predicate::str::contains("No server running"));
}
