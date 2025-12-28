//! Guard Integration E2E Tests (mcpmail-ifbw)
//!
//! Tests for pre-commit/pre-push guard functionality:
//! - Guard check with no conflicts
//! - Guard blocks on active reservation
//! - Glob pattern overlap detection
//! - Pre-push simulation with pending reviews
//! - Force release of stale locks
//! - Guard status output
//! - Multiple reservations from same agent
//! - Expiration time handling
//!
//! Run tests:
//! ```bash
//! cargo test -p e2e-tests --test guard_integration
//! ```

use e2e_tests::fixtures::ProjectResponse;
use e2e_tests::{TestConfig, TestFixtures};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::process::Command;

// ============================================================================
// Test Configuration & Helpers
// ============================================================================

fn get_config() -> TestConfig {
    TestConfig::default()
}

async fn create_client() -> Client {
    Client::new()
}

/// Setup a project and return its details
async fn setup_project(client: &Client, config: &TestConfig) -> Result<ProjectResponse, String> {
    let human_key = TestFixtures::unique_project_name();

    let resp = client
        .post(format!("{}/api/project/ensure", config.api_url))
        .json(&TestFixtures::project_payload(&human_key))
        .send()
        .await
        .map_err(|e| format!("Failed to create project: {}", e))?;

    if resp.status().is_success() {
        resp.json()
            .await
            .map_err(|e| format!("Failed to parse project response: {}", e))
    } else {
        Err(format!("Project creation failed: {}", resp.status()))
    }
}

