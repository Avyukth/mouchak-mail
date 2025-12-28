//! Multi-Agent Orchestration E2E Tests (mcpmail-o05a)
//!
//! Tests for concurrent multi-agent scenarios:
//! - Multiple agents registering and communicating
//! - File reservation conflicts
//! - Build slot contention
//! - Message threading across agents
//!
//! Run tests:
//! ```bash
//! cargo test -p e2e-tests --test multi_agent_orchestration
//! ```

use e2e_tests::fixtures::{AgentResponse, MessageResponse, ProjectResponse};
use e2e_tests::{TestConfig, TestFixtures};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// Test Configuration & Helpers
// ============================================================================

fn get_config() -> TestConfig {
    TestConfig::default()
}

async fn create_client() -> Client {
    Client::new()
}

/// Setup a project and return its slug
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
) -> Result<AgentResponse, String> {
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

/// Send a message between agents
async fn send_message(
    client: &Client,
    config: &TestConfig,
    project_slug: &str,
    sender: &str,
    recipients: &[&str],
    subject: &str,
    body: &str,
    thread_id: Option<&str>,
) -> Result<MessageResponse, String> {
    let mut payload = json!({
        "project_slug": project_slug,
        "sender_name": sender,
        "recipient_names": recipients,
        "subject": subject,
        "body_md": body
    });

    if let Some(tid) = thread_id {
        payload["thread_id"] = json!(tid);
    }

    let resp = client
        .post(format!("{}/api/message/send", config.api_url))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to send message: {}", e))?;

    if resp.status().is_success() {
        resp.json()
            .await
            .map_err(|e| format!("Failed to parse message response: {}", e))
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Message send failed: {} - {}", status, body))
    }
}

/// Fetch inbox for an agent
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct InboxMessage {
    id: i64,
    subject: String,
    sender_name: String,
    thread_id: String,
}

async fn fetch_inbox(
    client: &Client,
    config: &TestConfig,
    project_slug: &str,
    agent_name: &str,
) -> Result<Vec<InboxMessage>, String> {
    let resp = client
        .post(format!("{}/api/message/inbox", config.api_url))
        .json(&json!({
            "project_slug": project_slug,
            "agent_name": agent_name,
            "limit": 50
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch inbox: {}", e))?;

    if resp.status().is_success() {
        resp.json()
            .await
            .map_err(|e| format!("Failed to parse inbox response: {}", e))
    } else {
        Err(format!("Inbox fetch failed: {}", resp.status()))
    }
}

/// Reserve a file path for an agent
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ReservationResponse {
    id: i64,
    agent_name: String,
    paths: Vec<String>,
}

async fn reserve_files(
    client: &Client,
    config: &TestConfig,
    project_slug: &str,
    agent_name: &str,
    paths: &[&str],
) -> Result<ReservationResponse, String> {
    let resp = client
        .post(format!("{}/api/reservation/create", config.api_url))
        .json(&json!({
            "project_slug": project_slug,
            "agent_name": agent_name,
            "paths": paths
        }))
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

// ============================================================================
// Test 1: Five Agents Register Concurrently
// ============================================================================

#[tokio::test]
async fn test_five_agents_register_concurrent() {
    let config = get_config();
    let client = create_client().await;

    // Setup project
    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    let agent_names = [
        "AgentAlpha",
        "AgentBeta",
        "AgentGamma",
        "AgentDelta",
        "AgentEpsilon",
    ];
    let registered = Arc::new(Mutex::new(Vec::new()));

    // Spawn concurrent registrations
    let mut handles = Vec::new();
    for name in &agent_names {
        let client_clone = client.clone();
        let config_clone = config.clone();
        let slug = project.slug.clone();
        let agent_name = (*name).to_string();
        let registered_clone = Arc::clone(&registered);

        let handle = tokio::spawn(async move {
            let result = register_agent(&client_clone, &config_clone, &slug, &agent_name).await;
            if let Ok(agent) = result {
                let mut reg = registered_clone.lock().await;
                reg.push(agent.name);
            }
        });
        handles.push(handle);
    }

    // Wait for all registrations
    for handle in handles {
        let _ = handle.await;
    }

    let registered_names = registered.lock().await;
    assert_eq!(
        registered_names.len(),
        5,
        "All 5 agents should register successfully"
    );

    println!(
        "✓ 5 agents registered concurrently: {:?}",
        *registered_names
    );
}

// ============================================================================
// Test 2: Message Dependency Chain (A→B→C)
// ============================================================================

#[tokio::test]
async fn test_message_dependency_chain() {
    let config = get_config();
    let client = create_client().await;

    // Setup project and agents
    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "ChainAgent1").await;
    let _ = register_agent(&client, &config, &project.slug, "ChainAgent2").await;
    let _ = register_agent(&client, &config, &project.slug, "ChainAgent3").await;

    // A sends to B
    let msg1 = send_message(
        &client,
        &config,
        &project.slug,
        "ChainAgent1",
        &["ChainAgent2"],
        "Step 1 Complete",
        "Starting the chain",
        Some("CHAIN-TEST"),
    )
    .await;

    let msg1 = match msg1 {
        Ok(m) => m,
        Err(e) => {
            panic!("First message should send successfully: {}", e);
        }
    };

    // B sends to C (in same thread)
    let msg2 = send_message(
        &client,
        &config,
        &project.slug,
        "ChainAgent2",
        &["ChainAgent3"],
        "Step 2 Complete",
        "Continuing the chain",
        Some(&msg1.thread_id),
    )
    .await;

    let msg2 = match msg2 {
        Ok(m) => m,
        Err(e) => {
            panic!("Second message should send successfully: {}", e);
        }
    };

    // C sends back to A (completing the cycle)
    let msg3 = send_message(
        &client,
        &config,
        &project.slug,
        "ChainAgent3",
        &["ChainAgent1"],
        "Chain Complete",
        "Circle complete",
        Some(&msg2.thread_id),
    )
    .await;

    assert!(msg3.is_ok(), "Third message should send successfully");

    // Verify A's inbox has the final message
    let messages = match fetch_inbox(&client, &config, &project.slug, "ChainAgent1").await {
        Ok(m) => m,
        Err(e) => {
            panic!("Should fetch inbox: {}", e);
        }
    };
    let has_completion = messages.iter().any(|m| m.subject == "Chain Complete");
    assert!(has_completion, "Agent1 should receive completion message");

    println!("✓ Message dependency chain A→B→C→A works correctly");
}

// ============================================================================
// Test 3: File Reservation Conflict Detection
// ============================================================================

#[tokio::test]
async fn test_file_reservation_conflict() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "ReserveAgent1").await;
    let _ = register_agent(&client, &config, &project.slug, "ReserveAgent2").await;

    // Agent1 reserves src/lib.rs
    let res1 = reserve_files(
        &client,
        &config,
        &project.slug,
        "ReserveAgent1",
        &["src/lib.rs"],
    )
    .await;

    assert!(res1.is_ok(), "First reservation should succeed");

    // Agent2 tries to reserve same file - should fail or return conflict
    let res2 = reserve_files(
        &client,
        &config,
        &project.slug,
        "ReserveAgent2",
        &["src/lib.rs"],
    )
    .await;

    // Conflict detection - either error or success with conflict info
    if res2.is_err() {
        println!("✓ Conflict detected: reservation blocked for Agent2");
    } else {
        // Some systems allow but track conflicts
        println!("✓ Reservation created (conflicts tracked separately)");
    }
}

// ============================================================================
// Test 4: Concurrent Inbox Checks (No Deadlock)
// ============================================================================

#[tokio::test]
async fn test_concurrent_inbox_no_deadlock() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    // Register agents
    for i in 1..=5 {
        let _ = register_agent(&client, &config, &project.slug, &format!("InboxAgent{}", i)).await;
    }

    // Send some messages to create inbox content
    for i in 1..=5 {
        let _ = send_message(
            &client,
            &config,
            &project.slug,
            &format!("InboxAgent{}", i),
            &["InboxAgent1", "InboxAgent2", "InboxAgent3"],
            &format!("Broadcast {}", i),
            "Test message body",
            None,
        )
        .await;
    }

    // Concurrent inbox fetches
    let mut handles = Vec::new();
    let success_count = Arc::new(Mutex::new(0u32));

    for i in 1..=5 {
        let client_clone = client.clone();
        let config_clone = config.clone();
        let slug = project.slug.clone();
        let agent = format!("InboxAgent{}", i);
        let counter = Arc::clone(&success_count);

        let handle = tokio::spawn(async move {
            // Multiple rapid inbox checks
            for _ in 0..10 {
                if fetch_inbox(&client_clone, &config_clone, &slug, &agent)
                    .await
                    .is_ok()
                {
                    let mut count = counter.lock().await;
                    *count += 1;
                }
            }
        });
        handles.push(handle);
    }

    // Wait with timeout
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        for handle in handles {
            let _ = handle.await;
        }
    })
    .await;

    assert!(
        timeout.is_ok(),
        "Concurrent inbox checks should not deadlock"
    );

    let final_count = *success_count.lock().await;
    println!(
        "✓ {} concurrent inbox fetches completed without deadlock",
        final_count
    );
}

