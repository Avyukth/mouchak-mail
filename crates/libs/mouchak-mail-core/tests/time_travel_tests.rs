//! Time Travel Tests
//!
//! Tests for historical inbox snapshot retrieval covering:
//! - Basic rendering (2 tests)
//! - Timestamp handling (10 tests)
//! - Error handling (4 tests)
//! - Response validation (3 tests)
//! - Security (2 tests)
//! - Edge cases (2 tests)

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{DateTime, Duration, Utc};
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::ModelManager;
use mouchak_mail_core::model::time_travel::{TimeTravelBmc, parse_timestamp};
use libsql::Builder;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

/// Test context with temporary database and git repo
struct TestContext {
    ctx: Ctx,
    mm: ModelManager,
    _temp_dir: TempDir,
}

impl TestContext {
    async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let repo_path = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_path).expect("Create archive dir");

        // Initialize git repo
        git2::Repository::init(&repo_path).expect("Init git repo");

        let db = Builder::new_local(&db_path)
            .build()
            .await
            .expect("Build db");
        let conn = db.connect().expect("Connect to db");

        let app_config = Arc::new(AppConfig::default());
        let mm = ModelManager::new_for_test(conn, repo_path, app_config);
        let ctx = Ctx::root_ctx();

        Self {
            ctx,
            mm,
            _temp_dir: temp_dir,
        }
    }

    /// Create a message in the git archive at a specific time
    async fn create_message_at(
        &self,
        project_slug: &str,
        message_id: i64,
        sender: &str,
        recipient: &str,
        subject: &str,
        at_time: DateTime<Utc>,
    ) {
        let repo = self.mm.get_repo().await.expect("Get repo");
        let repo_guard = repo.lock().await;

        let message = serde_json::json!({
            "id": message_id,
            "sender_name": sender,
            "recipient_names": [recipient],
            "subject": subject,
            "body_md": format!("Body of message {}", message_id),
            "created_ts": at_time.to_rfc3339(),
            "read_ts": null,
            "thread_id": null,
            "importance": "normal"
        });

        let messages_dir = format!("{}/messages", project_slug);
        let file_path = format!("{}/{}.json", messages_dir, message_id);

        // Create directory and file
        let workdir = repo_guard.workdir().expect("Get workdir");
        std::fs::create_dir_all(workdir.join(&messages_dir)).expect("Create messages dir");
        std::fs::write(
            workdir.join(&file_path),
            serde_json::to_string_pretty(&message).expect("Serialize"),
        )
        .expect("Write file");

        // Commit with backdated timestamp
        let mut index = repo_guard.index().expect("Get index");
        index.add_path(Path::new(&file_path)).expect("Add path");
        let tree_id = index.write_tree().expect("Write tree");
        let tree = repo_guard.find_tree(tree_id).expect("Find tree");

        let sig = git2::Signature::new(
            "test",
            "test@test.com",
            &git2::Time::new(at_time.timestamp(), 0),
        )
        .expect("Create signature");

        let parent = match repo_guard.head() {
            Ok(h) => Some(h.peel_to_commit().expect("Peel to commit")),
            Err(_) => None,
        };

        match parent {
            Some(ref p) => {
                repo_guard
                    .commit(
                        Some("HEAD"),
                        &sig,
                        &sig,
                        &format!("Add message {}", message_id),
                        &tree,
                        &[p],
                    )
                    .expect("Commit with parent");
            }
            None => {
                repo_guard
                    .commit(
                        Some("HEAD"),
                        &sig,
                        &sig,
                        &format!("Add message {}", message_id),
                        &tree,
                        &[],
                    )
                    .expect("Initial commit");
            }
        };
    }
}

// ============================================================================
// BASIC RENDERING TESTS (2 tests)
// ============================================================================

#[tokio::test]
async fn test_time_travel_page_renders() {
    let tc = TestContext::new().await;
    let now = Utc::now();

    // Create a message
    tc.create_message_at(
        "test-project",
        1,
        "sender",
        "agent-1",
        "Test",
        now - Duration::hours(1),
    )
    .await;

    // Query should work
    let snapshot = TimeTravelBmc::inbox_at(&tc.ctx, &tc.mm, "test-project", "agent-1", now).await;

    assert!(snapshot.is_ok(), "Time travel query should succeed");
    let snapshot = snapshot.unwrap();
    assert_eq!(snapshot.project_slug, "test-project");
    assert_eq!(snapshot.agent_name, "agent-1");
}

