//! Input validation tests for MCP tools - NIST SC-5 compliance
//! TDD RED phase: failing tests for validation guards

#![allow(clippy::unwrap_used, clippy::expect_used)]

use libsql::Builder;
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    project::ProjectBmc,
};
use mouchak_mail_mcp::tools::helpers;
use std::sync::Arc;
use tempfile::TempDir;

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_validation.db");
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
async fn test_whois_rejects_invalid_agent_name_special_chars() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let result = helpers::resolve_agent(&ctx, &mm, 1, "invalid-agent!@#$").await;

    assert!(
        result.is_err(),
        "Should reject agent name with special chars"
    );
    let err = result.unwrap_err();
    assert!(
        err.message.contains("agent") || err.message.contains("Agent"),
        "Error should mention agent name issue: {:?}",
        err
    );
}

#[tokio::test]
async fn test_whois_rejects_agent_name_with_spaces() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let result = helpers::resolve_agent(&ctx, &mm, 1, "my agent").await;

    assert!(result.is_err(), "Should reject agent name with spaces");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("agent") || err.message.contains("Agent"),
        "Error should mention agent name issue: {:?}",
        err
    );
}

#[tokio::test]
async fn test_whois_rejects_sql_injection_attempt() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let result = helpers::resolve_agent(&ctx, &mm, 1, "'; DROP TABLE agents; --").await;

    assert!(
        result.is_err(),
        "Should reject SQL injection attempt in agent name"
    );
}

#[tokio::test]
async fn test_whois_rejects_empty_agent_name() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let result = helpers::resolve_agent(&ctx, &mm, 1, "").await;

    assert!(result.is_err(), "Should reject empty agent name");
}

#[tokio::test]
async fn test_whois_rejects_too_long_agent_name() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let long_name = "a".repeat(100);
    let result = helpers::resolve_agent(&ctx, &mm, 1, &long_name).await;

    assert!(
        result.is_err(),
        "Should reject agent name longer than 64 chars"
    );
}

#[tokio::test]
async fn test_resolve_project_rejects_invalid_slug() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let result = helpers::resolve_project(&ctx, &mm, "invalid slug!@#$").await;

    assert!(result.is_err(), "Should reject invalid project slug");
}

#[tokio::test]
async fn test_resolve_project_accepts_valid_absolute_path() {
    let (mm, temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    // Use the temp directory path which is a valid absolute path
    let project_path = temp.path().to_string_lossy().to_string();

    ProjectBmc::create(&ctx, &mm, &project_path, "test-abs-path")
        .await
        .unwrap();

    let result = helpers::resolve_project(&ctx, &mm, &project_path).await;
    assert!(result.is_ok(), "Should accept valid absolute path");
}

#[tokio::test]
async fn test_resolve_project_accepts_valid_human_key() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    ProjectBmc::create(&ctx, &mm, "valid-key", "valid-key")
        .await
        .unwrap();

    let result = helpers::resolve_project(&ctx, &mm, "valid-key").await;
    assert!(result.is_ok(), "Should accept valid human_key");
}

#[tokio::test]
async fn test_validation_error_includes_suggestion() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_project_id, _agent_id) = setup_test_project_and_agent(&mm).await;

    let result = helpers::resolve_agent(&ctx, &mm, 1, "my-agent").await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    if let Some(data) = &err.data {
        let data_str = data.to_string();
        assert!(
            data_str.contains("suggestion") || data_str.contains("myagent"),
            "Error data should contain suggestion: {}",
            data_str
        );
    }
}

#[tokio::test]
async fn test_resolve_agent_validates_before_db_lookup() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let result = helpers::resolve_agent(&ctx, &mm, 9999, "invalid!name").await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("agent") || err.message.contains("Agent"),
        "Error should be about invalid agent name format, not about agent not found: {:?}",
        err
    );
}

#[tokio::test]
async fn test_resolve_project_and_agent_validates_both() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let result =
        helpers::resolve_project_and_agent(&ctx, &mm, "invalid slug!", "invalid!agent").await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("Project") || err.message.contains("project"),
        "Should fail on project validation first: {:?}",
        err
    );
}