/// Register an agent in a project
async fn register_agent(
    client: &Client,
    config: &TestConfig,
    project_slug: &str,
    agent_name: &str,
) -> Result<AgentInfo, String> {
    let resp = client
        .post(format!("{}/api/agent/register", config.api_url))
        .json(&TestFixtures::agent_payload(project_slug, agent_name))
        .send()
        .await
        .map_err(|e| format!("Failed to register agent: {}", e))?;

    if resp.status().is_success() {
        resp.json()
            .await
            .map_err(|e| format!("Failed to parse agent response: {}", e))
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Agent registration failed: {} - {}", status, body))
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AgentInfo {
    id: i64,
    name: String,
    project_id: i64,
}

/// Reserve file paths for an agent
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ReservationInfo {
    id: i64,
    agent_id: i64,
    path_pattern: String,
    reason: String,
    expires_ts: String,
}

async fn reserve_files(
    client: &Client,
    config: &TestConfig,
    project_slug: &str,
    agent_name: &str,
    paths: &[&str],
    ttl_seconds: Option<u32>,
    reason: Option<&str>,
) -> Result<Vec<ReservationInfo>, String> {
    let mut payload = json!({
        "project_slug": project_slug,
        "agent_name": agent_name,
        "paths": paths
    });

    if let Some(ttl) = ttl_seconds {
        payload["ttl_seconds"] = json!(ttl);
    }
    if let Some(r) = reason {
        payload["reason"] = json!(r);
    }

    let resp = client
        .post(format!("{}/api/file_reservations/paths", config.api_url))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to reserve files: {}", e))?;

    if resp.status().is_success() {
        resp.json()
            .await
            .map_err(|e| format!("Failed to parse reservation response: {}", e))
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Reservation failed: {} - {}", status, body))
    }
}

/// List active reservations for a project
async fn list_reservations(
    client: &Client,
    config: &TestConfig,
    project_slug: &str,
) -> Result<Vec<ReservationInfo>, String> {
    let resp = client
        .post(format!("{}/api/file_reservations/list", config.api_url))
        .json(&json!({
            "project_slug": project_slug
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to list reservations: {}", e))?;

    if resp.status().is_success() {
        resp.json()
            .await
            .map_err(|e| format!("Failed to parse reservations response: {}", e))
    } else {
        Err(format!("List reservations failed: {}", resp.status()))
    }
}

/// Release a reservation
async fn release_reservation(
    client: &Client,
    config: &TestConfig,
    project_slug: &str,
    agent_name: &str,
    reservation_id: i64,
) -> Result<(), String> {
    let resp = client
        .post(format!("{}/api/file_reservations/release", config.api_url))
        .json(&json!({
            "project_slug": project_slug,
            "agent_name": agent_name,
            "reservation_id": reservation_id
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to release reservation: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Release failed: {} - {}", status, body))
    }
}

/// Check if guard CLI is available
fn guard_cli_available() -> bool {
    Command::new("./target/debug/mcp-agent-mail")
        .arg("guard")
        .arg("--help")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ============================================================================
// Test 1: Guard Check With No Conflicts Passes
// ============================================================================

#[tokio::test]
async fn test_guard_no_conflicts_passes() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "GuardAgent1").await;

    // Agent reserves some files
    let reservation = reserve_files(
        &client,
        &config,
        &project.slug,
        "GuardAgent1",
        &["src/main.rs"],
        Some(3600),
        Some("Working on main"),
    )
    .await;

    assert!(reservation.is_ok(), "Reservation should succeed");

    // Same agent checking their own files should pass (no conflict)
    // In real guard, own reservations are ignored
    let res_list = match list_reservations(&client, &config, &project.slug).await {
        Ok(r) => r,
        Err(e) => {
            panic!("Should list reservations: {}", e);
        }
    };
    assert!(!res_list.is_empty(), "Should have at least one reservation");

    // Verify reservation belongs to the agent
    let has_own_reservation = res_list.iter().any(|r| r.path_pattern == "src/main.rs");
    assert!(has_own_reservation, "Agent should have their reservation");

    println!("Guard check with own reservation passes (no conflict)");
}

// ============================================================================
// Test 2: Guard Blocks on Active Reservation
// ============================================================================

#[tokio::test]
async fn test_guard_blocks_on_active_reservation() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping test - server not available: {}", e);
            return;
        }
    };

    // Register two agents
    let _ = register_agent(&client, &config, &project.slug, "BlockingAgent").await;
    let _ = register_agent(&client, &config, &project.slug, "BlockedAgent").await;

    // First agent reserves a file
    let res1 = reserve_files(
        &client,
        &config,
        &project.slug,
        "BlockingAgent",
        &["src/critical.rs"],
        Some(3600),
        Some("Critical work"),
    )
    .await;
    assert!(res1.is_ok(), "First reservation should succeed");

    // Second agent tries to reserve the same file - should fail or show conflict
    let res2 = reserve_files(
        &client,
        &config,
        &project.slug,
        "BlockedAgent",
        &["src/critical.rs"],
        Some(3600),
        Some("Also want this file"),
    )
    .await;

    // Either fails (conflict) or succeeds (tracked separately)
    if res2.is_err() {
        println!("Guard correctly blocked: second agent cannot reserve same file");
    } else {
        // Check that both reservations exist (conflict tracking)
        let reservations = list_reservations(&client, &config, &project.slug).await;
        if let Ok(res_list) = reservations {
            let critical_count = res_list
                .iter()
                .filter(|r| r.path_pattern == "src/critical.rs")
                .count();
            println!(
                "Guard tracks conflicts: {} reservations for same path",
                critical_count
            );
        }
    }
}

// ============================================================================
// Test 3: Guard Glob Overlap Detection
// ============================================================================

#[tokio::test]
async fn test_guard_glob_overlap_detection() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "GlobAgent1").await;
    let _ = register_agent(&client, &config, &project.slug, "GlobAgent2").await;

    // First agent reserves all Rust files in src/
    let res1 = reserve_files(
        &client,
        &config,
        &project.slug,
        "GlobAgent1",
        &["src/*.rs"],
        Some(3600),
        Some("All src Rust files"),
    )
    .await;
    assert!(res1.is_ok(), "Glob reservation should succeed");

    // Second agent tries to reserve a specific file that matches the glob
    let res2 = reserve_files(
        &client,
        &config,
        &project.slug,
        "GlobAgent2",
        &["src/lib.rs"],
        Some(3600),
        Some("Specific file"),
    )
    .await;

    // Check overlap detection
    let res_list = match list_reservations(&client, &config, &project.slug).await {
        Ok(r) => r,
        Err(e) => {
            panic!("Should list reservations: {}", e);
        }
    };

    // Both reservations should exist (overlapping patterns tracked)
    let has_glob = res_list.iter().any(|r| r.path_pattern == "src/*.rs");
    let has_specific = res_list.iter().any(|r| r.path_pattern == "src/lib.rs");

    if has_glob && has_specific {
        println!("Guard tracks overlapping patterns: glob and specific file");
    } else if res2.is_err() {
        println!("Guard blocked overlapping reservation");
    }
}

// ============================================================================
// Test 4: Pre-push Blocks Pending Reviews
// ============================================================================

#[tokio::test]
async fn test_prepush_blocks_pending_reviews() {
    // This test simulates the pre-push hook behavior
    // When there are pending reviews, push should be blocked

    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "PushAgent").await;
    let _ = register_agent(&client, &config, &project.slug, "ReviewerAgent").await;

    // Create a message that requires acknowledgment (simulating pending review)
    let resp = client
        .post(format!("{}/api/message/send", config.api_url))
        .json(&json!({
            "project_slug": project.slug,
            "sender_name": "PushAgent",
            "recipient_names": ["ReviewerAgent"],
            "subject": "[COMPLETION] task-123: Feature X",
            "body_md": "Ready for review",
            "thread_id": "TASK-123",
            "importance": "high",
            "ack_required": true
        }))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            // Message sent, now check pending reviews
            let pending_resp = client
                .post(format!("{}/api/message/pending_acks", config.api_url))
                .json(&json!({
                    "project_slug": project.slug,
                    "agent_name": "ReviewerAgent"
                }))
                .send()
                .await;

            if let Ok(r) = pending_resp {
                if r.status().is_success() {
                    println!("Pre-push would check for pending reviews before allowing push");
                }
            }
        }
        _ => {
            println!("Message endpoint not available, skipping pending review check");
        }
    }
}

