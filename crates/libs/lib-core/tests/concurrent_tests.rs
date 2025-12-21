//! Concurrency Tests for Parallel MCP Operations (PORT-7.1)
//!
//! Tests verifying thread safety and race condition handling.
//! Uses tokio::spawn for parallelism and futures::future::join_all for collection.

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

use chrono::{Duration, Utc};
use futures::future::join_all;
use lib_common::config::AppConfig;
use lib_core::ctx::Ctx;
use lib_core::model::ModelManager;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::project::ProjectBmc;
use libsql::Builder;
use std::sync::Arc;
use tempfile::TempDir;

/// Create a test ModelManager with a fresh database including migrations
async fn create_test_mm() -> (ModelManager, TempDir) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let db_path = temp_dir.path().join("concurrent_test.db");
    let db = Builder::new_local(&db_path).build().await.unwrap();
    let conn = db.connect().unwrap();

    // Run migrations inline (same as store/mod.rs)
    let migrations = [
        include_str!("../../../../migrations/001_initial_schema.sql"),
        include_str!("../../../../migrations/002_agent_capabilities.sql"),
        include_str!("../../../../migrations/003_tool_metrics.sql"),
        include_str!("../../../../migrations/004_attachments.sql"),
    ];
    for migration in &migrations {
        conn.execute_batch(migration).await.expect("run migration");
    }

    let app_config = Arc::new(AppConfig::default());
    let mm = ModelManager::new_for_test(conn, temp_dir.path().to_path_buf(), app_config);
    (mm, temp_dir)
}

/// Setup a project with agents for testing
async fn setup_test_project(mm: &ModelManager) -> (i64, Vec<i64>) {
    let ctx = Ctx::root_ctx();
    let project_id = ProjectBmc::create(&ctx, mm, "concurrent-test", "/concurrent/test")
        .await
        .expect("create project");

    let mut agent_ids = Vec::new();
    for i in 0..5 {
        let agent = AgentForCreate {
            project_id,
            name: format!("agent-{}", i),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "concurrent test agent".to_string(),
        };
        let id = AgentBmc::create(&ctx, mm, agent)
            .await
            .expect("create agent");
        agent_ids.push(id);
    }

    (project_id, agent_ids)
}

/// Helper to create a MessageForCreate with all required fields
fn make_message(
    project_id: i64,
    sender_id: i64,
    recipient_ids: Vec<i64>,
    subject: String,
    body_md: String,
    thread_id: Option<String>,
) -> MessageForCreate {
    MessageForCreate {
        project_id,
        sender_id,
        recipient_ids,
        cc_ids: None,
        bcc_ids: None,
        subject,
        body_md,
        thread_id,
        importance: Some("normal".to_string()),
        ack_required: false,
    }
}

/// Helper to create a FileReservationForCreate
fn make_reservation(
    project_id: i64,
    agent_id: i64,
    path_pattern: String,
) -> FileReservationForCreate {
    FileReservationForCreate {
        project_id,
        agent_id,
        path_pattern,
        exclusive: true,
        reason: "concurrent test".to_string(),
        expires_ts: (Utc::now() + Duration::hours(1)).naive_utc(),
    }
}

// ============================================================================
// TEST 1: Concurrent Message Sends (10 parallel)
// ============================================================================

#[tokio::test]
async fn test_concurrent_message_sends() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let (project_id, agent_ids) = setup_test_project(&mm).await;

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let mm = Arc::clone(&mm);
            let sender_id = agent_ids[i % agent_ids.len()];
            let recipient_id = agent_ids[(i + 1) % agent_ids.len()];

            tokio::spawn(async move {
                let ctx = Ctx::root_ctx();
                let msg = make_message(
                    project_id,
                    sender_id,
                    vec![recipient_id],
                    format!("Concurrent message {}", i),
                    format!("Body of message {}", i),
                    None,
                );
                MessageBmc::create(&ctx, &mm, msg).await.is_ok()
            })
        })
        .collect();

    let results: Vec<bool> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    let successful = results.iter().filter(|&&b| b).count();
    assert_eq!(successful, 10, "All 10 concurrent messages should succeed");
}