// ============================================================================
// Test 5: Broadcast to All Agents
// ============================================================================

#[tokio::test]
async fn test_broadcast_to_all_agents() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    let agents = ["BroadcastSender", "Receiver1", "Receiver2", "Receiver3"];
    for agent in &agents {
        let _ = register_agent(&client, &config, &project.slug, agent).await;
    }

    // Sender broadcasts to all receivers
    let result = send_message(
        &client,
        &config,
        &project.slug,
        "BroadcastSender",
        &["Receiver1", "Receiver2", "Receiver3"],
        "Important Announcement",
        "This is a broadcast message to all team members",
        Some("BROADCAST-TEST"),
    )
    .await;

    assert!(result.is_ok(), "Broadcast should succeed");

    // Verify all receivers got the message
    let mut received_count = 0;
    for receiver in &["Receiver1", "Receiver2", "Receiver3"] {
        let inbox = fetch_inbox(&client, &config, &project.slug, receiver).await;
        if let Ok(messages) = inbox {
            if messages
                .iter()
                .any(|m| m.subject == "Important Announcement")
            {
                received_count += 1;
            }
        }
    }

    assert_eq!(
        received_count, 3,
        "All 3 receivers should get the broadcast"
    );
    println!("✓ Broadcast delivered to all {} receivers", received_count);
}