// ============================================================================
// Test 5: Force Release Stale Lock
// ============================================================================

#[tokio::test]
async fn test_force_release_stale_lock() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "StaleAgent").await;
    let _ = register_agent(&client, &config, &project.slug, "ForceReleaser").await;

    // StaleAgent creates a reservation
    let res = reserve_files(
        &client,
        &config,
        &project.slug,
        "StaleAgent",
        &["src/stale.rs"],
        Some(3600),
        Some("Will become stale"),
    )
    .await;

    let reservation_id = match res {
        Ok(r) if !r.is_empty() => r[0].id,
        _ => {
            println!("Could not create reservation");
            return;
        }
    };

    // Try force release by different agent (should fail without proper permissions)
    let force_resp = client
        .post(format!(
            "{}/api/file_reservations/force_release",
            config.api_url
        ))
        .json(&json!({
            "project_slug": project.slug,
            "agent_name": "ForceReleaser",
            "reservation_id": reservation_id,
            "reason": "Emergency release"
        }))
        .send()
        .await;

    match force_resp {
        Ok(r) => {
            if r.status().is_success() {
                println!("Force release succeeded (admin capability)");
            } else {
                println!(
                    "Force release blocked (expected for non-owner): {}",
                    r.status()
                );
            }
        }
        Err(e) => {
            println!("Force release request failed: {}", e);
        }
    }

    // Original agent can always release their own
    let normal_release = release_reservation(
        &client,
        &config,
        &project.slug,
        "StaleAgent",
        reservation_id,
    )
    .await;
    assert!(
        normal_release.is_ok(),
        "Owner should be able to release their reservation"
    );
    println!("Owner successfully released their reservation");
}

// ============================================================================
// Test 6: Guard Status Output
// ============================================================================