// ============================================================================
// TEST 2: Concurrent Messages to Same Thread (5 agents)
// ============================================================================

#[tokio::test]
async fn test_concurrent_messages_to_same_thread() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let (project_id, agent_ids) = setup_test_project(&mm).await;
    let thread_id = "shared-thread-001";

    let handles: Vec<_> = agent_ids
        .iter()
        .enumerate()
        .map(|(i, &sender_id)| {
            let mm = Arc::clone(&mm);
            let recipient_id = agent_ids[(i + 1) % agent_ids.len()];
            let thread = thread_id.to_string();

            tokio::spawn(async move {
                let ctx = Ctx::root_ctx();
                let msg = make_message(
                    project_id,
                    sender_id,
                    vec![recipient_id],
                    format!("Thread message from agent {}", i),
                    "Same thread test".to_string(),
                    Some(thread),
                );
                MessageBmc::create(&ctx, &mm, msg).await.is_ok()
            })
        })
        .collect();

    let results: Vec<bool> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    assert!(
        results.iter().all(|&b| b),
        "All thread messages should succeed"
    );
}

// ============================================================================
// TEST 3: Concurrent File Reservation - Different Paths
// ============================================================================

#[tokio::test]
async fn test_concurrent_file_reservation_different_paths() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let (project_id, agent_ids) = setup_test_project(&mm).await;

    let handles: Vec<_> = agent_ids
        .iter()
        .enumerate()
        .map(|(i, &agent_id)| {
            let mm = Arc::clone(&mm);
            let path = format!("src/module_{}/file.rs", i);

            tokio::spawn(async move {
                let ctx = Ctx::root_ctx();
                let reservation = make_reservation(project_id, agent_id, path);
                FileReservationBmc::create(&ctx, &mm, reservation)
                    .await
                    .is_ok()
            })
        })
        .collect();

    let results: Vec<bool> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    let successful = results.iter().filter(|&&b| b).count();
    assert_eq!(
        successful,
        agent_ids.len(),
        "All different path reservations should succeed"
    );
}

// ============================================================================
// TEST 4: Concurrent File Reservation - Same Path Conflict
// ============================================================================

#[tokio::test]
async fn test_concurrent_file_reservation_same_path_conflict() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let (project_id, agent_ids) = setup_test_project(&mm).await;
    let contested_path = "src/shared/config.rs".to_string();

    let handles: Vec<_> = agent_ids
        .iter()
        .map(|&agent_id| {
            let mm = Arc::clone(&mm);
            let path = contested_path.clone();

            tokio::spawn(async move {
                let ctx = Ctx::root_ctx();
                let reservation = make_reservation(project_id, agent_id, path);
                FileReservationBmc::create(&ctx, &mm, reservation)
                    .await
                    .is_ok()
            })
        })
        .collect();

    let results: Vec<bool> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    let successful = results.iter().filter(|&&b| b).count();
    assert!(successful >= 1, "At least one reservation should succeed");
}

// ============================================================================
// TEST 5: Concurrent File Reservation - Overlapping Globs
// ============================================================================

#[tokio::test]
async fn test_concurrent_file_reservation_overlapping_globs() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let (project_id, agent_ids) = setup_test_project(&mm).await;
    let globs = [
        "src/**/*.rs",
        "src/core/**/*",
        "src/*.rs",
        "**/*.rs",
        "src/core/*.rs",
    ];

    let handles: Vec<_> = agent_ids
        .iter()
        .zip(globs.iter())
        .map(|(&agent_id, glob)| {
            let mm = Arc::clone(&mm);
            let glob = (*glob).to_string();

            tokio::spawn(async move {
                let ctx = Ctx::root_ctx();
                let reservation = make_reservation(project_id, agent_id, glob);
                FileReservationBmc::create(&ctx, &mm, reservation)
                    .await
                    .is_ok()
            })
        })
        .collect();

    let results: Vec<bool> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    let successful = results.iter().filter(|&&b| b).count();
    assert!(
        successful >= 1,
        "System should handle overlapping globs gracefully"
    );
}