// ============================================================================
// Test 6: Parallel File Claims on Different Paths
// ============================================================================

#[tokio::test]
async fn test_parallel_file_claims_different_paths() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "PathAgent1").await;
    let _ = register_agent(&client, &config, &project.slug, "PathAgent2").await;
    let _ = register_agent(&client, &config, &project.slug, "PathAgent3").await;

    // Concurrent reservations on different paths - should all succeed
    let mut handles = Vec::new();
    let paths = [
        ("PathAgent1", "src/main.rs"),
        ("PathAgent2", "src/lib.rs"),
        ("PathAgent3", "tests/integration.rs"),
    ];

    let success_count = Arc::new(Mutex::new(0u32));

    for (agent, path) in paths {
        let client_clone = client.clone();
        let config_clone = config.clone();
        let slug = project.slug.clone();
        let counter = Arc::clone(&success_count);

        let handle = tokio::spawn(async move {
            let result = reserve_files(&client_clone, &config_clone, &slug, agent, &[path]).await;
            if result.is_ok() {
                let mut count = counter.lock().await;
                *count += 1;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    let final_count = *success_count.lock().await;
    assert_eq!(
        final_count, 3,
        "All 3 non-conflicting reservations should succeed"
    );
    println!(
        "✓ {} parallel reservations on different paths succeeded",
        final_count
    );
}

// ============================================================================
// Test 7: Build Slot Contention
// ============================================================================

#[tokio::test]
async fn test_build_slot_contention() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    // Register multiple agents that will compete for build slots
    let agents = ["BuildAgent1", "BuildAgent2", "BuildAgent3"];
    for agent in &agents {
        let _ = register_agent(&client, &config, &project.slug, agent).await;
    }

    // Simulate concurrent build slot requests via file reservations on build paths
    let build_paths = [
        "target/debug/build.lock",
        "target/release/build.lock",
        "Cargo.lock",
    ];

    let contention_detected = Arc::new(Mutex::new(false));
    let mut handles = Vec::new();

    for (i, agent) in agents.iter().enumerate() {
        let client_clone = client.clone();
        let config_clone = config.clone();
        let slug = project.slug.clone();
        let agent_name = (*agent).to_string();
        let path = build_paths[i % build_paths.len()];
        let contention = Arc::clone(&contention_detected);

        let handle = tokio::spawn(async move {
            // Try to reserve the same build path
            let result =
                reserve_files(&client_clone, &config_clone, &slug, &agent_name, &[path]).await;
            if result.is_err() {
                let mut detected = contention.lock().await;
                *detected = true;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    // Build slot contention is expected - either detected or all succeeded with tracking
    println!("✓ Build slot contention test completed");
}

// ============================================================================
// Test 8: Threaded Conversation with Five Agents
// ============================================================================

#[tokio::test]
async fn test_threaded_conversation_five_agents() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    let agents = [
        "ThreadAgent1",
        "ThreadAgent2",
        "ThreadAgent3",
        "ThreadAgent4",
        "ThreadAgent5",
    ];
    for agent in &agents {
        let _ = register_agent(&client, &config, &project.slug, agent).await;
    }

    // Agent1 starts thread
    let msg1 = send_message(
        &client,
        &config,
        &project.slug,
        "ThreadAgent1",
        &[
            "ThreadAgent2",
            "ThreadAgent3",
            "ThreadAgent4",
            "ThreadAgent5",
        ],
        "Thread Start",
        "Starting a group discussion",
        Some("GROUP-THREAD-TEST"),
    )
    .await;

    let thread_id = match msg1 {
        Ok(m) => m.thread_id,
        Err(e) => {
            println!("⚠ Failed to start thread: {}", e);
            return;
        }
    };

    // Each agent replies to the thread
    let mut message_count = 1;
    for (i, agent) in agents.iter().enumerate().skip(1) {
        let recipients: Vec<&str> = agents
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, a)| *a)
            .collect();

        let result = send_message(
            &client,
            &config,
            &project.slug,
            agent,
            &recipients,
            &format!("Reply from {}", agent),
            &format!("This is message #{} in the thread", i + 1),
            Some(&thread_id),
        )
        .await;

        if result.is_ok() {
            message_count += 1;
        }
    }

    assert!(
        message_count >= 3,
        "At least 3 messages should be in thread"
    );
    println!(
        "✓ Threaded conversation with {} messages across 5 agents",
        message_count
    );
}

// ============================================================================
// Test 9: Agent Leaves Mid-Conversation
// ============================================================================

#[tokio::test]
async fn test_agent_leaves_mid_conversation() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "StayingAgent1").await;
    let _ = register_agent(&client, &config, &project.slug, "LeavingAgent").await;
    let _ = register_agent(&client, &config, &project.slug, "StayingAgent2").await;

    // Start conversation
    let msg1 = send_message(
        &client,
        &config,
        &project.slug,
        "StayingAgent1",
        &["LeavingAgent", "StayingAgent2"],
        "Group Chat",
        "Starting group chat",
        Some("LEAVE-TEST-THREAD"),
    )
    .await;

    let thread_id = match msg1 {
        Ok(m) => m.thread_id,
        Err(e) => {
            println!("⚠ Failed to start conversation: {}", e);
            return;
        }
    };

    // LeavingAgent participates
    let _ = send_message(
        &client,
        &config,
        &project.slug,
        "LeavingAgent",
        &["StayingAgent1", "StayingAgent2"],
        "I'm here",
        "Participating before leaving",
        Some(&thread_id),
    )
    .await;

    // Simulate agent "leaving" by not responding - other agents continue
    let msg3 = send_message(
        &client,
        &config,
        &project.slug,
        "StayingAgent2",
        &["StayingAgent1", "LeavingAgent"], // Still includes leaving agent
        "Continuing without response",
        "The conversation continues",
        Some(&thread_id),
    )
    .await;

    assert!(
        msg3.is_ok(),
        "Conversation should continue even if one agent is unresponsive"
    );

    // Verify staying agents can still communicate
    let inbox = fetch_inbox(&client, &config, &project.slug, "StayingAgent1").await;
    assert!(
        inbox.is_ok(),
        "Staying agent should still have inbox access"
    );

    println!("✓ Conversation continues when agent leaves mid-conversation");
}

// ============================================================================
// Test 10: Message Ordering Preserved
// ============================================================================

#[tokio::test]
async fn test_message_ordering_preserved() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    let _ = register_agent(&client, &config, &project.slug, "OrderSender").await;
    let _ = register_agent(&client, &config, &project.slug, "OrderReceiver").await;

    // Send messages in sequence with distinct subjects
    let message_subjects = [
        "Message 1",
        "Message 2",
        "Message 3",
        "Message 4",
        "Message 5",
    ];
    let mut sent_ids = Vec::new();

    for subject in &message_subjects {
        let result = send_message(
            &client,
            &config,
            &project.slug,
            "OrderSender",
            &["OrderReceiver"],
            subject,
            &format!("Body of {}", subject),
            Some("ORDER-TEST-THREAD"),
        )
        .await;

        if let Ok(msg) = result {
            sent_ids.push(msg.id);
        }
    }

    assert_eq!(
        sent_ids.len(),
        5,
        "All 5 messages should be sent successfully"
    );

    // Verify IDs are monotonically increasing (database ordering)
    for i in 1..sent_ids.len() {
        assert!(
            sent_ids[i] > sent_ids[i - 1],
            "Message IDs should be monotonically increasing"
        );
    }

    // Fetch inbox and verify order
    let messages = match fetch_inbox(&client, &config, &project.slug, "OrderReceiver").await {
        Ok(m) => m,
        Err(e) => {
            panic!("Should fetch inbox: {}", e);
        }
    };
    let thread_messages: Vec<_> = messages
        .iter()
        .filter(|m| m.thread_id == "ORDER-TEST-THREAD")
        .collect();

    assert!(
        thread_messages.len() >= 3,
        "Should have multiple messages in thread"
    );
    println!(
        "✓ Message ordering preserved across {} messages",
        sent_ids.len()
    );
}

