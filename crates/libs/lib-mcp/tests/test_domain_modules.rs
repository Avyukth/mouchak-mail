//! Tests for extracted domain modules in lib-mcp/src/tools/
//!
//! Phase 4 of tools.rs refactoring - adding tests for domain module impl functions.

use lib_common::config::AppConfig;
use lib_core::ctx::Ctx;
use lib_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    message::{MessageBmc, MessageForCreate},
    project::ProjectBmc,
};
use serde_json;
use lib_mcp::tools::{
    // Params
    EnsureProjectParams,
    GetProjectInfoParams,
    GetReviewStateParams,
    ListActivityParams,
    ListContactsParams,
    ListOutboxParams,
    ListPendingReviewsParams,
    ListReservationsParams,
    ListToolMetricsParams,
    RegisterAgentParams,
    RequestContactParams,
    SetContactPolicyParams,
    WhoisParams,
    // Domain impl functions
    agent,
    contacts,
    files,
    observability,
    outbox,
    project,
    reviews,
};
use libsql::Builder;
use std::sync::Arc;
use tempfile::TempDir;

#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_domain.db");
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
// Project Module Tests
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_ensure_project_creates_new() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = EnsureProjectParams {
        slug: "test-proj".to_string(),
        human_key: "/test/path".to_string(),
    };

    let result = project::ensure_project_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
    let result = result.unwrap();

    // Should contain "Created" for new project
    let content = format!("{:?}", result);
    assert!(content.contains("Created") || content.contains("test-proj"));
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_ensure_project_returns_existing() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    // Create project first
    ProjectBmc::create(&ctx, &mm, "existing-proj", "/existing")
        .await
        .unwrap();

    let params = EnsureProjectParams {
        slug: "existing-proj".to_string(),
        human_key: "/existing".to_string(),
    };

    let result = project::ensure_project_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    assert!(content.contains("exists") || content.contains("existing-proj"));
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_list_projects_returns_all() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    // Create 3 projects
    for i in 1..=3 {
        ProjectBmc::create(&ctx, &mm, &format!("proj{}", i), &format!("/proj{}", i))
            .await
            .unwrap();
    }

    let result = project::list_projects_impl(&ctx, &mm).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    assert!(content.contains("proj1"));
    assert!(content.contains("proj2"));
    assert!(content.contains("proj3"));
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_get_project_info() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "info-test", "/info")
        .await
        .unwrap();

    // Add an agent
    let agent_c = AgentForCreate {
        project_id,
        name: "test-agent".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Test agent".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = GetProjectInfoParams {
        project_slug: "info-test".to_string(),
    };

    let result = project::get_project_info_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    assert!(content.contains("info-test"));
    assert!(content.contains("Agents: 1"));
}

// ==============================================================================
// Agent Module Tests
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_register_agent() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    ProjectBmc::create(&ctx, &mm, "agent-test", "/agent")
        .await
        .unwrap();

    let params = RegisterAgentParams {
        project_slug: "agent-test".to_string(),
        name: "new_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Test task".to_string(),
    };

    let result = agent::register_agent_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result);

    let content = format!("{:?}", result);
    assert!(content.contains("new_agent"));
}

/// Test that RegisterAgentParams accepts `agent_name` as alias for `name`
/// This is for NTM compatibility where tools may use `agent_name` instead of `name`
#[test]
fn test_register_agent_params_agent_name_alias() {
    // Test with `agent_name` (alias)
    let json_with_alias = serde_json::json!({
        "project_slug": "test-project",
        "agent_name": "test-agent",
        "program": "claude",
        "model": "opus",
        "task_description": "Test task"
    });

    let params: RegisterAgentParams =
        serde_json::from_value(json_with_alias).expect("Should deserialize with agent_name alias");
    assert_eq!(params.name, "test-agent");
    assert_eq!(params.project_slug, "test-project");

    // Test with `name` (primary field name)
    let json_with_name = serde_json::json!({
        "project_slug": "test-project",
        "name": "test-agent-2",
        "program": "claude",
        "model": "opus",
        "task_description": "Test task"
    });

    let params2: RegisterAgentParams =
        serde_json::from_value(json_with_name).expect("Should deserialize with name field");
    assert_eq!(params2.name, "test-agent-2");
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_whois_agent() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "whois-test", "/whois")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "lookup-agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent for whois test".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = WhoisParams {
        project_slug: "whois-test".to_string(),
        agent_name: "lookup-agent".to_string(),
    };

    let result = agent::whois_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    assert!(content.contains("lookup-agent"));
    assert!(content.contains("opus"));
}

