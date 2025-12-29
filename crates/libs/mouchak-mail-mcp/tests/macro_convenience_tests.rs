//! Integration tests for convenience macros (require running server)
//! Run with: cargo test -p lib-mcp --test macro_convenience_tests -- --ignored

use reqwest::Client;
use serde_json::json;

/// Test macro_start_session combines register_agent + file_reservation_paths
#[tokio::test]
#[ignore = "Requires running MCP server"]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_macro_start_session() {
    let client = Client::new();
    let base_url =
        std::env::var("MOUCHAK_MAIL_URL").unwrap_or_else(|_| "http://localhost:8765".to_string());

    // Call macro_start_session
    let response = client
        .post(format!("{}/api/macros/start_session", base_url))
        .json(&json!({
            "project_slug": "test-macro-project",
            "name": "macro-test-agent",
            "model": "test-model",
            "program": "test-program",
            "patterns": ["src/**/*.rs"],
            "ttl_seconds": 3600
        }))
        .send()
        .await
        .expect("Failed to call macro_start_session");

    assert!(
        response.status().is_success(),
        "Expected 2xx, got {}",
        response.status()
    );

    let body: serde_json::Value = response.json().await.unwrap();

    // Verify agent was registered
    assert!(
        body.get("agent_id").is_some(),
        "Expected agent_id in response"
    );

    // Verify reservation was created
    assert!(
        body.get("reservation_ids").is_some(),
        "Expected reservation_ids in response"
    );
}

/// Test macro_file_reservation_cycle reserve action
#[tokio::test]
#[ignore = "Requires running MCP server"]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_macro_file_reservation_cycle_reserve() {
    let client = Client::new();
    let base_url =
        std::env::var("MOUCHAK_MAIL_URL").unwrap_or_else(|_| "http://localhost:8765".to_string());

    let response = client
        .post(format!("{}/api/macros/file_reservation_cycle", base_url))
        .json(&json!({
            "project_slug": "test-macro-project",
            "agent_name": "macro-test-agent",
            "patterns": ["tests/**"],
            "action": "reserve"
        }))
        .send()
        .await
        .expect("Failed to call macro_file_reservation_cycle");

    assert!(response.status().is_success());
}

/// Test macro_contact_handshake creates bidirectional contacts
#[tokio::test]
#[ignore = "Requires running MCP server"]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_macro_contact_handshake() {
    let client = Client::new();
    let base_url =
        std::env::var("MOUCHAK_MAIL_URL").unwrap_or_else(|_| "http://localhost:8765".to_string());

    let response = client
        .post(format!("{}/api/macros/contact_handshake", base_url))
        .json(&json!({
            "project_slug": "test-macro-project",
            "requester": "agent-a",
            "target": "agent-b"
        }))
        .send()
        .await
        .expect("Failed to call macro_contact_handshake");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("contacts_created").is_some());
}