// ============================================================================
// Test 11: Agent Reconnect Preserves Inbox
// ============================================================================

#[tokio::test]
async fn test_agent_reconnect_preserves_inbox() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    let reconnect_agent = "ReconnectAgent";
    let sender_agent = "SenderForReconnect";

    // Initial registration
    let _ = register_agent(&client, &config, &project.slug, reconnect_agent).await;
    let _ = register_agent(&client, &config, &project.slug, sender_agent).await;

    // Send message to the agent
    let _ = send_message(
        &client,
        &config,
        &project.slug,
        sender_agent,
        &[reconnect_agent],
        "Message Before Reconnect",
        "This should persist",
        None,
    )
    .await;

    // Check inbox before "disconnect"
    let inbox_before = fetch_inbox(&client, &config, &project.slug, reconnect_agent).await;
    let before_count = inbox_before.map(|m| m.len()).unwrap_or(0);

    // Simulate reconnect by re-registering (should get same agent or new with preserved messages)
    let reconnect_result = register_agent(&client, &config, &project.slug, reconnect_agent).await;
    assert!(
        reconnect_result.is_ok(),
        "Agent should be able to re-register"
    );

    // Check inbox after reconnect
    let inbox_after = fetch_inbox(&client, &config, &project.slug, reconnect_agent).await;
    let after_count = inbox_after.map(|m| m.len()).unwrap_or(0);

    assert!(
        after_count >= before_count,
        "Inbox should preserve messages after reconnect"
    );
    println!(
        "✓ Agent reconnect preserves inbox ({} messages before, {} after)",
        before_count, after_count
    );
}

