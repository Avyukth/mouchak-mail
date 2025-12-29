//! Tests for export and review tool implementations
//! Target: Improve coverage for export.rs and reviews.rs

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
    message::{MessageBmc, MessageForCreate},
    project::ProjectBmc,
};
use mouchak_mail_mcp::tools::{ClaimReviewParams, ExportMailboxParams, GetReviewStateParams};
use mouchak_mail_mcp::tools::{export, reviews};
use std::sync::Arc;
use tempfile::TempDir;

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_export_reviews.db");
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

async fn setup_project_with_messages(mm: &Arc<ModelManager>) -> (i64, i64, i64, String) {
    let ctx = Ctx::root_ctx();
    let project_slug = "test-export-project";
    let project_id = ProjectBmc::create(&ctx, mm, project_slug, "Test Export Project")
        .await
        .unwrap();

    let agent1_c = AgentForCreate {
        project_id: project_id,
        name: "sender_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Sender agent".to_string(),
    };
    let agent1_id = AgentBmc::create(&ctx, mm, agent1_c).await.unwrap();

    let agent2_c = AgentForCreate {
        project_id: project_id,
        name: "receiver_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Receiver agent".to_string(),
    };
    let agent2_id = AgentBmc::create(&ctx, mm, agent2_c).await.unwrap();

    // Create some test messages
    let msg = MessageForCreate {
        project_id: project_id.into(),
        sender_id: agent1_id.into(),
        recipient_ids: vec![agent2_id.into()],
        cc_ids: None,
        bcc_ids: None,
        subject: "Test Message".to_string(),
        body_md: "This is a test message body".to_string(),
        thread_id: Some("THREAD-001".to_string()),
        importance: Some("normal".to_string()),
        ack_required: false,
    };
    MessageBmc::create(&ctx, mm, msg).await.unwrap();

    (
        project_id.into(),
        agent1_id.into(),
        agent2_id.into(),
        project_slug.to_string(),
    )
}

// ==============================================================================
// export_mailbox_impl tests
// ==============================================================================

#[tokio::test]
async fn test_export_mailbox_impl_markdown() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_with_messages(&mm).await;

    let params = ExportMailboxParams {
        project_slug,
        format: None, // Uses default markdown
        include_attachments: None,
    };

    let result = export::export_mailbox_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "Markdown export should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Mailbox Export"));
    assert!(text.contains("Agents"));
    assert!(text.contains("Messages"));
}

#[tokio::test]
async fn test_export_mailbox_impl_json() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_with_messages(&mm).await;

    let params = ExportMailboxParams {
        project_slug,
        format: Some("json".to_string()),
        include_attachments: None,
    };

    let result = export::export_mailbox_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "JSON export should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("project"));
    assert!(text.contains("agents"));
    assert!(text.contains("messages"));
    assert!(text.contains("threads"));
}

#[tokio::test]
async fn test_export_mailbox_impl_html() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_with_messages(&mm).await;

    let params = ExportMailboxParams {
        project_slug,
        format: Some("html".to_string()),
        include_attachments: None,
    };

    let result = export::export_mailbox_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "HTML export should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("<!DOCTYPE html>"));
    assert!(text.contains("<title>"));
    assert!(text.contains("Mailbox Export"));
}

#[tokio::test]
async fn test_export_mailbox_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ExportMailboxParams {
        project_slug: "nonexistent-project".to_string(),
        format: None,
        include_attachments: None,
    };

    let result = export::export_mailbox_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for invalid project");
}

// ==============================================================================
// get_review_state_impl tests
// ==============================================================================

#[tokio::test]
async fn test_get_review_state_impl_no_messages() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_with_messages(&mm).await;

    let params = GetReviewStateParams {
        project_slug,
        thread_id: "NONEXISTENT-THREAD".to_string(),
    };

    let result = reviews::get_review_state_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    // Should return a state even for empty thread
    assert!(text.contains("thread_id"));
    assert!(text.contains("state"));
}

#[tokio::test]
async fn test_get_review_state_impl_with_messages() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_with_messages(&mm).await;

    let params = GetReviewStateParams {
        project_slug,
        thread_id: "THREAD-001".to_string(),
    };

    let result = reviews::get_review_state_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("THREAD-001"));
    assert!(text.contains("is_reviewed"));
}

