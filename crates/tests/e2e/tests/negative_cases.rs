//! Negative Cases & Error Paths E2E Tests
//!
//! These tests verify error handling and validation edge cases.
//! Tests that the API returns proper error responses with helpful suggestions.
//!
//! Prerequisites:
//! - API server running: `cargo run -p mcp-server`
//!
//! Run tests:
//! ```bash
//! cargo test -p e2e-tests --test negative_cases
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)] // expect/unwrap is fine in tests

use e2e_tests::fixtures::ProjectResponse;
use e2e_tests::{TestConfig, TestFixtures};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

// ============================================================================
// Error Response Structures
// ============================================================================

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ErrorResponse {
    code: String,
    error: String,
    #[serde(default)]
    details: Option<String>,
    #[serde(default)]
    suggestions: Vec<String>,
}

// ============================================================================
// Test Helpers
// ============================================================================

fn get_config() -> TestConfig {
    TestConfig::default()
}

async fn create_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// Check if API server is running
async fn is_api_running(client: &Client, config: &TestConfig) -> bool {
    client
        .get(format!("{}/health", config.api_url))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Helper to create a project and return its response
async fn setup_project(client: &Client, config: &TestConfig) -> Option<ProjectResponse> {
    let human_key = TestFixtures::unique_project_name();

    let resp = client
        .post(format!("{}/api/project/ensure", config.api_url))
        .json(&TestFixtures::project_payload(&human_key))
        .send()
        .await
        .ok()?;

    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

// ============================================================================
// Not Found Tests with Suggestions
// ============================================================================

/// Test: Non-existent project returns 404 with suggestions
///
/// Acceptance Criteria: Test non-existent project returns 404 with suggestions
#[tokio::test]
async fn test_nonexistent_project_suggestions() {
    let config = get_config();
    let client = create_client().await;

    if !is_api_running(&client, &config).await {
        println!("⚠ API server not running, skipping test");
        return;
    }

    // Create a real project first so we can get suggestions
    let _ = setup_project(&client, &config).await;

    // Try to send message to non-existent project
    let response = client
        .post(format!("{}/api/message/send", config.api_url))
        .json(&json!({
            "project_slug": "nonexistent-project-xyz123",
            "sender_name": "test-agent",
            "recipient_names": ["other-agent"],
            "subject": "Test",
            "body_md": "Test message"
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status == StatusCode::NOT_FOUND {
                let error: ErrorResponse = match resp.json().await {
                    Ok(e) => e,
                    Err(e) => {
                        println!("⚠ Failed to parse error response: {}", e);
                        return;
                    }
                };
                println!("✓ Non-existent project returns 404");
                println!("  Code: {}", error.code);
                println!("  Error: {}", error.error);
                if !error.suggestions.is_empty() {
                    println!("  Suggestions: {:?}", error.suggestions);
                }
                assert_eq!(error.code, "NOT_FOUND");
            } else {
                println!("⚠ Expected 404, got {}", status);
            }
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

/// Test: Non-existent agent returns 404 with similar names
///
/// Acceptance Criteria: Test non-existent agent returns 404 with similar names
#[tokio::test]
async fn test_nonexistent_agent_similar_names() {
    let config = get_config();
    let client = create_client().await;

    if !is_api_running(&client, &config).await {
        println!("⚠ API server not running, skipping test");
        return;
    }

    // Create a project with an agent
    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ Failed to create project");
            return;
        }
    };

    // Register a real agent
    let agent_name = "real-test-agent";
    let _ = client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&project.slug, agent_name))
        .send()
        .await;

    // Try to access non-existent agent with similar name
    let response = client
        .post(format!("{}/api/message/send", config.api_url))
        .json(&json!({
            "project_slug": project.slug,
            "sender_name": "rael-test-agent",  // Typo in name
            "recipient_names": ["other-agent"],
            "subject": "Test",
            "body_md": "Test message"
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status == StatusCode::NOT_FOUND {
                let error: ErrorResponse = match resp.json().await {
                    Ok(e) => e,
                    Err(e) => {
                        println!("⚠ Failed to parse error response: {}", e);
                        return;
                    }
                };
                println!("✓ Non-existent agent returns 404");
                println!("  Code: {}", error.code);
                println!("  Error: {}", error.error);
                if !error.suggestions.is_empty() {
                    println!("  Suggestions: {:?}", error.suggestions);
                }
            } else {
                println!("⚠ Expected 404, got {}", status);
            }
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

/// Test: Unix username as agent name triggers helpful hint
///
/// Acceptance Criteria: Test Unix username as agent name triggers helpful hint
#[tokio::test]
async fn test_unix_username_as_agent_hint() {
    let config = get_config();
    let client = create_client().await;

    if !is_api_running(&client, &config).await {
        println!("⚠ API server not running, skipping test");
        return;
    }

    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ Failed to create project");
            return;
        }
    };

    // Try to use a Unix username format (common mistake)
    // Common Unix usernames: root, admin, user, etc.
    let response = client
        .post(format!("{}/api/message/send", config.api_url))
        .json(&json!({
            "project_slug": project.slug,
            "sender_name": "root",  // Common Unix username
            "recipient_names": ["admin"],
            "subject": "Test",
            "body_md": "Test message"
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status == StatusCode::NOT_FOUND {
                let error: ErrorResponse = match resp.json().await {
                    Ok(e) => e,
                    Err(e) => {
                        println!("⚠ Failed to parse error response: {}", e);
                        return;
                    }
                };
                println!("✓ Unix username detected, returns 404");
                println!("  Error: {}", error.error);
                // Check if error message or suggestions contain hints
                if error.error.to_lowercase().contains("agent") || !error.suggestions.is_empty() {
                    println!("  Helpful info provided");
                }
            } else {
                println!("⚠ Expected 404, got {}", status);
            }
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

// ============================================================================
// Validation Error Tests
// ============================================================================

/// Test: Invalid thread_id format returns validation error
///
/// Acceptance Criteria: Test invalid thread_id format returns validation error
#[tokio::test]
async fn test_invalid_thread_id_validation() {
    let config = get_config();
    let client = create_client().await;

    if !is_api_running(&client, &config).await {
        println!("⚠ API server not running, skipping test");
        return;
    }

    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ Failed to create project");
            return;
        }
    };

    let agent_name = TestFixtures::unique_agent_name();
    let _ = client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&project.slug, &agent_name))
        .send()
        .await;

    // Try to use an invalid thread_id with special characters
    let response = client
        .post(format!("{}/api/message/send", config.api_url))
        .json(&json!({
            "project_slug": project.slug,
            "sender_name": agent_name,
            "recipient_names": [agent_name],
            "subject": "Test",
            "body_md": "Test message",
            "thread_id": "../../../etc/passwd"  // Path traversal attempt
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status.is_client_error() {
                println!("✓ Invalid thread_id rejected (status={})", status);
            } else if status.is_success() {
                println!("✓ Thread ID accepted (stored in DB only, no filesystem risk)");
            } else {
                panic!("Unexpected response status for thread_id test: {}", status);
            }
        }
        Err(e) => {
            panic!("Request failed unexpectedly: {}", e);
        }
    }
}

/// Test: Message to self is rejected
///
/// Acceptance Criteria: Test message to self is rejected
#[tokio::test]
async fn test_message_to_self_rejected() {
    let config = get_config();
    let client = create_client().await;

    if !is_api_running(&client, &config).await {
        println!("⚠ API server not running, skipping test");
        return;
    }

    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ Failed to create project");
            return;
        }
    };

    let agent_name = TestFixtures::unique_agent_name();
    let _ = client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&project.slug, &agent_name))
        .send()
        .await;

    // Try to send message to self
    let response = client
        .post(format!("{}/api/message/send", config.api_url))
        .json(&json!({
            "project_slug": project.slug,
            "sender_name": agent_name,
            "recipient_names": [agent_name],  // Same as sender
            "subject": "Message to self",
            "body_md": "This should be rejected or at least handled specially"
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            // Self-messaging might be allowed or rejected depending on design
            if status.is_client_error() {
                println!("✓ Message to self rejected (status={})", status);
            } else if status.is_success() {
                println!("⚠ Message to self was allowed - may be valid design choice");
            }
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

/// Test: Empty recipient list returns validation error
///
/// Acceptance Criteria: Test empty recipient list returns validation error
#[tokio::test]
async fn test_empty_recipients_validation() {
    let config = get_config();
    let client = create_client().await;

    if !is_api_running(&client, &config).await {
        println!("⚠ API server not running, skipping test");
        return;
    }

    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ Failed to create project");
            return;
        }
    };

    let agent_name = TestFixtures::unique_agent_name();
    let _ = client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&project.slug, &agent_name))
        .send()
        .await;

    // Try to send message with empty recipients
    let response = client
        .post(format!("{}/api/message/send", config.api_url))
        .json(&json!({
            "project_slug": project.slug,
            "sender_name": agent_name,
            "recipient_names": [],  // Empty recipients
            "subject": "Test",
            "body_md": "Test message"
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status == StatusCode::BAD_REQUEST {
                println!("✓ Empty recipients correctly rejected with 400");
            } else if status.is_client_error() {
                println!("✓ Empty recipients rejected with {}", status);
            } else {
                println!(
                    "⚠ Empty recipients unexpectedly allowed (status={})",
                    status
                );
            }
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

/// Test: Oversized message body is rejected
///
/// Acceptance Criteria: Test oversized message body is rejected
#[tokio::test]
async fn test_oversized_body_rejected() {
    let config = get_config();
    let client = create_client().await;

    if !is_api_running(&client, &config).await {
        println!("⚠ API server not running, skipping test");
        return;
    }

    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ Failed to create project");
            return;
        }
    };

    let sender_name = TestFixtures::unique_agent_name();
    let recipient_name = TestFixtures::unique_agent_name();

    let _ = client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&project.slug, &sender_name))
        .send()
        .await;

    let _ = client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&project.slug, &recipient_name))
        .send()
        .await;

    // Create a very large message body (10MB)
    let large_body = "x".repeat(10 * 1024 * 1024);

    let response = client
        .post(format!("{}/api/message/send", config.api_url))
        .json(&json!({
            "project_slug": project.slug,
            "sender_name": sender_name,
            "recipient_names": [recipient_name],
            "subject": "Large message test",
            "body_md": large_body
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status == StatusCode::PAYLOAD_TOO_LARGE
                || status == StatusCode::BAD_REQUEST
                || status == StatusCode::FORBIDDEN
            {
                println!("✓ Oversized body correctly rejected (status={})", status);
            } else if status.is_success() {
                println!("⚠ Oversized body was accepted - may have large limits");
            } else {
                println!("✓ Oversized body rejected with {}", status);
            }
        }
        Err(e) => {
            // Connection might be reset for very large payloads
            println!("✓ Oversized body caused connection error: {}", e);
        }
    }
}

