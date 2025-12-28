//! Tests for macro tool implementations
//! Target: Improve coverage for lib-mcp/src/tools/macros.rs

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::redundant_field_names
)]

use lib_common::config::AppConfig;
use lib_core::ctx::Ctx;
use lib_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    project::ProjectBmc,
};
use lib_mcp::tools::macros;
use lib_mcp::tools::{
    InvokeMacroParams, ListBuiltinWorkflowsParams, ListMacrosParams, MacroContactHandshakeParams,
    MacroFileReservationCycleParams, MacroPrepareThreadParams, MacroStartSessionParams,
    QuickHandoffWorkflowParams, QuickReviewWorkflowParams, QuickStandupWorkflowParams,
    RegisterMacroParams, UnregisterMacroParams,
};
use libsql::Builder;
use std::sync::Arc;
use tempfile::TempDir;

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_macro_impl.db");
    let archive_root = temp_dir.path().join("archive");
    std::fs::create_dir_all(&archive_root).unwrap();

    let db = Builder::new_local(&db_path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let _ = conn.execute("PRAGMA journal_mode=WAL;", ()).await;

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

async fn setup_project_and_agent(mm: &Arc<ModelManager>) -> (i64, i64, String) {
    let ctx = Ctx::root_ctx();
    let project_slug = "test-macro-project";
    let project_id = ProjectBmc::create(&ctx, mm, project_slug, "Test Macro Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id: project_id,
        name: "test_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Test agent".to_string(),
    };
    let agent_id = AgentBmc::create(&ctx, mm, agent_c).await.unwrap();

    (project_id.into(), agent_id.into(), project_slug.to_string())
}

// ==============================================================================
// list_macros_impl tests
// ==============================================================================

#[tokio::test]
async fn test_list_macros_impl_returns_builtin_macros() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = ListMacrosParams {
        project_slug: project_slug.clone(),
    };

    let result = macros::list_macros_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "list_macros should succeed");

    let call_result = result.unwrap();
    let text = format!("{:?}", call_result);

    // Should contain built-in macros
    assert!(
        text.contains("start_session") || text.contains("5"),
        "Should list built-in macros"
    );
}

#[tokio::test]
async fn test_list_macros_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListMacrosParams {
        project_slug: "nonexistent-project".to_string(),
    };

    let result = macros::list_macros_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for invalid project");
}

// ==============================================================================
// register_macro_impl tests
// ==============================================================================

#[tokio::test]
async fn test_register_macro_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = RegisterMacroParams {
        project_slug: project_slug.clone(),
        name: "custom-workflow".to_string(),
        description: "Custom test workflow".to_string(),
        steps: vec![serde_json::json!({"action": "test", "target": "foo"})],
    };

    let result = macros::register_macro_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "register_macro should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Registered") && text.contains("custom-workflow"));
}

// ==============================================================================
// unregister_macro_impl tests
// ==============================================================================

#[tokio::test]
async fn test_unregister_macro_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    // First register a macro
    let register_params = RegisterMacroParams {
        project_slug: project_slug.clone(),
        name: "to-delete".to_string(),
        description: "Macro to delete".to_string(),
        steps: vec![serde_json::json!({"action": "delete"})],
    };
    macros::register_macro_impl(&ctx, &mm, register_params)
        .await
        .unwrap();

    // Now unregister it
    let params = UnregisterMacroParams {
        project_slug: project_slug.clone(),
        name: "to-delete".to_string(),
    };

    let result = macros::unregister_macro_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Unregistered"));
}

#[tokio::test]
async fn test_unregister_macro_impl_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = UnregisterMacroParams {
        project_slug,
        name: "nonexistent-macro".to_string(),
    };

    let result = macros::unregister_macro_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("not found"));
}

// ==============================================================================
// invoke_macro_impl tests
// ==============================================================================

#[tokio::test]
async fn test_invoke_macro_impl_builtin() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = InvokeMacroParams {
        project_slug,
        name: "start_session".to_string(),
    };

    let result = macros::invoke_macro_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "Should invoke built-in macro");
    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("start_session"));
}

#[tokio::test]
async fn test_invoke_macro_impl_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = InvokeMacroParams {
        project_slug,
        name: "nonexistent".to_string(),
    };

    let result = macros::invoke_macro_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for nonexistent macro");
}

// ==============================================================================
// list_builtin_workflows_impl tests
// ==============================================================================

#[tokio::test]
async fn test_list_builtin_workflows_impl() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListBuiltinWorkflowsParams {};

    let result = macros::list_builtin_workflows_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("start_session"));
    assert!(text.contains("prepare_thread"));
    assert!(text.contains("file_reservation_cycle"));
    assert!(text.contains("contact_handshake"));
    assert!(text.contains("broadcast_message"));
}

// ==============================================================================
// quick_standup_workflow_impl tests
// ==============================================================================

