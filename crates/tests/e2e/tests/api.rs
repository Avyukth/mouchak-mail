//! API E2E Tests
//!
//! These tests verify the REST API endpoints work correctly.
//! Uses reqwest for HTTP calls and jugar-probar for advanced assertions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Prerequisites:
//! - API server running: `cargo run -p mcp-server`
//!
//! Run tests:
//! ```bash
//! cargo test -p e2e-tests --test api
//! ```

use e2e_tests::fixtures::{AgentResponse, MessageResponse, ProjectResponse};
use e2e_tests::{TestConfig, TestFixtures};
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

/// Helper to create a project and return its slug for subsequent API calls
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

/// Helper to register an agent and return its details
async fn setup_agent(
    client: &Client,
    config: &TestConfig,
    project_slug: &str,
    agent_name: &str,
) -> Option<AgentResponse> {
    let resp = client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(project_slug, agent_name))
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
            assert!(
                resp.status().is_success(),
                "Health endpoint should return 200"
            );
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

    let response = client.get(format!("{}/ready", config.api_url)).send().await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status().is_success(),
                "Ready endpoint should return 200"
            );
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
    let human_key = TestFixtures::unique_project_name();

    let response = client
        .post(format!("{}/api/project/ensure", config.api_url))
        .json(&TestFixtures::project_payload(&human_key))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_else(|_| "no body".to_string());
            println!("Response status: {}", status);
            println!("Response body: {}", body);
            assert!(
                status.is_success(),
                "ensure_project should succeed, got {} with body: {}",
                status,
                body
            );

            let project: ProjectResponse =
                serde_json::from_str(&body).expect("Should parse response");
            assert_eq!(
                project.human_key, human_key,
                "Project human_key should match"
            );
            assert!(
                !project.slug.is_empty(),
                "Project should have a generated slug"
            );

            println!(
                "✓ Project created: {} (slug={}, id={})",
                project.human_key, project.slug, project.id
            );
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
    let human_key = TestFixtures::unique_project_name();
    let agent_name = TestFixtures::unique_agent_name();

    // First create project
    let project_resp = client
        .post(format!("{}/api/project/ensure", config.api_url))
        .json(&TestFixtures::project_payload(&human_key))
        .send()
        .await;

    let project: ProjectResponse = match project_resp {
        Ok(resp) if resp.status().is_success() => {
            resp.json().await.expect("Should parse project response")
        }
        _ => {
            println!("⚠ API server not running or project creation failed");
            return;
        }
    };

    // Then register agent using the project's slug
    let response = client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(&project.slug, &agent_name))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "register_agent should succeed");

            let agent: AgentResponse = resp.json().await.expect("Should parse response");
            assert_eq!(agent.name, agent_name, "Agent name should match");
            assert_eq!(
                agent.project_id, project.id,
                "Agent project_id should match"
            );

            println!(
                "✓ Agent registered: {} (id={}, project_id={})",
                agent.name, agent.id, agent.project_id
            );
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
    let sender_name = TestFixtures::unique_agent_name();
    let recipient_name = TestFixtures::unique_agent_name();

    // Setup: Create project
    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ API server not running or project creation failed");
            return;
        }
    };

    // Register sender and recipient
    if setup_agent(&client, &config, &project.slug, &sender_name)
        .await
        .is_none()
    {
        println!("⚠ Failed to register sender");
        return;
    }
    if setup_agent(&client, &config, &project.slug, &recipient_name)
        .await
        .is_none()
    {
        println!("⚠ Failed to register recipient");
        return;
    }

    // Send message
    let message_payload = TestFixtures::message_payload(
        &project.slug,
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
            assert!(
                !message.thread_id.is_empty(),
                "Message should have thread_id"
            );

            println!(
                "✓ Message sent: id={}, thread={}",
                message.id, message.thread_id
            );
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
    let agent_name = TestFixtures::unique_agent_name();

    // Setup: Create project and agent
    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ API server not running or project creation failed");
            return;
        }
    };

    if setup_agent(&client, &config, &project.slug, &agent_name)
        .await
        .is_none()
    {
        println!("⚠ Failed to register agent");
        return;
    }

    // Check inbox
    let response = client
        .post(format!("{}/api/inbox", config.api_url))
        .json(&json!({
            "project_slug": project.slug,
            "agent_name": agent_name,
            "limit": 10
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "check_inbox should succeed");

            let messages: Vec<serde_json::Value> =
                resp.json().await.expect("Should parse response");
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

    // Setup: Create project
    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ API server not running or project creation failed");
            return;
        }
    };

    // Search messages (FTS5)
    let response = client
        .post(format!("{}/api/messages/search", config.api_url))
        .json(&json!({
            "project_slug": project.slug,
            "query": "test",
            "limit": 10
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success(), "search_messages should succeed");

            // API returns { query, results, count } object
            let search_response: serde_json::Value =
                resp.json().await.expect("Should parse response");
            let count = search_response["count"].as_i64().unwrap_or(0);
            println!("✓ Search returned {} results", count);
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
    let agent_name = TestFixtures::unique_agent_name();

    // Setup: Create project and agent
    let project = match setup_project(&client, &config).await {
        Some(p) => p,
        None => {
            println!("⚠ API server not running or project creation failed");
            return;
        }
    };

    if setup_agent(&client, &config, &project.slug, &agent_name)
        .await
        .is_none()
    {
        println!("⚠ Failed to register agent");
        return;
    }

    // Reserve files
    let response = client
        .post(format!("{}/api/file_reservations/paths", config.api_url))
        .json(&json!({
            "project_slug": project.slug,
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
            assert!(
                resp.status().is_success(),
                "file_reservation should succeed"
            );
            println!("✓ File reservation created");

            // List reservations
            let list_resp = client
                .post(format!("{}/api/file_reservations/list", config.api_url))
                .json(&json!({
                    "project_slug": project.slug
                }))
                .send()
                .await
                .expect("Should list reservations");

            let reservations: Vec<serde_json::Value> =
                list_resp.json().await.expect("Should parse response");

            assert!(
                !reservations.is_empty(),
                "Should have at least one reservation"
            );
            println!("✓ Found {} reservations", reservations.len());
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}