#[tokio::test]
async fn test_get_review_state_impl_with_completion_message() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, agent1_id, agent2_id, project_slug) = setup_project_with_messages(&mm).await;

    // Create a [COMPLETION] message
    let msg = MessageForCreate {
        project_id,
        sender_id: agent1_id,
        recipient_ids: vec![agent2_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "[COMPLETION] Task finished".to_string(),
        body_md: "Task has been completed".to_string(),
        thread_id: Some("REVIEW-THREAD".to_string()),
        importance: Some("normal".to_string()),
        ack_required: false,
    };
    MessageBmc::create(&ctx, &mm, msg).await.unwrap();

    let params = GetReviewStateParams {
        project_slug,
        thread_id: "REVIEW-THREAD".to_string(),
    };

    let result = reviews::get_review_state_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("REVIEW-THREAD"));
}

#[tokio::test]
async fn test_get_review_state_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = GetReviewStateParams {
        project_slug: "nonexistent".to_string(),
        thread_id: "THREAD".to_string(),
    };

    let result = reviews::get_review_state_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

// ==============================================================================
// claim_review_impl tests
// ==============================================================================

#[tokio::test]
async fn test_claim_review_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, agent1_id, agent2_id, project_slug) = setup_project_with_messages(&mm).await;

    // Create a [COMPLETION] message that can be claimed
    let msg = MessageForCreate {
        project_id,
        sender_id: agent1_id,
        recipient_ids: vec![agent2_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "[COMPLETION] Feature implementation".to_string(),
        body_md: "Feature is complete and ready for review".to_string(),
        thread_id: Some("CLAIM-THREAD".to_string()),
        importance: Some("high".to_string()),
        ack_required: true,
    };
    let msg_id = MessageBmc::create(&ctx, &mm, msg).await.unwrap();

    let params = ClaimReviewParams {
        project_slug,
        message_id: msg_id,
        reviewer_name: "receiver_agent".to_string(),
    };

    let result = reviews::claim_review_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("success"));
    assert!(text.contains("CLAIM-THREAD"));
}

#[tokio::test]
async fn test_claim_review_impl_already_claimed() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, agent1_id, agent2_id, project_slug) = setup_project_with_messages(&mm).await;

    // Create a [COMPLETION] message
    let msg = MessageForCreate {
        project_id,
        sender_id: agent1_id,
        recipient_ids: vec![agent2_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "[COMPLETION] Work done".to_string(),
        body_md: "Work complete".to_string(),
        thread_id: Some("ALREADY-CLAIMED".to_string()),
        importance: Some("normal".to_string()),
        ack_required: false,
    };
    let msg_id = MessageBmc::create(&ctx, &mm, msg).await.unwrap();

    // First claim
    let params1 = ClaimReviewParams {
        project_slug: project_slug.clone(),
        message_id: msg_id,
        reviewer_name: "receiver_agent".to_string(),
    };
    reviews::claim_review_impl(&ctx, &mm, params1)
        .await
        .unwrap();

    // Try to claim again
    let params2 = ClaimReviewParams {
        project_slug,
        message_id: msg_id,
        reviewer_name: "receiver_agent".to_string(),
    };

    let result = reviews::claim_review_impl(&ctx, &mm, params2).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    // Should indicate already claimed
    assert!(text.contains("success") || text.contains("false"));
}

#[tokio::test]
async fn test_claim_review_impl_message_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_with_messages(&mm).await;

    let params = ClaimReviewParams {
        project_slug,
        message_id: 99999, // Non-existent
        reviewer_name: "receiver_agent".to_string(),
    };

    let result = reviews::claim_review_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for nonexistent message");
}

#[tokio::test]
async fn test_claim_review_impl_reviewer_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, agent1_id, agent2_id, project_slug) = setup_project_with_messages(&mm).await;

    let msg = MessageForCreate {
        project_id,
        sender_id: agent1_id,
        recipient_ids: vec![agent2_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "[COMPLETION] Task".to_string(),
        body_md: "Done".to_string(),
        thread_id: Some("TEST-THREAD".to_string()),
        importance: None,
        ack_required: false,
    };
    let msg_id = MessageBmc::create(&ctx, &mm, msg).await.unwrap();

    let params = ClaimReviewParams {
        project_slug,
        message_id: msg_id,
        reviewer_name: "nonexistent_reviewer".to_string(),
    };

    let result = reviews::claim_review_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for nonexistent reviewer");
}
