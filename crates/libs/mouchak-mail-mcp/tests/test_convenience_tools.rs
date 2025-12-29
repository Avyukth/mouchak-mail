//! TDD Tests for convenience workflow tools
//! Following extreme TDD: Tests written BEFORE implementation

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::redundant_field_names
)]

use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    project::ProjectBmc,
};
use libsql::Builder;
use std::sync::Arc;
use tempfile::TempDir;

#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_convenience.db");
    let archive_root = temp_dir.path().join("archive");
    std::fs::create_dir_all(&archive_root).unwrap();

    let db = Builder::new_local(&db_path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let _ = conn.execute("PRAGMA journal_mode=WAL;", ()).await;

    // Run migrations
    let schema1 = include_str!("../../../../migrations/001_initial_schema.sql");
    conn.execute_batch(schema1).await.unwrap();
    let schema2 = include_str!("../../../../migrations/002_agent_capabilities.sql");
    conn.execute_batch(schema2).await.unwrap();
    let schema3 = include_str!("../../../../migrations/003_tool_metrics.sql");
    conn.execute_batch(schema3).await.unwrap();
    let schema4 = include_str!("../../../../migrations/004_attachments.sql");
    conn.execute_batch(schema4).await.unwrap();

    let app_config = Arc::new(AppConfig::default());
    let mm = ModelManager::new_for_test(conn, archive_root, app_config);
    (Arc::new(mm), temp_dir)
}

// ==============================================================================
// TDD RED PHASE: Test 1 - list_builtin_workflows
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_list_builtin_workflows_returns_5_workflows() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    // Create a project (which auto-creates 5 built-in macros)
    let project_id = ProjectBmc::create(&ctx, &mm, "test-project", "/test")
        .await
        .unwrap();

    // RED: This should list the 5 built-in workflow names
    // We're testing the CONTENT, not the tool (tool will call this logic)
    use mouchak_mail_core::model::macro_def::MacroDefBmc;
    let macros = MacroDefBmc::list(&ctx, &mm, project_id.into())
        .await
        .unwrap();

    // Should have exactly 5 built-ins
    assert_eq!(macros.len(), 5, "Should have 5 built-in workflows");

    // Verify the expected names
    let names: Vec<String> = macros.iter().map(|m| m.name.clone()).collect();
    assert!(names.contains(&"start_session".to_string()));
    assert!(names.contains(&"prepare_thread".to_string()));
    assert!(names.contains(&"file_reservation_cycle".to_string()));
    assert!(names.contains(&"contact_handshake".to_string()));
    assert!(names.contains(&"broadcast_message".to_string()));
}

// ==============================================================================
// TDD RED PHASE: Test 2 - quick_standup_workflow
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_quick_standup_sends_to_all_agents() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "test-project", "/test")
        .await
        .unwrap();

    // Create 3 agents
    for i in 1..=3 {
        let agent_c = AgentForCreate {
            project_id,
            name: format!("agent{}", i),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: format!("Agent {}", i),
        };
        AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();
    }

    // RED: Test that we can list all agents (needed for standup broadcast)
    let agents = AgentBmc::list_all_for_project(&ctx, &mm, project_id)
        .await
        .unwrap();
    assert_eq!(agents.len(), 3, "Should have 3 agents");

    // Test we can send a message to all of them
    use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};
    let agent_ids: Vec<i64> = agents.iter().map(|a| a.id.into()).collect();

    let msg_c = MessageForCreate {
        project_id: project_id.into(),
        sender_id: agent_ids[0],          // First agent sends
        recipient_ids: agent_ids.clone(), // To all agents
        cc_ids: None,
        bcc_ids: None,
        subject: "Daily Standup".to_string(),
        body_md: "What are you working on?".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();
    assert!(msg_id > 0, "Should create standup message");
}

// ==============================================================================
// TDD RED PHASE: Test 3 - quick_handoff_workflow
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_quick_handoff_sends_message() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "test-project", "/test")
        .await
        .unwrap();

    // Create 2 agents (from and to)
    let agent1_c = AgentForCreate {
        project_id,
        name: "agent1".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Agent 1".to_string(),
    };
    let agent1_id = AgentBmc::create(&ctx, &mm, agent1_c).await.unwrap();

    let agent2_c = AgentForCreate {
        project_id,
        name: "agent2".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Agent 2".to_string(),
    };
    let agent2_id = AgentBmc::create(&ctx, &mm, agent2_c).await.unwrap();

    // RED: Test handoff message creation
    use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};

    let handoff_msg = MessageForCreate {
        project_id: project_id.into(),
        sender_id: agent1_id.into(),
        recipient_ids: vec![agent2_id.into()],
        cc_ids: None,
        bcc_ids: None,
        subject: "Task Handoff: Feature X".to_string(),
        body_md: "Taking over feature X development.\n\nFiles: src/main.rs".to_string(),
        thread_id: Some("HANDOFF-FEATURE-X".to_string()),
        importance: Some("high".to_string()),
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&ctx, &mm, handoff_msg).await.unwrap();
    assert!(msg_id > 0, "Should create handoff message");
}

// ==============================================================================
// TDD RED PHASE: Test 4 - quick_review_workflow
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_quick_review_reserves_files_and_sends_message() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "test-project", "/test")
        .await
        .unwrap();

    // Create 2 agents (reviewer and reviewee)
    let requester_c = AgentForCreate {
        project_id,
        name: "dev1".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Developer 1".to_string(),
    };
    let requester_id = AgentBmc::create(&ctx, &mm, requester_c).await.unwrap();

    let reviewer_c = AgentForCreate {
        project_id,
        name: "reviewer1".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Reviewer 1".to_string(),
    };
    let reviewer_id = AgentBmc::create(&ctx, &mm, reviewer_c).await.unwrap();

    // RED: Test file reservation (non-exclusive for review)
    use mouchak_mail_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};

    let res_c = FileReservationForCreate {
        project_id: project_id,
        agent_id: reviewer_id,
        path_pattern: "src/main.rs".to_string(),
        exclusive: false, // Non-exclusive for review
        reason: "Code review".to_string(),
        expires_ts: chrono::Utc::now().naive_utc() + chrono::Duration::hours(1),
    };

    let res_id = FileReservationBmc::create(&ctx, &mm, res_c).await.unwrap();
    assert!(res_id > 0, "Should create file reservation");

    // RED: Test review request message
    use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};

    let review_msg = MessageForCreate {
        project_id: project_id.into(),
        sender_id: requester_id.into(),
        recipient_ids: vec![reviewer_id.into()],
        cc_ids: None,
        bcc_ids: None,
        subject: "Code Review Request".to_string(),
        body_md: "Please review src/main.rs\n\nDescription: Added new feature".to_string(),
        thread_id: Some("REVIEW-MAIN-RS".to_string()),
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&ctx, &mm, review_msg).await.unwrap();
    assert!(msg_id > 0, "Should create review request message");
}
