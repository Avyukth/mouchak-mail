//! Integration tests for workflow macro MCP tools
//! Following extreme TDD: Tests written BEFORE implementation

use lib_common::config::AppConfig;
use lib_core::ctx::Ctx;
use lib_core::model::{
    ModelManager,
    macro_def::{MacroDefBmc, MacroDefForCreate},
    project::ProjectBmc,
};
use libsql::Builder;
use std::sync::Arc;
use tempfile::TempDir;

#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_macros.db");
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
// RED PHASE: Write failing tests FIRST (before implementation)
// ==============================================================================

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_unregister_macro_removes_from_database() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    // Setup: Create project and register a macro
    let project_id = ProjectBmc::create(&ctx, &mm, "test-project", "/test")
        .await
        .unwrap();

    let macro_c = MacroDefForCreate {
        project_id,
        name: "test-workflow".to_string(),
        description: "Test workflow".to_string(),
        steps: vec![serde_json::json!({"action": "test"})],
    };
    let _macro_id = MacroDefBmc::create(&ctx, &mm, macro_c).await.unwrap();

    // Verify macro exists (along with 5 built-in macros = 6 total)
    let macros = MacroDefBmc::list(&ctx, &mm, project_id).await.unwrap();
    assert_eq!(macros.len(), 6, "Should have 5 built-in + 1 test macro");

    // GREEN: Call MacroDefBmc::delete directly
    let deleted = MacroDefBmc::delete(&ctx, &mm, project_id, "test-workflow")
        .await
        .unwrap();
    assert!(deleted, "delete should return true");

    // Verify custom macro is gone (5 built-in macros remain)
    let macros = MacroDefBmc::list(&ctx, &mm, project_id).await.unwrap();
    assert_eq!(
        macros.len(),
        5,
        "Should have 5 built-in macros after deleting custom one"
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_unregister_nonexistent_macro_returns_false() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "test-project", "/test")
        .await
        .unwrap();

    // Try to delete a macro that doesn't exist
    let deleted = MacroDefBmc::delete(&ctx, &mm, project_id, "nonexistent-macro")
        .await
        .unwrap();
    assert!(!deleted, "Should return false for nonexistent macro");
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_list_returns_correct_macros() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "test-project", "/test")
        .await
        .unwrap();

    // Register 2 macros using BMC directly
    let macro1 = MacroDefForCreate {
        project_id,
        name: "workflow-1".to_string(),
        description: "First workflow".to_string(),
        steps: vec![serde_json::json!({"action": "step1"})],
    };
    MacroDefBmc::create(&ctx, &mm, macro1).await.unwrap();

    let macro2 = MacroDefForCreate {
        project_id,
        name: "workflow-2".to_string(),
        description: "Second workflow".to_string(),
        steps: vec![serde_json::json!({"action": "step2"})],
    };
    MacroDefBmc::create(&ctx, &mm, macro2).await.unwrap();

    // Use lib-core list method directly (5 built-in + 2 custom = 7)
    let macros = MacroDefBmc::list(&ctx, &mm, project_id).await.unwrap();

    assert_eq!(macros.len(), 7, "Should have 5 built-in + 2 custom macros");
    assert!(macros.iter().any(|m| m.name == "workflow-1"));
    assert!(macros.iter().any(|m| m.name == "workflow-2"));
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_create_and_get_macro_by_name() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "test-project", "/test")
        .await
        .unwrap();

    let macro_c = MacroDefForCreate {
        project_id,
        name: "new-workflow".to_string(),
        description: "New workflow".to_string(),
        steps: vec![serde_json::json!({"action": "deploy"})],
    };

    let macro_id = MacroDefBmc::create(&ctx, &mm, macro_c).await.unwrap();
    assert!(macro_id > 0);

    // Verify it exists via get_by_name
    let retrieved = MacroDefBmc::get_by_name(&ctx, &mm, project_id, "new-workflow")
        .await
        .unwrap();
    assert_eq!(retrieved.name, "new-workflow");
    assert_eq!(retrieved.description, "New workflow");
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::expect_used)]
async fn test_get_macro_returns_correct_steps() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "test-project", "/test")
        .await
        .unwrap();

    let macro_c = MacroDefForCreate {
        project_id,
        name: "deploy-workflow".to_string(),
        description: "Deployment workflow".to_string(),
        steps: vec![
            serde_json::json!({"action": "build"}),
            serde_json::json!({"action": "test"}),
            serde_json::json!({"action": "deploy"}),
        ],
    };
    MacroDefBmc::create(&ctx, &mm, macro_c).await.unwrap();

    let retrieved = MacroDefBmc::get_by_name(&ctx, &mm, project_id, "deploy-workflow")
        .await
        .unwrap();

    assert_eq!(retrieved.steps.len(), 3);
    assert_eq!(retrieved.steps[0]["action"], "build");
    assert_eq!(retrieved.steps[1]["action"], "test");
    assert_eq!(retrieved.steps[2]["action"], "deploy");
}