#[tokio::test]
async fn test_quick_standup_workflow_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = QuickStandupWorkflowParams {
        project_slug,
        sender_name: "test_agent".to_string(),
        standup_question: Some("What did you accomplish?".to_string()),
    };

    let result = macros::quick_standup_workflow_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "Standup workflow should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Standup request sent"));
}

#[tokio::test]
async fn test_quick_standup_workflow_impl_default_question() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = QuickStandupWorkflowParams {
        project_slug,
        sender_name: "test_agent".to_string(),
        standup_question: None, // Uses default
    };

    let result = macros::quick_standup_workflow_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

// ==============================================================================
// quick_handoff_workflow_impl tests
// ==============================================================================

#[tokio::test]
async fn test_quick_handoff_workflow_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent(&mm).await;

    // Create second agent to hand off to
    let agent2_c = AgentForCreate {
        project_id: project_id.into(),
        name: "agent2".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Second agent".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent2_c).await.unwrap();

    let params = QuickHandoffWorkflowParams {
        project_slug,
        from_agent: "test_agent".to_string(),
        to_agent: "agent2".to_string(),
        task_description: "Complete feature X".to_string(),
        files: Some(vec!["src/main.rs".to_string(), "src/lib.rs".to_string()]),
    };

    let result = macros::quick_handoff_workflow_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "Handoff workflow should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Handoff message sent"));
}

#[tokio::test]
async fn test_quick_handoff_workflow_impl_no_files() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent(&mm).await;

    let agent2_c = AgentForCreate {
        project_id: project_id.into(),
        name: "agent2".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Second agent".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent2_c).await.unwrap();

    let params = QuickHandoffWorkflowParams {
        project_slug,
        from_agent: "test_agent".to_string(),
        to_agent: "agent2".to_string(),
        task_description: "Complete feature X".to_string(),
        files: None,
    };

    let result = macros::quick_handoff_workflow_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

// ==============================================================================
// quick_review_workflow_impl tests
// ==============================================================================

#[tokio::test]
async fn test_quick_review_workflow_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent(&mm).await;

    // Create reviewer agent
    let reviewer_c = AgentForCreate {
        project_id: project_id.into(),
        name: "reviewer".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Code reviewer".to_string(),
    };
    AgentBmc::create(&ctx, &mm, reviewer_c).await.unwrap();

    let params = QuickReviewWorkflowParams {
        project_slug,
        requester: "test_agent".to_string(),
        reviewer: "reviewer".to_string(),
        files_to_review: vec!["src/main.rs".to_string()],
        description: "Please review this PR".to_string(),
    };

    let result = macros::quick_review_workflow_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "Review workflow should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Review request sent"));
    assert!(text.contains("Reserved"));
}

// ==============================================================================
// macro_start_session_impl tests
// ==============================================================================

#[tokio::test]
async fn test_macro_start_session_impl_new_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = MacroStartSessionParams {
        human_key: "new-session-project".to_string(),
        program: "claude".to_string(),
        model: "opus-4".to_string(),
        task_description: "Test session".to_string(),
        agent_name: Some("session-agent".to_string()),
        file_reservation_paths: None,
        file_reservation_ttl_seconds: 3600,
        file_reservation_reason: "Work session".to_string(),
        inbox_limit: 10,
    };

    let result = macros::macro_start_session_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "Start session should create project");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("project") && text.contains("agent"));
}

#[tokio::test]
async fn test_macro_start_session_impl_existing_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    // Create project first
    ProjectBmc::create(&ctx, &mm, "existing-project", "Existing Project")
        .await
        .unwrap();

    let params = MacroStartSessionParams {
        human_key: "existing-project".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Test session".to_string(),
        agent_name: None, // Auto-generate name
        file_reservation_paths: Some(vec!["src/**/*.rs".to_string()]),
        file_reservation_ttl_seconds: 1800,
        file_reservation_reason: "Development".to_string(),
        inbox_limit: 5,
    };

    let result = macros::macro_start_session_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("existing-project"));
    assert!(text.contains("file_reservations"));
}

// ==============================================================================
// macro_prepare_thread_impl tests
// ==============================================================================

#[tokio::test]
async fn test_macro_prepare_thread_impl_new_agent() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = MacroPrepareThreadParams {
        project_key: project_slug,
        thread_id: "THREAD-001".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Join thread".to_string(),
        agent_name: Some("new-thread-agent".to_string()),
        register_if_missing: true,
        include_examples: true,
        include_inbox_bodies: true,
        inbox_limit: 10,
    };

    let result = macros::macro_prepare_thread_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("thread") && text.contains("THREAD-001"));
}