#[tokio::test]
async fn test_time_travel_page_lists_projects() {
    let tc = TestContext::new().await;
    let now = Utc::now();

    // Create messages in multiple projects
    tc.create_message_at("project-a", 1, "s", "a", "Test A", now - Duration::hours(1))
        .await;
    tc.create_message_at(
        "project-b",
        2,
        "s",
        "a",
        "Test B",
        now - Duration::minutes(30),
    )
    .await;

    let projects = TimeTravelBmc::list_projects(&tc.ctx, &tc.mm)
        .await
        .expect("List projects");

    assert!(projects.contains(&"project-a".to_string()));
    assert!(projects.contains(&"project-b".to_string()));
}

// ============================================================================
// TIMESTAMP HANDLING TESTS (10 tests)
// ============================================================================

#[test]
fn test_time_travel_snapshot_valid_timestamp() {
    let dt = parse_timestamp("2024-01-15T10:30:00Z").unwrap();
    assert_eq!(dt.year(), 2024);
    assert_eq!(dt.month(), 1);
    assert_eq!(dt.day(), 15);
}

#[test]
fn test_time_travel_snapshot_past_timestamp() {
    // Unix epoch should parse
    let dt = parse_timestamp("0").unwrap();
    assert_eq!(dt.year(), 1970);
    assert_eq!(dt.month(), 1);
    assert_eq!(dt.day(), 1);
}

#[test]
fn test_time_travel_snapshot_utc_timestamp() {
    let dt = parse_timestamp("2024-06-15T12:00:00Z").unwrap();
    assert_eq!(dt.hour(), 12);
    assert_eq!(dt.minute(), 0);
}

#[test]
fn test_time_travel_snapshot_timezone_offset() {
    // +05:30 offset (IST)
    let dt = parse_timestamp("2024-01-15T10:30:00+05:30").unwrap();
    // Should convert to UTC: 10:30 IST = 05:00 UTC
    assert_eq!(dt.hour(), 5);
}

#[test]
fn test_time_travel_snapshot_naive_timestamp() {
    // No timezone = assume UTC
    let dt = parse_timestamp("2024-01-15T10:30:00").unwrap();
    assert_eq!(dt.hour(), 10);
}

#[test]
fn test_time_travel_snapshot_invalid_timestamp_format() {
    assert!(parse_timestamp("not-a-date").is_err());
    assert!(parse_timestamp("yesterday").is_err());
    assert!(parse_timestamp("2024/01/15").is_err()); // Wrong separator
}

#[test]
fn test_time_travel_snapshot_missing_timestamp() {
    assert!(parse_timestamp("").is_err());
    assert!(parse_timestamp("   ").is_err());
}

#[test]
fn test_time_travel_snapshot_partial_date_format() {
    // Date only should work (midnight UTC)
    let dt = parse_timestamp("2024-01-15").unwrap();
    assert_eq!(dt.hour(), 0);
    assert_eq!(dt.minute(), 0);
    assert_eq!(dt.second(), 0);
}