// ============================================================================
// TEST 6: Concurrent Inbox Fetches (10 parallel)
// ============================================================================

#[tokio::test]
async fn test_concurrent_inbox_fetches() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let (project_id, agent_ids) = setup_test_project(&mm).await;

    // Create some messages first
    let ctx = Ctx::root_ctx();
    for i in 0..5 {
        let msg = make_message(
            project_id,
            agent_ids[0],
            vec![agent_ids[1]],
            format!("Test message {}", i),
            "Test body".to_string(),
            None,
        );
        MessageBmc::create(&ctx, &mm, msg).await.ok();
    }

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let mm = Arc::clone(&mm);
            let agent_id = agent_ids[1];
            let pid = project_id;

            tokio::spawn(async move {
                let ctx = Ctx::root_ctx();
                MessageBmc::list_inbox_for_agent(&ctx, &mm, pid, agent_id, 100)
                    .await
                    .is_ok()
            })
        })
        .collect();

    let results: Vec<bool> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    let successful = results.iter().filter(|&&b| b).count();
    assert_eq!(
        successful, 10,
        "All 10 concurrent inbox fetches should succeed"
    );
}

// ============================================================================
// TEST 7: Concurrent Inbox Fetch During Message Send
// ============================================================================

#[tokio::test]
async fn test_concurrent_inbox_fetch_during_message_send() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let (project_id, agent_ids) = setup_test_project(&mm).await;

    let mut handles = Vec::new();

    // 5 message sends
    for i in 0..5 {
        let mm = Arc::clone(&mm);
        let sender_id = agent_ids[0];
        let recipient_id = agent_ids[1];

        handles.push(tokio::spawn(async move {
            let ctx = Ctx::root_ctx();
            let msg = make_message(
                project_id,
                sender_id,
                vec![recipient_id],
                format!("Concurrent send {}", i),
                "Body".to_string(),
                None,
            );
            MessageBmc::create(&ctx, &mm, msg).await.is_ok()
        }));
    }

    // 5 inbox fetches
    for _ in 0..5 {
        let mm = Arc::clone(&mm);
        let agent_id = agent_ids[1];
        let pid = project_id;

        handles.push(tokio::spawn(async move {
            let ctx = Ctx::root_ctx();
            MessageBmc::list_inbox_for_agent(&ctx, &mm, pid, agent_id, 100)
                .await
                .is_ok()
        }));
    }

    let results: Vec<Result<bool, _>> = join_all(handles).await;
    let panics = results
        .iter()
        .filter(|r: &&Result<bool, _>| r.is_err())
        .count();
    assert_eq!(panics, 0, "No operations should panic");
}

// ============================================================================
// TEST 8: Concurrent Project Creation (conflict handling)
// ============================================================================

#[tokio::test]
async fn test_concurrent_project_creation() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    // 10 concurrent calls trying to create projects with unique slugs
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let mm = Arc::clone(&mm);

            tokio::spawn(async move {
                let ctx = Ctx::root_ctx();
                ProjectBmc::create(
                    &ctx,
                    &mm,
                    &format!("project-{}", i),
                    &format!("/path/{}", i),
                )
                .await
                .is_ok()
            })
        })
        .collect();

    let results: Vec<bool> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    let successful = results.iter().filter(|&&b| b).count();
    assert_eq!(
        successful, 10,
        "All 10 unique project creations should succeed"
    );
}

// ============================================================================
// TEST 9: Concurrent Agent Registration
// ============================================================================

#[tokio::test]
async fn test_concurrent_agent_registration() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let ctx = Ctx::root_ctx();
    let project_id = ProjectBmc::create(&ctx, &mm, "agent-reg-test", "/agent/reg")
        .await
        .expect("create project");

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let mm = Arc::clone(&mm);
            let name = format!("concurrent-agent-{}", i);

            tokio::spawn(async move {
                let ctx = Ctx::root_ctx();
                let agent = AgentForCreate {
                    project_id,
                    name,
                    program: "test".to_string(),
                    model: "test".to_string(),
                    task_description: "concurrent registration test".to_string(),
                };
                AgentBmc::create(&ctx, &mm, agent).await.is_ok()
            })
        })
        .collect();

    let results: Vec<bool> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    let successful = results.iter().filter(|&&b| b).count();
    assert_eq!(
        successful, 10,
        "All 10 concurrent agent registrations should succeed"
    );
}