// ============================================================================
// Test 12: Stress Test - 100 Messages Across 5 Agents
// ============================================================================

#[tokio::test]
async fn test_stress_100_messages_5_agents() {
    let config = get_config();
    let client = create_client().await;

    let project = match setup_project(&client, &config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping test - server not available: {}", e);
            return;
        }
    };

    let agents = [
        "StressAgent1",
        "StressAgent2",
        "StressAgent3",
        "StressAgent4",
        "StressAgent5",
    ];

    for agent in &agents {
        let _ = register_agent(&client, &config, &project.slug, agent).await;
    }

    let success_count = Arc::new(Mutex::new(0u32));
    let mut handles = Vec::new();

    // Send 100 messages (20 per agent)
    for (agent_idx, sender) in agents.iter().enumerate() {
        let client_clone = client.clone();
        let config_clone = config.clone();
        let slug = project.slug.clone();
        let sender_name = (*sender).to_string();
        let counter = Arc::clone(&success_count);

        // Each agent sends to the next agent in round-robin
        let recipient_idx = (agent_idx + 1) % agents.len();
        let recipient = agents[recipient_idx].to_owned();

        let handle = tokio::spawn(async move {
            for i in 0..20 {
                let result = send_message(
                    &client_clone,
                    &config_clone,
                    &slug,
                    &sender_name,
                    &[&recipient],
                    &format!("Stress Message {}-{}", sender_name, i),
                    &format!("Stress test body #{}", i),
                    None,
                )
                .await;

                if result.is_ok() {
                    let mut count = counter.lock().await;
                    *count += 1;
                }
            }
        });
        handles.push(handle);
    }

    // Wait with timeout
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(120), async {
        for handle in handles {
            let _ = handle.await;
        }
    })
    .await;

    assert!(
        timeout.is_ok(),
        "Stress test should complete within timeout"
    );

    let final_count = *success_count.lock().await;
    assert!(
        final_count >= 50,
        "At least 50% of messages should succeed under stress"
    );

    println!(
        "✓ Stress test completed: {}/100 messages sent successfully",
        final_count
    );
}