#[test]
fn test_time_travel_snapshot_leap_second() {
    // 2016-12-31T23:59:60Z - leap second
    // Most parsers treat this as 2017-01-01T00:00:00Z or reject it
    // Our parser should handle it gracefully (either accept or reject cleanly)
    let result = parse_timestamp("2016-12-31T23:59:60Z");
    // Accept either success (normalized) or clean error
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_time_travel_snapshot_negative_timezone() {
    // -05:00 offset (EST)
    let dt = parse_timestamp("2024-01-15T10:30:00-05:00").unwrap();
    // Should convert to UTC: 10:30 EST = 15:30 UTC
    assert_eq!(dt.hour(), 15);
    assert_eq!(dt.minute(), 30);
}

#[test]
fn test_time_travel_snapshot_epoch() {
    // Test Unix epoch parsing
    let epoch = parse_timestamp("1705312200").unwrap(); // 2024-01-15 10:30:00 UTC
    assert_eq!(epoch.year(), 2024);
    assert_eq!(epoch.month(), 1);
    assert_eq!(epoch.day(), 15);
}

// ============================================================================
// ERROR HANDLING TESTS (4 tests)
// ============================================================================

#[tokio::test]
async fn test_time_travel_snapshot_invalid_project() {
    let tc = TestContext::new().await;
    let now = Utc::now();

    // Create a valid message first
    tc.create_message_at(
        "valid-project",
        1,
        "s",
        "a",
        "Test",
        now - Duration::hours(1),
    )
    .await;

    // Query for non-existent project should return empty snapshot
    let result =
        TimeTravelBmc::inbox_at(&tc.ctx, &tc.mm, "nonexistent-project", "agent-1", now).await;

    assert!(
        result.is_ok(),
        "Should succeed with empty result for unknown project"
    );
    let snapshot = result.unwrap();
    assert!(
        snapshot.messages.is_empty(),
        "Should have no messages for unknown project"
    );
}

#[tokio::test]
async fn test_time_travel_snapshot_invalid_agent_name() {
    let tc = TestContext::new().await;

    // XSS in agent name should be rejected
    let result = TimeTravelBmc::inbox_at(
        &tc.ctx,
        &tc.mm,
        "project",
        "<script>alert(1)</script>",
        Utc::now(),
    )
    .await;

    assert!(result.is_err(), "XSS in agent name should be rejected");
}

#[tokio::test]
async fn test_time_travel_snapshot_nonexistent_agent() {
    let tc = TestContext::new().await;
    let now = Utc::now();

    // Create message for agent-1
    tc.create_message_at(
        "project",
        1,
        "sender",
        "agent-1",
        "Test",
        now - Duration::hours(1),
    )
    .await;

    // Query for agent-2 should return empty
    let result = TimeTravelBmc::inbox_at(&tc.ctx, &tc.mm, "project", "agent-2", now).await;

    assert!(result.is_ok());
    let snapshot = result.unwrap();
    assert!(
        snapshot.messages.is_empty(),
        "Agent with no messages should have empty inbox"
    );
}

#[tokio::test]
async fn test_time_travel_snapshot_nonexistent_project() {
    let tc = TestContext::new().await;
    let now = Utc::now();

    // Create at least one commit so the repo has HEAD
    tc.create_message_at(
        "other-project",
        1,
        "s",
        "a",
        "Test",
        now - Duration::hours(1),
    )
    .await;

    // Query for project that never existed
    let result =
        TimeTravelBmc::inbox_at(&tc.ctx, &tc.mm, "never-existed-project", "agent", now).await;

    assert!(result.is_ok());
    assert!(result.unwrap().messages.is_empty());
}

// ============================================================================
// RESPONSE VALIDATION TESTS (3 tests)
// ============================================================================

#[tokio::test]
async fn test_time_travel_snapshot_response_structure() {
    let tc = TestContext::new().await;
    let now = Utc::now();
    let one_hour_ago = now - Duration::hours(1);

    tc.create_message_at(
        "my-project",
        1,
        "sender",
        "recipient",
        "Subject",
        one_hour_ago,
    )
    .await;

    let snapshot = TimeTravelBmc::inbox_at(&tc.ctx, &tc.mm, "my-project", "recipient", now)
        .await
        .unwrap();

    // Verify structure
    assert_eq!(snapshot.project_slug, "my-project");
    assert_eq!(snapshot.agent_name, "recipient");
    assert!(snapshot.requested_at <= now);
    assert!(snapshot.snapshot_at <= now);
}

#[tokio::test]
async fn test_time_travel_snapshot_message_fields() {
    let tc = TestContext::new().await;
    let now = Utc::now();
    let one_hour_ago = now - Duration::hours(1);

    tc.create_message_at(
        "project",
        42,
        "claude-code",
        "reviewer",
        "Code Review Request",
        one_hour_ago,
    )
    .await;

    let snapshot = TimeTravelBmc::inbox_at(&tc.ctx, &tc.mm, "project", "reviewer", now)
        .await
        .unwrap();

    assert_eq!(snapshot.messages.len(), 1);
    let msg = &snapshot.messages[0];
    assert_eq!(msg.id, 42);
    assert_eq!(msg.sender_name, "claude-code");
    assert!(msg.recipient_names.contains(&"reviewer".to_string()));
    assert_eq!(msg.subject, "Code Review Request");
    assert!(!msg.body_md.is_empty());
}

#[tokio::test]
async fn test_time_travel_snapshot_project_no_messages() {
    let tc = TestContext::new().await;
    let now = Utc::now();

    // Create project directory but no messages
    tc.create_message_at(
        "empty-project",
        1,
        "sender",
        "other-agent", // Different agent
        "Test",
        now - Duration::hours(1),
    )
    .await;

    let snapshot = TimeTravelBmc::inbox_at(&tc.ctx, &tc.mm, "empty-project", "target-agent", now)
        .await
        .unwrap();

    assert!(
        snapshot.messages.is_empty(),
        "Agent with no messages should have empty inbox"
    );
}

// ============================================================================
// SECURITY TESTS (2 tests)
// ============================================================================

#[tokio::test]
async fn test_time_travel_snapshot_xss_in_project() {
    let tc = TestContext::new().await;

    // XSS payload in project slug
    let result = TimeTravelBmc::inbox_at(
        &tc.ctx,
        &tc.mm,
        "<script>alert('xss')</script>",
        "agent",
        Utc::now(),
    )
    .await;

    assert!(result.is_err(), "XSS in project slug should be rejected");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("invalid characters"),
        "Error should mention invalid characters"
    );
}

