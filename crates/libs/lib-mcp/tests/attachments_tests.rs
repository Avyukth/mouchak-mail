#![allow(clippy::unwrap_used, clippy::expect_used)]

use base64::Engine;
use lib_common::config::AppConfig;
use lib_core::ctx::Ctx;
use lib_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    message::{MessageBmc, MessageForCreate},
    project::ProjectBmc,
};
use lib_core::store::git_store;
use lib_mcp::tools::attachments;
use lib_mcp::tools::{AddAttachmentParams, GetAttachmentParams};
use libsql::Builder;
use std::sync::Arc;
use tempfile::TempDir;

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_attachments.db");
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

async fn setup_project_and_message(mm: &Arc<ModelManager>) -> (i64, i64, String) {
    let ctx = Ctx::root_ctx();
    let project_slug = format!("test-attachments-project-{}", uuid::Uuid::new_v4());
    let project_id = ProjectBmc::create(&ctx, mm, &project_slug, "Test Attachments Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id: project_id.into(),
        name: "sender_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Sender agent".to_string(),
    };
    let sender_id = AgentBmc::create(&ctx, mm, agent_c).await.unwrap();

    let agent_c2 = AgentForCreate {
        project_id: project_id.into(),
        name: "receiver_agent".to_string(),
        program: "claude".to_string(),
        model: "sonnet".to_string(),
        task_description: "Receiver agent".to_string(),
    };
    let receiver_id = AgentBmc::create(&ctx, mm, agent_c2).await.unwrap();

    let msg_c = MessageForCreate {
        project_id: project_id.get(),
        sender_id: sender_id.get(),
        recipient_ids: vec![receiver_id.get()],
        cc_ids: None,
        bcc_ids: None,
        subject: "Test message".to_string(),
        body_md: "Test body".to_string(),
        thread_id: None,
        importance: Some("normal".to_string()),
        ack_required: false,
    };
    let message_id = MessageBmc::create(&ctx, mm, msg_c).await.unwrap();

    (project_id.into(), message_id, project_slug)
}

#[tokio::test]
async fn test_add_attachment_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, message_id, project_slug) = setup_project_and_message(&mm).await;

    let content = "Hello, this is test content!";
    let content_base64 = base64::engine::general_purpose::STANDARD.encode(content);

    let params = AddAttachmentParams {
        project_slug: project_slug.clone(),
        message_id,
        filename: "test.txt".to_string(),
        content_base64,
    };

    let result = attachments::add_attachment_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "add_attachment should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Attachment"));
    assert!(text.contains("test.txt"));
}

#[tokio::test]
async fn test_add_attachment_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let content_base64 = base64::engine::general_purpose::STANDARD.encode("test");

    let params = AddAttachmentParams {
        project_slug: "nonexistent-project".to_string(),
        message_id: 999999,
        filename: "test.txt".to_string(),
        content_base64,
    };

    let result = attachments::add_attachment_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for invalid project");
}

#[tokio::test]
async fn test_add_attachment_impl_invalid_message() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let project_slug = format!("test-project-{}", uuid::Uuid::new_v4());
    ProjectBmc::create(&ctx, &mm, &project_slug, "Test Project")
        .await
        .unwrap();

    let content_base64 = base64::engine::general_purpose::STANDARD.encode("test");

    let params = AddAttachmentParams {
        project_slug,
        message_id: 999999,
        filename: "test.txt".to_string(),
        content_base64,
    };

    let result = attachments::add_attachment_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for invalid message");
    let err = result.unwrap_err();
    assert!(err.message.contains("Message not found"));
}

#[tokio::test]
async fn test_add_attachment_impl_invalid_base64() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, message_id, project_slug) = setup_project_and_message(&mm).await;

    let params = AddAttachmentParams {
        project_slug,
        message_id,
        filename: "test.txt".to_string(),
        content_base64: "not-valid-base64!!!".to_string(),
    };

    let result = attachments::add_attachment_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for invalid base64");
    let err = result.unwrap_err();
    assert!(err.message.contains("Invalid base64"));
}

#[tokio::test]
async fn test_add_attachment_impl_json_file() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, message_id, project_slug) = setup_project_and_message(&mm).await;

    let json_content = r#"{"key": "value", "number": 42}"#;
    let content_base64 = base64::engine::general_purpose::STANDARD.encode(json_content);

    let params = AddAttachmentParams {
        project_slug,
        message_id,
        filename: "data.json".to_string(),
        content_base64,
    };

    let result = attachments::add_attachment_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_attachment_impl_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_message(&mm).await;

    let params = GetAttachmentParams {
        project_slug,
        attachment_id: "nonexistent_attachment".to_string(),
        filename: "missing.txt".to_string(),
    };

    let result = attachments::get_attachment_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for missing attachment");
    let err = result.unwrap_err();
    assert!(err.message.contains("Attachment not found"));
}

#[tokio::test]
async fn test_get_attachment_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = GetAttachmentParams {
        project_slug: "nonexistent-project".to_string(),
        attachment_id: "some_id".to_string(),
        filename: "test.txt".to_string(),
    };

    let result = attachments::get_attachment_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for invalid project");
}

#[tokio::test]
async fn test_add_attachment_impl_multiple_files() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, message_id, project_slug) = setup_project_and_message(&mm).await;

    let files = vec![
        ("file1.txt", "Content of file 1"),
        ("file2.md", "# Markdown content"),
        ("config.json", r#"{"enabled": true}"#),
    ];

    for (filename, content) in files {
        let content_base64 = base64::engine::general_purpose::STANDARD.encode(content);
        let params = AddAttachmentParams {
            project_slug: project_slug.clone(),
            message_id,
            filename: filename.to_string(),
            content_base64,
        };
        let result = attachments::add_attachment_impl(&ctx, &mm, params).await;
        assert!(result.is_ok(), "Adding {} should succeed", filename);
    }
}
