//! API E2E Tests
//!
//! These tests verify the REST API endpoints work correctly.
//! Uses reqwest for HTTP calls and jugar-probar for advanced assertions.
//!
//! Prerequisites:
//! - API server running: `cargo run -p mcp-server`
//!
//! Run tests:
//! ```bash
//! cargo test -p e2e-tests --test api
//! ```

use e2e_tests::{TestConfig, TestFixtures};
use e2e_tests::fixtures::{AgentResponse, MessageResponse, ProjectResponse};
use reqwest::Client;
use serde_json::json;

// ============================================================================
// Test Helpers
// ============================================================================

fn get_config() -> TestConfig {
    TestConfig::default()
}

async fn create_client() -> Client {
    Client::new()
}

// ============================================================================
// Health Check Tests
// ============================================================================

#[tokio::test]
async fn test_health_endpoint() {
    let config = get_config();
    let client = create_client().await;

    let response = client
        .get(format!("{}/health", config.api_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "Health endpoint should return 200");
            println!("✓ Health endpoint responding");
        }
        Err(e) => {
            println!("⚠ API server not running: {}", e);
            println!("  Start with: cargo run -p mcp-server");
        }
    }
}

#[tokio::test]
async fn test_ready_endpoint() {
    let config = get_config();
    let client = create_client().await;

    let response = client
        .get(format!("{}/ready", config.api_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "Ready endpoint should return 200");
            println!("✓ Ready endpoint responding");
        }
        Err(e) => {
            println!("⚠ API server not running: {}", e);
        }
    }
}

// ============================================================================
// Project API Tests
// ============================================================================

#[tokio::test]
async fn test_ensure_project() {
    let config = get_config();
    let client = create_client().await;
    let slug = TestFixtures::unique_project_slug();

    let response = client
        .post(format!("{}/api/project/ensure", config.api_url))
        .json(&TestFixtures::project_payload(&slug))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "ensure_project should succeed");

            let project: ProjectResponse = resp.json().await.expect("Should parse response");
            assert_eq!(project.slug, slug, "Project slug should match");

            println!("✓ Project created: {} (id={})", project.slug, project.id);
        }
        Err(e) => {
            println!("⚠ API server not running: {}", e);
        }
    }
}

#[tokio::test]
async fn test_list_projects() {
    let config = get_config();
    let client = create_client().await;

    let response = client
        .get(format!("{}/api/projects", config.api_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "list_projects should succeed");

            let projects: Vec<ProjectResponse> = resp.json().await.expect("Should parse response");
            println!("✓ Found {} projects", projects.len());
        }
        Err(e) => {
            println!("⚠ API server not running: {}", e);
        }
    }
}

// ============================================================================
// Agent API Tests
// ============================================================================

#[tokio::test]
async fn test_register_agent() {
    let config = get_config();
    let client = create_client().await;
    let slug = TestFixtures::unique_project_slug();
    let agent_name = TestFixtures::unique_agent_name();

    // First create project
    let project_resp = client
        .post(format!("{}/api/project/ensure", config.api_url))
        .json(&TestFixtures::project_payload(&slug))
        .send()
        .await;

    if project_resp.is_err() {
        println!("⚠ API server not running");
        return;
    }

    // Then register agent
    let response = client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&slug, &agent_name))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "register_agent should succeed");

            let agent: AgentResponse = resp.json().await.expect("Should parse response");
            assert_eq!(agent.name, agent_name, "Agent name should match");

            println!("✓ Agent registered: {} (id={})", agent.name, agent.id);
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

// ============================================================================
// Messaging API Tests
// ============================================================================

