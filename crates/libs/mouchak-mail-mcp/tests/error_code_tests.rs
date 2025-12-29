#![allow(clippy::unwrap_used, clippy::expect_used)]

use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    project::ProjectBmc,
};
use mouchak_mail_mcp::tools::helpers;
use libsql::Builder;
use std::sync::Arc;
use tempfile::TempDir;

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_error_codes.db");
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

async fn setup_test_project_and_agent(mm: &Arc<ModelManager>) -> (i64, i64) {
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, mm, "test-project", "test-project")
        .await
        .unwrap();

    let agent_id = AgentBmc::create(
        &ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "valid_agent".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "Test agent".to_string(),
        },
    )
    .await
    .unwrap();

    (project_id.into(), agent_id.into())
}

#[tokio::test]
async fn test_agent_not_found_includes_error_code() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let result = helpers::resolve_agent(&ctx, &mm, project_id, "nonexistent_agent").await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    let data = err.data.as_ref().expect("error should have data");
    let error_code = data.get("error_code").expect("should have error_code");
    assert_eq!(error_code, "AGENT_NOT_FOUND");
}

#[tokio::test]
async fn test_agent_not_found_includes_suggestion() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let result = helpers::resolve_agent(&ctx, &mm, project_id, "nonexistent_agent").await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    let data = err.data.as_ref().expect("error should have data");
    let suggestion = data.get("suggestion").expect("should have suggestion");
    assert!(
        suggestion.as_str().unwrap().contains("list_agents"),
        "Suggestion should mention list_agents"
    );
}

#[tokio::test]
async fn test_agent_not_found_includes_context() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let result = helpers::resolve_agent(&ctx, &mm, project_id, "nonexistent_agent").await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    let data = err.data.as_ref().expect("error should have data");
    assert_eq!(
        data.get("agent_name").expect("should have agent_name"),
        "nonexistent_agent"
    );
    assert_eq!(
        data.get("project_id")
            .expect("should have project_id")
            .as_i64()
            .unwrap(),
        project_id
    );
}

#[tokio::test]
async fn test_project_not_found_includes_error_code() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let result = helpers::resolve_project(&ctx, &mm, "nonexistent-project").await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    let data = err.data.as_ref().expect("error should have data");
    let error_code = data.get("error_code").expect("should have error_code");
    assert_eq!(error_code, "PROJECT_NOT_FOUND");
}

#[tokio::test]
async fn test_project_not_found_includes_suggestion() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let result = helpers::resolve_project(&ctx, &mm, "nonexistent-project").await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    let data = err.data.as_ref().expect("error should have data");
    let suggestion = data.get("suggestion").expect("should have suggestion");
    assert!(
        suggestion.as_str().unwrap().contains("list_projects"),
        "Suggestion should mention list_projects"
    );
}

#[tokio::test]
async fn test_invalid_agent_name_includes_error_code() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let result = helpers::resolve_agent(&ctx, &mm, project_id, "invalid!agent@name").await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    let data = err.data.as_ref().expect("error should have data");
    let error_code = data.get("error_code").expect("should have error_code");
    assert_eq!(error_code, "INVALID_AGENT_NAME");
}

#[tokio::test]
async fn test_invalid_project_key_includes_error_code() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let result = helpers::resolve_project(&ctx, &mm, "invalid project!key").await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    let data = err.data.as_ref().expect("error should have data");
    let error_code = data.get("error_code").expect("should have error_code");
    assert_eq!(error_code, "INVALID_PROJECT_KEY");
}

#[tokio::test]
async fn test_error_code_in_combined_lookup() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let result = helpers::resolve_project_and_agent(&ctx, &mm, "test-project", "nonexistent").await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    let data = err.data.as_ref().expect("error should have data");
    let error_code = data.get("error_code").expect("should have error_code");
    assert_eq!(error_code, "AGENT_NOT_FOUND");
}