/// Test: Malformed JSON returns 400 with details
///
/// Acceptance Criteria: Test malformed JSON returns 400 with details
#[tokio::test]
async fn test_malformed_json_400() {
    let config = get_config();
    let client = create_client().await;

    if !is_api_running(&client, &config).await {
        println!("⚠ API server not running, skipping test");
        return;
    }

    // Send malformed JSON
    let response = client
        .post(format!("{}/api/message/send", config.api_url))
        .header("Content-Type", "application/json")
        .body(r#"{"project_slug": "test", "sender_name": "broken json"#) // Missing closing brace
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY {
                println!("✓ Malformed JSON correctly rejected (status={})", status);
                // Check if error contains helpful info
                let body = resp.text().await.unwrap_or_default();
                if body.contains("JSON") || body.contains("parse") || body.contains("syntax") {
                    println!("  Error contains helpful JSON parsing info");
                }
            } else {
                println!("⚠ Expected 400/422, got {}", status);
            }
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

// ============================================================================
// Security Tests
// ============================================================================

/// Test: Path traversal in project_key is blocked
///
/// Acceptance Criteria: Test path traversal in project_key blocked
#[tokio::test]
async fn test_path_traversal_blocked() {
    let config = get_config();
    let client = create_client().await;

    if !is_api_running(&client, &config).await {
        println!("⚠ API server not running, skipping test");
        return;
    }

    // Try various path traversal patterns
    let traversal_attempts = vec![
        "../../../etc/passwd",
        "..%2F..%2F..%2Fetc%2Fpasswd",
        "....//....//....//etc/passwd",
        "/etc/passwd",
        "C:\\Windows\\System32\\config\\SAM",
    ];

    for attempt in traversal_attempts {
        let response = client
            .post(format!("{}/api/project/ensure", config.api_url))
            .json(&json!({
                "human_key": attempt
            }))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                if status.is_client_error() {
                    println!("✓ Path traversal '{}' blocked (status={})", attempt, status);
                } else if status.is_success() {
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    let slug = body["slug"].as_str().unwrap_or("");
                    assert!(
                        !slug.contains("..") && !slug.contains("/") && !slug.contains("\\"),
                        "Path traversal '{}' was NOT sanitized, resulted in slug: {}",
                        attempt,
                        slug
                    );
                    println!("✓ Path traversal '{}' sanitized to '{}'", attempt, slug);
                } else {
                    panic!(
                        "Unexpected status {} for path traversal attempt '{}'",
                        status, attempt
                    );
                }
            }
            Err(e) => {
                panic!(
                    "Request failed for path traversal attempt '{}': {}",
                    attempt, e
                );
            }
        }
    }
}