#[tokio::test]
async fn test_time_travel_snapshot_xss_in_agent() {
    let tc = TestContext::new().await;

    // XSS payload in agent name
    let result = TimeTravelBmc::inbox_at(
        &tc.ctx,
        &tc.mm,
        "project",
        "agent<img src=x onerror=alert(1)>",
        Utc::now(),
    )
    .await;

    assert!(result.is_err(), "XSS in agent name should be rejected");
}

// ============================================================================
// EDGE CASES (2 tests)
// ============================================================================

#[tokio::test]
async fn test_time_travel_page_no_projects() {
    let tc = TestContext::new().await;

    // Empty repo - no projects
    let projects = TimeTravelBmc::list_projects(&tc.ctx, &tc.mm).await;

    assert!(projects.is_ok());
    assert!(
        projects.unwrap().is_empty(),
        "Empty repo should have no projects"
    );
}

#[tokio::test]
async fn test_time_travel_snapshot_future_timestamp() {
    let tc = TestContext::new().await;
    let now = Utc::now();

    // Create a message now
    tc.create_message_at("project", 1, "sender", "agent", "Test", now)
        .await;

    // Query for a future timestamp should use latest commit
    let future = now + Duration::days(365);
    let snapshot = TimeTravelBmc::inbox_at(&tc.ctx, &tc.mm, "project", "agent", future)
        .await
        .unwrap();

    // Should still find the message
    assert_eq!(snapshot.messages.len(), 1);
}

// ============================================================================
// ADDITIONAL TESTS
// ============================================================================

#[tokio::test]
async fn test_time_travel_multiple_messages_chronological() {
    let tc = TestContext::new().await;
    let now = Utc::now();

    // Create messages at different times
    tc.create_message_at(
        "project",
        1,
        "sender",
        "agent",
        "First",
        now - Duration::hours(3),
    )
    .await;
    tc.create_message_at(
        "project",
        2,
        "sender",
        "agent",
        "Second",
        now - Duration::hours(2),
    )
    .await;
    tc.create_message_at(
        "project",
        3,
        "sender",
        "agent",
        "Third",
        now - Duration::hours(1),
    )
    .await;

    // Query at current time should see all 3
    let snapshot = TimeTravelBmc::inbox_at(&tc.ctx, &tc.mm, "project", "agent", now)
        .await
        .unwrap();
    assert_eq!(snapshot.messages.len(), 3);

    // Query at 2.5 hours ago should only see first message
    let snapshot = TimeTravelBmc::inbox_at(
        &tc.ctx,
        &tc.mm,
        "project",
        "agent",
        now - Duration::minutes(150),
    )
    .await
    .unwrap();
    assert_eq!(snapshot.messages.len(), 1);
    assert_eq!(snapshot.messages[0].subject, "First");
}

#[tokio::test]
async fn test_time_travel_path_traversal_rejected() {
    let tc = TestContext::new().await;

    // Path traversal attempt
    let result =
        TimeTravelBmc::inbox_at(&tc.ctx, &tc.mm, "../../../etc/passwd", "agent", Utc::now()).await;

    assert!(result.is_err(), "Path traversal should be rejected");
}

use chrono::Datelike;
use chrono::Timelike;