#[tokio::test]
async fn test_macro_prepare_thread_impl_existing_agent() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = MacroPrepareThreadParams {
        project_key: project_slug,
        thread_id: "THREAD-002".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Test".to_string(),
        agent_name: Some("test_agent".to_string()),
        register_if_missing: false,
        include_examples: false,
        include_inbox_bodies: false,
        inbox_limit: 5,
    };

    let result = macros::macro_prepare_thread_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

// ==============================================================================
// macro_file_reservation_cycle_impl tests
// ==============================================================================

#[tokio::test]
async fn test_macro_file_reservation_cycle_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = MacroFileReservationCycleParams {
        project_key: project_slug,
        agent_name: "test_agent".to_string(),
        paths: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
        ttl_seconds: 3600,
        exclusive: true,
        reason: "Development work".to_string(),
        auto_release: false,
    };

    let result = macros::macro_file_reservation_cycle_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("granted"));
}

#[tokio::test]
async fn test_macro_file_reservation_cycle_impl_with_auto_release() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = MacroFileReservationCycleParams {
        project_key: project_slug,
        agent_name: "test_agent".to_string(),
        paths: vec!["test.rs".to_string()],
        ttl_seconds: 60,
        exclusive: false,
        reason: "Quick edit".to_string(),
        auto_release: true,
    };

    let result = macros::macro_file_reservation_cycle_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("released"));
}

// ==============================================================================
// macro_contact_handshake_impl tests
// ==============================================================================

#[tokio::test]
async fn test_macro_contact_handshake_impl_basic() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent(&mm).await;

    // Create target agent
    let target_c = AgentForCreate {
        project_id: project_id.into(),
        name: "target_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Target".to_string(),
    };
    AgentBmc::create(&ctx, &mm, target_c).await.unwrap();

    let params = MacroContactHandshakeParams {
        project_key: project_slug,
        requester: Some("test_agent".to_string()),
        agent_name: None,
        target: Some("target_agent".to_string()),
        to_agent: None,
        to_project: None,
        reason: "Collaboration".to_string(),
        ttl_seconds: 604800, // 7 days
        auto_accept: false,
        welcome_subject: None,
        welcome_body: None,
        thread_id: None,
        register_if_missing: false,
        program: None,
        model: None,
    };

    let result = macros::macro_contact_handshake_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("request") && text.contains("pending"));
}

#[tokio::test]
async fn test_macro_contact_handshake_impl_with_auto_accept_and_welcome() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent(&mm).await;

    // Create target agent
    let target_c = AgentForCreate {
        project_id: project_id.into(),
        name: "target2".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Target 2".to_string(),
    };
    AgentBmc::create(&ctx, &mm, target_c).await.unwrap();

    let params = MacroContactHandshakeParams {
        project_key: project_slug,
        requester: None,
        agent_name: Some("test_agent".to_string()),
        target: None,
        to_agent: Some("target2".to_string()),
        to_project: None,
        reason: "Team collaboration".to_string(),
        ttl_seconds: 604800, // 7 days
        auto_accept: true,
        welcome_subject: Some("Welcome to the team!".to_string()),
        welcome_body: Some("Looking forward to working with you.".to_string()),
        thread_id: Some("WELCOME-THREAD".to_string()),
        register_if_missing: false,
        program: None,
        model: None,
    };

    let result = macros::macro_contact_handshake_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("accepted"));
    assert!(text.contains("welcome_message"));
}

#[tokio::test]
async fn test_macro_contact_handshake_impl_register_if_missing() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent(&mm).await;

    // Create target agent (requester will be created)
    let target_c = AgentForCreate {
        project_id: project_id.into(),
        name: "existing_target".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Existing target".to_string(),
    };
    AgentBmc::create(&ctx, &mm, target_c).await.unwrap();

    let params = MacroContactHandshakeParams {
        project_key: project_slug,
        requester: Some("new_requester".to_string()),
        agent_name: None,
        target: Some("existing_target".to_string()),
        to_agent: None,
        to_project: None,
        reason: "New agent joining".to_string(),
        ttl_seconds: 604800, // 7 days
        auto_accept: false,
        welcome_subject: None,
        welcome_body: None,
        thread_id: None,
        register_if_missing: true,
        program: Some("claude".to_string()),
        model: Some("opus".to_string()),
    };

    let result = macros::macro_contact_handshake_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_macro_contact_handshake_impl_missing_params() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    // Missing both requester and agent_name
    let params = MacroContactHandshakeParams {
        project_key: project_slug,
        requester: None,
        agent_name: None,
        target: Some("someone".to_string()),
        to_agent: None,
        to_project: None,
        reason: "Test".to_string(),
        ttl_seconds: 604800, // 7 days
        auto_accept: false,
        welcome_subject: None,
        welcome_body: None,
        thread_id: None,
        register_if_missing: false,
        program: None,
        model: None,
    };

    let result = macros::macro_contact_handshake_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail without requester");
}