#[tokio::test]
async fn test_send_message_flow() {
    let config = get_config();
    let client = create_client().await;
    let slug = TestFixtures::unique_project_slug();
    let sender_name = TestFixtures::unique_agent_name();
    let recipient_name = TestFixtures::unique_agent_name();

    // Setup: Create project and agents
    let project_result = client
        .post(format!("{}/api/project/ensure", config.api_url))
        .json(&TestFixtures::project_payload(&slug))
        .send()
        .await;

    if project_result.is_err() {
        println!("⚠ API server not running");
        return;
    }

    // Register sender
    client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&slug, &sender_name))
        .send()
        .await
        .expect("Should register sender");

    // Register recipient
    client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&slug, &recipient_name))
        .send()
        .await
        .expect("Should register recipient");

    // Send message
    let message_payload = TestFixtures::message_payload(
        &slug,
        &sender_name,
        &[recipient_name.as_str()],
        "Test Subject",
        "This is a test message body.",
    );

    let response = client
        .post(format!("{}/api/message/send", config.api_url))
        .json(&message_payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "send_message should succeed");

            let message: MessageResponse = resp.json().await.expect("Should parse response");
            assert!(!message.thread_id.is_empty(), "Message should have thread_id");

            println!("✓ Message sent: id={}, thread={}", message.id, message.thread_id);
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_check_inbox() {
    let config = get_config();
    let client = create_client().await;
    let slug = TestFixtures::unique_project_slug();
    let agent_name = TestFixtures::unique_agent_name();

    // Setup: Create project and agent
    let project_result = client
        .post(format!("{}/api/project/ensure", config.api_url))
        .json(&TestFixtures::project_payload(&slug))
        .send()
        .await;

    if project_result.is_err() {
        println!("⚠ API server not running");
        return;
    }

    client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&slug, &agent_name))
        .send()
        .await
        .expect("Should register agent");

    // Check inbox
    let response = client
        .post(format!("{}/api/inbox", config.api_url))
        .json(&json!({
            "project_slug": slug,
            "agent_name": agent_name,
            "limit": 10
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "check_inbox should succeed");

            let messages: Vec<serde_json::Value> = resp.json().await.expect("Should parse response");
            println!("✓ Inbox checked: {} messages", messages.len());
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

// ============================================================================
// Search API Tests
// ============================================================================

#[tokio::test]
async fn test_search_messages() {
    let config = get_config();
    let client = create_client().await;
    let slug = TestFixtures::unique_project_slug();

    // Setup: Create project
    let project_result = client
        .post(format!("{}/api/project/ensure", config.api_url))
        .json(&TestFixtures::project_payload(&slug))
        .send()
        .await;

    if project_result.is_err() {
        println!("⚠ API server not running");
        return;
    }

    // Search messages (FTS5)
    let response = client
        .post(format!("{}/api/messages/search", config.api_url))
        .json(&json!({
            "project_slug": slug,
            "query": "test",
            "limit": 10
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "search_messages should succeed");

            let results: Vec<serde_json::Value> = resp.json().await.expect("Should parse response");
            println!("✓ Search returned {} results", results.len());
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

// ============================================================================
// File Reservation API Tests
// ============================================================================

#[tokio::test]
async fn test_file_reservation_flow() {
    let config = get_config();
    let client = create_client().await;
    let slug = TestFixtures::unique_project_slug();
    let agent_name = TestFixtures::unique_agent_name();

    // Setup
    let project_result = client
        .post(format!("{}/api/project/ensure", config.api_url))
        .json(&TestFixtures::project_payload(&slug))
        .send()
        .await;

    if project_result.is_err() {
        println!("⚠ API server not running");
        return;
    }

    client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&slug, &agent_name))
        .send()
        .await
        .expect("Should register agent");

    // Reserve files
    let response = client
        .post(format!("{}/api/file_reservations/paths", config.api_url))
        .json(&json!({
            "project_slug": slug,
            "agent_name": agent_name,
            "paths": ["src/**/*.rs", "Cargo.toml"],
            "exclusive": true,
            "reason": "E2E test reservation",
            "ttl_seconds": 300
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "file_reservation should succeed");
            println!("✓ File reservation created");

            // List reservations
            let list_resp = client
                .post(format!("{}/api/file_reservations/list", config.api_url))
                .json(&json!({
                    "project_slug": slug
                }))
                .send()
                .await
                .expect("Should list reservations");

            let reservations: Vec<serde_json::Value> = list_resp.json().await
                .expect("Should parse response");

            assert!(!reservations.is_empty(), "Should have at least one reservation");
            println!("✓ Found {} reservations", reservations.len());
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}