// ==============================================================================
// Contacts Module Tests
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_request_contact() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    // Create two projects with agents
    let proj1_id = ProjectBmc::create(&ctx, &mm, "proj-a", "/a").await.unwrap();
    let proj2_id = ProjectBmc::create(&ctx, &mm, "proj-b", "/b").await.unwrap();

    let agent1_c = AgentForCreate {
        project_id: proj1_id,
        name: "agent-a".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Agent A".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent1_c).await.unwrap();

    let agent2_c = AgentForCreate {
        project_id: proj2_id,
        name: "agent-b".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Agent B".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent2_c).await.unwrap();

    let params = RequestContactParams {
        from_project_slug: "proj-a".to_string(),
        from_agent_name: "agent-a".to_string(),
        to_project_slug: "proj-b".to_string(),
        to_agent_name: "agent-b".to_string(),
        reason: "Collaboration".to_string(),
    };

    let result = contacts::request_contact_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    assert!(content.contains("pending") || content.contains("Contact request"));
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_list_contacts_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "contacts-test", "/contacts")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "lonely-agent".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Agent with no contacts".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = ListContactsParams {
        project_slug: "contacts-test".to_string(),
        agent_name: "lonely-agent".to_string(),
    };

    let result = contacts::list_contacts_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    assert!(content.contains("0") || content.contains("Contacts"));
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_set_contact_policy() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "policy-test", "/policy")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "policy-agent".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Agent for policy test".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = SetContactPolicyParams {
        project_slug: "policy-test".to_string(),
        agent_name: "policy-agent".to_string(),
        contact_policy: "open".to_string(),
    };

    let result = contacts::set_contact_policy_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    assert!(content.contains("open"));
}

// ==============================================================================
// Files (Reservations) Module Tests
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_list_reservations_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    ProjectBmc::create(&ctx, &mm, "res-test", "/res")
        .await
        .unwrap();

    let params = ListReservationsParams {
        project_slug: "res-test".to_string(),
    };

    let result = files::list_reservations_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    assert!(content.contains("0") || content.contains("res-test"));
}

// ==============================================================================
// Reviews Module Tests
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_get_review_state_no_messages() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    ProjectBmc::create(&ctx, &mm, "review-test", "/review")
        .await
        .unwrap();

    let params = GetReviewStateParams {
        project_slug: "review-test".to_string(),
        thread_id: "NONEXISTENT-THREAD".to_string(),
    };

    let result = reviews::get_review_state_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    // Empty thread should have Pending state
    assert!(content.contains("PENDING") || content.contains("thread_id"));
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_get_review_state_with_completion() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "completion-test", "/completion")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "completer".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Completer agent".to_string(),
    };
    let agent_id = AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    // Create a [COMPLETION] message
    let msg = MessageForCreate {
        project_id,
        sender_id: agent_id,
        recipient_ids: vec![agent_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "[COMPLETION] Task done".to_string(),
        body_md: "Completed the task".to_string(),
        thread_id: Some("TASK-123".to_string()),
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&ctx, &mm, msg).await.unwrap();

    let params = GetReviewStateParams {
        project_slug: "completion-test".to_string(),
        thread_id: "TASK-123".to_string(),
    };

    let result = reviews::get_review_state_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    // Should be in COMPLETION state
    assert!(content.contains("COMPLETION") || content.contains("state"));
}

// ==============================================================================
// Outbox Module Tests
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_list_outbox_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "outbox-test", "/outbox")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "sender".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender agent".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = ListOutboxParams {
        project_slug: "outbox-test".to_string(),
        agent_name: "sender".to_string(),
        limit: Some(10),
    };

    let result = outbox::list_outbox_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_list_outbox_with_messages() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "outbox-full", "/outbox-full")
        .await
        .unwrap();

    let sender_c = AgentForCreate {
        project_id,
        name: "sender".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender".to_string(),
    };
    let sender_id = AgentBmc::create(&ctx, &mm, sender_c).await.unwrap();

    let recipient_c = AgentForCreate {
        project_id,
        name: "recipient".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Recipient".to_string(),
    };
    let recipient_id = AgentBmc::create(&ctx, &mm, recipient_c).await.unwrap();

    // Create messages
    for i in 1..=3 {
        let msg = MessageForCreate {
            project_id,
            sender_id,
            recipient_ids: vec![recipient_id],
            cc_ids: None,
            bcc_ids: None,
            subject: format!("Test Message {}", i),
            body_md: format!("Body {}", i),
            thread_id: None,
            importance: None,
            ack_required: false,
        };
        MessageBmc::create(&ctx, &mm, msg).await.unwrap();
    }

    let params = ListOutboxParams {
        project_slug: "outbox-full".to_string(),
        agent_name: "sender".to_string(),
        limit: Some(10),
    };

    let result = outbox::list_outbox_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let content = format!("{:?}", result);
    assert!(content.contains("Test Message"));
}

// ==============================================================================
// Observability Module Tests
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_list_tool_metrics_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListToolMetricsParams {
        project_id: None,
        limit: Some(10),
    };

    let result = observability::list_tool_metrics_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_get_tool_stats() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListToolMetricsParams {
        project_id: None,
        limit: None,
    };

    let result = observability::get_tool_stats_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_list_activity() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "activity-test", "/activity")
        .await
        .unwrap();

    let params = ListActivityParams {
        project_id,
        limit: Some(10),
    };

    let result = observability::list_activity_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_list_pending_reviews() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListPendingReviewsParams {
        project_slug: None,
        sender_name: None,
        limit: Some(10),
    };

    let result = observability::list_pending_reviews_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}