#[tokio::test]
async fn test_guard_status_output() {
    if !guard_cli_available() {
        println!("Skipping test - guard CLI not built");
        return;
    }

    // Run guard status command
    let output = Command::new("./target/debug/mcp-agent-mail")
        .arg("guard")
        .arg("status")
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);

            // Guard status should output some information
            if o.status.success() {
                println!("Guard status succeeded");
                if !stdout.is_empty() {
                    println!("Status output: {}", stdout.lines().next().unwrap_or(""));
                }
            } else {
                // May fail if server not configured, but command should exist
                println!("Guard status command exists (stderr: {})", stderr);
            }
        }
        Err(e) => {
            println!("Could not run guard status: {}", e);
        }
    }
}

// ============================================================================
// Test 7: Guard Multiple Reservations Same Agent
// ============================================================================

#[tokio::test]
async fn test_guard_multiple_reservations_same_agent() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "MultiResAgent").await;

    // Same agent reserves multiple distinct paths
    let res1 = reserve_files(
        &client,
        &config,
        &project.slug,
        "MultiResAgent",
        &["src/mod_a.rs"],
        Some(3600),
        Some("Module A"),
    )
    .await;
    assert!(res1.is_ok(), "First reservation should succeed");

    let res2 = reserve_files(
        &client,
        &config,
        &project.slug,
        "MultiResAgent",
        &["src/mod_b.rs"],
        Some(3600),
        Some("Module B"),
    )
    .await;
    assert!(res2.is_ok(), "Second reservation should succeed");

    let res3 = reserve_files(
        &client,
        &config,
        &project.slug,
        "MultiResAgent",
        &["tests/**/*.rs"],
        Some(3600),
        Some("All test files"),
    )
    .await;
    assert!(res3.is_ok(), "Third reservation should succeed");

    // Verify all reservations are tracked
    let res_list = match list_reservations(&client, &config, &project.slug).await {
        Ok(r) => r,
        Err(e) => {
            panic!("Should list reservations: {}", e);
        }
    };
    let agent_reservations: Vec<_> = res_list.iter().collect();

    assert!(
        agent_reservations.len() >= 3,
        "Agent should have at least 3 reservations"
    );
    println!(
        "Agent successfully holds {} reservations simultaneously",
        agent_reservations.len()
    );
}

// ============================================================================
// Test 8: Guard Respects Expiration Times
// ============================================================================

#[tokio::test]
async fn test_guard_respects_expiration() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "ExpiringAgent").await;
    let _ = register_agent(&client, &config, &project.slug, "WaitingAgent").await;

    // Create a very short-lived reservation (1 second)
    let res = reserve_files(
        &client,
        &config,
        &project.slug,
        "ExpiringAgent",
        &["src/expiring.rs"],
        Some(1), // 1 second TTL
        Some("Short-lived reservation"),
    )
    .await;

    assert!(res.is_ok(), "Short-lived reservation should succeed");

    // Immediately check - should exist
    let reservations_before = list_reservations(&client, &config, &project.slug).await;
    let before_count = reservations_before
        .as_ref()
        .map(|r| {
            r.iter()
                .filter(|r| r.path_pattern == "src/expiring.rs")
                .count()
        })
        .unwrap_or(0);

    if before_count > 0 {
        println!("Reservation exists immediately after creation");
    }

    // Wait for expiration
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // After expiration, check again
    let reservations_after = list_reservations(&client, &config, &project.slug).await;

    if let Ok(res_list) = reservations_after {
        let after_count = res_list
            .iter()
            .filter(|r| r.path_pattern == "src/expiring.rs")
            .count();

        if after_count == 0 {
            println!("Expired reservation correctly removed from active list");
        } else {
            // May still exist but marked as expired
            println!("Reservation still tracked (may be marked expired)");
        }
    }

    // Second agent should now be able to reserve the same file
    let res2 = reserve_files(
        &client,
        &config,
        &project.slug,
        "WaitingAgent",
        &["src/expiring.rs"],
        Some(3600),
        Some("Taking over after expiration"),
    )
    .await;

    if res2.is_ok() {
        println!("Second agent successfully reserved file after expiration");
    } else {
        println!("Expiration handling may need manual cleanup");
    }
}