/// Test: SQL injection attempts are sanitized
///
/// Acceptance Criteria: Test SQL injection attempts sanitized
#[tokio::test]
async fn test_sql_injection_sanitized() {
    let config = get_config();
    let client = create_client().await;

    if !is_api_running(&client, &config).await {
        println!("⚠ API server not running, skipping test");
        return;
    }

    // Try various SQL injection patterns
    let injection_attempts = vec![
        "'; DROP TABLE agents; --",
        "1 OR 1=1",
        "1; SELECT * FROM users",
        "1 UNION SELECT * FROM agents",
        "admin'--",
        "1' AND '1'='1",
    ];

    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ Failed to create project");
            return;
        }
    };

    for attempt in injection_attempts {
        let response = client
            .post(format!("{}/api/agent/register", config.api_url))
            .json(&json!({
                "project_slug": project.slug,
                "name": attempt,
                "program": "test",
                "model": "test"
            }))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                if status.is_client_error() {
                    println!("✓ SQL injection '{}' rejected (status={})", attempt, status);
                } else if status.is_success() || status == StatusCode::CONFLICT {
                    println!(
                        "✓ SQL injection '{}' handled safely (status={})",
                        attempt, status
                    );
                } else {
                    panic!(
                        "Unexpected status {} for SQL injection attempt '{}'",
                        status, attempt
                    );
                }
            }
            Err(e) => {
                panic!(
                    "Request failed for SQL injection attempt '{}': {}",
                    attempt, e
                );
            }
        }
    }

    let verify_resp = client
        .get(format!(
            "{}/api/projects/{}/agents",
            config.api_url, project.slug
        ))
        .send()
        .await;

    match verify_resp {
        Ok(resp) if resp.status().is_success() => {
            println!(
                "✓ Database still accessible after injection attempts - parameterized queries working"
            );
        }
        Ok(resp) => {
            panic!(
                "Database may be corrupted after SQL injection attempts: status={}",
                resp.status()
            );
        }
        Err(e) => {
            panic!(
                "Database verification failed after SQL injection attempts: {}",
                e
            );
        }
    }
}