// ============================================================================
// TEST 10: Concurrent Message Read/Write
// ============================================================================

#[tokio::test]
async fn test_concurrent_message_read_write() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let (project_id, agent_ids) = setup_test_project(&mm).await;

    let mut handles = Vec::new();

    // 5 writers
    for i in 0..5 {
        let mm = Arc::clone(&mm);
        let sender_id = agent_ids[0];
        let recipient_id = agent_ids[1];

        handles.push(tokio::spawn(async move {
            let ctx = Ctx::root_ctx();
            let msg = make_message(
                project_id,
                sender_id,
                vec![recipient_id],
                format!("Write {}", i),
                "Write body".to_string(),
                None,
            );
            MessageBmc::create(&ctx, &mm, msg).await.is_ok()
        }));
    }

    // 5 readers
    for i in 1..=5 {
        let mm = Arc::clone(&mm);

        handles.push(tokio::spawn(async move {
            let ctx = Ctx::root_ctx();
            // May or may not find the message
            let _ = MessageBmc::get(&ctx, &mm, i).await;
            true // Just checking no panic
        }));
    }

    let results: Vec<Result<bool, _>> = join_all(handles).await;
    let panics = results
        .iter()
        .filter(|r: &&Result<bool, _>| r.is_err())
        .count();
    assert_eq!(panics, 0, "No read/write operations should panic");
}

// ============================================================================
// TEST 11: Concurrent Archive Writes
// ============================================================================

#[tokio::test]
async fn test_concurrent_archive_writes() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let (project_id, agent_ids) = setup_test_project(&mm).await;

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let mm = Arc::clone(&mm);
            let sender_id = agent_ids[i % agent_ids.len()];
            let recipient_id = agent_ids[(i + 1) % agent_ids.len()];

            tokio::spawn(async move {
                let ctx = Ctx::root_ctx();
                let msg = make_message(
                    project_id,
                    sender_id,
                    vec![recipient_id],
                    format!("Archive write {}", i),
                    format!("Archive content {}", i),
                    None,
                );
                MessageBmc::create(&ctx, &mm, msg).await.is_ok()
            })
        })
        .collect();

    let results: Vec<bool> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    let successful = results.iter().filter(|&&b| b).count();
    assert!(
        successful >= 1,
        "At least some archive writes should succeed"
    );
}

// ============================================================================
// TEST 12: Concurrent Message Bundle Writes
// ============================================================================

#[tokio::test]
async fn test_concurrent_message_bundle_writes() {
    let (mm, _temp) = create_test_mm().await;
    let mm = Arc::new(mm);

    let (project_id, agent_ids) = setup_test_project(&mm).await;

    let handles: Vec<_> = agent_ids
        .iter()
        .map(|&sender_id| {
            let mm = Arc::clone(&mm);
            let agents = agent_ids.clone();

            tokio::spawn(async move {
                let ctx = Ctx::root_ctx();
                let mut success_count = 0;

                for i in 0..3 {
                    let recipient_id = agents[(sender_id as usize + i + 1) % agents.len()];
                    let msg = make_message(
                        project_id,
                        sender_id,
                        vec![recipient_id],
                        format!("Bundle {} msg {}", sender_id, i),
                        "Bundle content".to_string(),
                        Some(format!("bundle-{}", sender_id)),
                    );
                    if MessageBmc::create(&ctx, &mm, msg).await.is_ok() {
                        success_count += 1;
                    }
                }

                success_count
            })
        })
        .collect();

    let results: Vec<usize> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    for (i, &count) in results.iter().enumerate() {
        assert!(count >= 1, "Agent {} bundle should have some successes", i);
    }
}
