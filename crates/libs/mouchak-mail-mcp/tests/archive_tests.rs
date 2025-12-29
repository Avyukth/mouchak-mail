#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::redundant_field_names
)]

use libsql::Builder;
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    project::ProjectBmc,
};
use mouchak_mail_core::store::git_store;
use mouchak_mail_mcp::tools::CommitArchiveParams;
use mouchak_mail_mcp::tools::archive;
use std::sync::Arc;
use tempfile::TempDir;

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_archive.db");
    let archive_root = temp_dir.path().join("archive");
    std::fs::create_dir_all(&archive_root).unwrap();

    git_store::init_or_open_repo(&archive_root).expect("Failed to init git repo");

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

async fn setup_project_and_agent(mm: &Arc<ModelManager>) -> (i64, String) {
    let ctx = Ctx::root_ctx();
    let project_slug = format!("test-archive-project-{}", uuid::Uuid::new_v4());
    let project_id = ProjectBmc::create(&ctx, mm, &project_slug, "Test Archive Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id: project_id,
        name: "archive_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Archive agent for testing".to_string(),
    };
    AgentBmc::create(&ctx, mm, agent_c).await.unwrap();

    (project_id.into(), project_slug)
}

#[tokio::test]
async fn test_commit_archive_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, project_slug) = setup_project_and_agent(&mm).await;

    let params = CommitArchiveParams {
        project_slug: project_slug.clone(),
        message: "Test archive commit".to_string(),
    };

    let result = archive::commit_archive_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "commit_archive should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Archived project"));
    assert!(text.contains(&project_slug));
}

#[tokio::test]
async fn test_commit_archive_impl_with_custom_message() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, project_slug) = setup_project_and_agent(&mm).await;

    let custom_message = "Custom archive message: deploying v1.0";
    let params = CommitArchiveParams {
        project_slug: project_slug.clone(),
        message: custom_message.to_string(),
    };

    let result = archive::commit_archive_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_commit_archive_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = CommitArchiveParams {
        project_slug: "nonexistent-project".to_string(),
        message: "Should fail".to_string(),
    };

    let result = archive::commit_archive_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for invalid project");
}

#[tokio::test]
async fn test_commit_archive_impl_multiple_commits() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, project_slug) = setup_project_and_agent(&mm).await;

    for i in 1..=3 {
        let params = CommitArchiveParams {
            project_slug: project_slug.clone(),
            message: format!("Archive commit #{}", i),
        };
        let result = archive::commit_archive_impl(&ctx, &mm, params).await;
        assert!(result.is_ok(), "Commit {} should succeed", i);
    }
}

#[tokio::test]
async fn test_commit_archive_impl_empty_message() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, project_slug) = setup_project_and_agent(&mm).await;

    let params = CommitArchiveParams {
        project_slug: project_slug.clone(),
        message: String::new(),
    };

    let result = archive::commit_archive_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}
