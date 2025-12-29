//! Tests for messaging tool implementations
//!
//! Target: Full coverage for lib-mcp/src/tools/messaging.rs

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
    agent_capabilities::{AgentCapabilityBmc, AgentCapabilityForCreate},
    message::{MessageBmc, MessageForCreate},
    project::ProjectBmc,
};
use mouchak_mail_mcp::tools::messaging;
use mouchak_mail_mcp::tools::{
    AcknowledgeMessageParams, GetMessageParams, GetThreadParams, ListInboxParams,
    ListThreadsParams, MarkMessageReadParams, ReplyMessageParams, SearchMessagesParams,
    SendMessageParams,
};
use std::sync::Arc;
use tempfile::TempDir;

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_messaging.db");
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

async fn setup_project_and_agents(mm: &Arc<ModelManager>) -> (i64, i64, i64, String) {
    let ctx = Ctx::root_ctx();
    let project_slug = "test-messaging-project";
    let project_id = ProjectBmc::create(&ctx, mm, project_slug, "Test Messaging Project")
        .await
        .unwrap();

    let sender_c = AgentForCreate {
        project_id: project_id,
        name: "sender_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Sender agent".to_string(),
    };
    let sender_id = AgentBmc::create(&ctx, mm, sender_c).await.unwrap();

    let receiver_c = AgentForCreate {
        project_id: project_id,
        name: "receiver_agent".to_string(),
        program: "claude".to_string(),
        model: "sonnet".to_string(),
        task_description: "Receiver agent".to_string(),
    };
    let receiver_id = AgentBmc::create(&ctx, mm, receiver_c).await.unwrap();

    let cap_send = AgentCapabilityForCreate {
        agent_id: sender_id.into(),
        capability: "send_message".to_string(),
        granted_by: None,
        expires_at: None,
    };
    AgentCapabilityBmc::create(&ctx, mm, cap_send)
        .await
        .unwrap();

    let cap_inbox = AgentCapabilityForCreate {
        agent_id: receiver_id.into(),
        capability: "fetch_inbox".to_string(),
        granted_by: None,
        expires_at: None,
    };
    AgentCapabilityBmc::create(&ctx, mm, cap_inbox)
        .await
        .unwrap();

    let cap_ack = AgentCapabilityForCreate {
        agent_id: receiver_id.into(),
        capability: "acknowledge_message".to_string(),
        granted_by: None,
        expires_at: None,
    };
    AgentCapabilityBmc::create(&ctx, mm, cap_ack).await.unwrap();

    let cap_send_recv = AgentCapabilityForCreate {
        agent_id: receiver_id.into(),
        capability: "send_message".to_string(),
        granted_by: None,
        expires_at: None,
    };
    AgentCapabilityBmc::create(&ctx, mm, cap_send_recv)
        .await
        .unwrap();

    (
        project_id.into(),
        sender_id.into(),
        receiver_id.into(),
        project_slug.to_string(),
    )
}

#[tokio::test]
async fn test_send_message_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_and_agents(&mm).await;

    let params = SendMessageParams {
        project_slug: project_slug.clone(),
        sender_name: "sender_agent".to_string(),
        to: "receiver_agent".to_string(),
        cc: None,
        bcc: None,
        subject: "Test Subject".to_string(),
        body_md: "This is a test message body.".to_string(),
        thread_id: Some("THREAD-001".to_string()),
        importance: Some("high".to_string()),
        ack_required: Some(true),
    };

    let result = messaging::send_message_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "send_message should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Message sent"));
    assert!(text.contains("sender_agent"));
    assert!(text.contains("Test Subject"));
}

#[tokio::test]
async fn test_send_message_impl_with_cc_bcc() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, _, project_slug) = setup_project_and_agents(&mm).await;

    let cc_agent_c = AgentForCreate {
        project_id: project_id.into(),
        name: "cc_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "CC agent".to_string(),
    };
    AgentBmc::create(&ctx, &mm, cc_agent_c).await.unwrap();

    let bcc_agent_c = AgentForCreate {
        project_id: project_id.into(),
        name: "bcc_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "BCC agent".to_string(),
    };
    AgentBmc::create(&ctx, &mm, bcc_agent_c).await.unwrap();

    let params = SendMessageParams {
        project_slug: project_slug.clone(),
        sender_name: "sender_agent".to_string(),
        to: "receiver_agent".to_string(),
        cc: Some("cc_agent".to_string()),
        bcc: Some("bcc_agent".to_string()),
        subject: "CC/BCC Test".to_string(),
        body_md: "Testing CC and BCC.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: None,
    };

    let result = messaging::send_message_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_send_message_impl_without_capability() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, _, project_slug) = setup_project_and_agents(&mm).await;

    let no_cap_agent_c = AgentForCreate {
        project_id: project_id.into(),
        name: "no_cap_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent without send capability".to_string(),
    };
    AgentBmc::create(&ctx, &mm, no_cap_agent_c).await.unwrap();

    let params = SendMessageParams {
        project_slug: project_slug.clone(),
        sender_name: "no_cap_agent".to_string(),
        to: "receiver_agent".to_string(),
        cc: None,
        bcc: None,
        subject: "Should Fail".to_string(),
        body_md: "This should fail.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: None,
    };

    let result = messaging::send_message_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message
            .contains("does not have 'send_message' capability")
    );
}

#[tokio::test]
async fn test_send_message_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = SendMessageParams {
        project_slug: "nonexistent".to_string(),
        sender_name: "someone".to_string(),
        to: "someone_else".to_string(),
        cc: None,
        bcc: None,
        subject: "Test".to_string(),
        body_md: "Test".to_string(),
        thread_id: None,
        importance: None,
        ack_required: None,
    };

    let result = messaging::send_message_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_inbox_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![receiver_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Inbox Test".to_string(),
        body_md: "Message for inbox test.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

    let params = ListInboxParams {
        project_slug: project_slug.clone(),
        agent_name: "receiver_agent".to_string(),
        limit: Some(10),
    };

    let result = messaging::list_inbox_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Inbox for 'receiver_agent'"));
    assert!(text.contains("Inbox Test"));
}

#[tokio::test]
async fn test_list_inbox_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_and_agents(&mm).await;

    let params = ListInboxParams {
        project_slug: project_slug.clone(),
        agent_name: "receiver_agent".to_string(),
        limit: Some(10),
    };

    let result = messaging::list_inbox_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("0 messages"));
}

#[tokio::test]
async fn test_list_inbox_impl_without_capability() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_and_agents(&mm).await;

    let params = ListInboxParams {
        project_slug: project_slug.clone(),
        agent_name: "sender_agent".to_string(),
        limit: None,
    };

    let result = messaging::list_inbox_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message
            .contains("does not have 'fetch_inbox' capability")
    );
}

#[tokio::test]
async fn test_get_message_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, _) = setup_project_and_agents(&mm).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![receiver_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Get Message Test".to_string(),
        body_md: "Body content for get message test.".to_string(),
        thread_id: Some("THREAD-GET".to_string()),
        importance: Some("normal".to_string()),
        ack_required: false,
    };
    let msg_id = MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

    let params = GetMessageParams { message_id: msg_id };

    let result = messaging::get_message_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Get Message Test"));
    assert!(text.contains("Body content for get message test"));
    assert!(text.contains("THREAD-GET"));
}

#[tokio::test]
async fn test_get_message_impl_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = GetMessageParams { message_id: 999999 };

    let result = messaging::get_message_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("Message not found"));
}

#[tokio::test]
async fn test_search_messages_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![receiver_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Searchable Subject".to_string(),
        body_md: "This message contains unique_keyword_xyz.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

    let params = SearchMessagesParams {
        project_slug: project_slug.clone(),
        query: "unique_keyword_xyz".to_string(),
        limit: Some(10),
    };

    let result = messaging::search_messages_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Searchable Subject"));
}

#[tokio::test]
async fn test_search_messages_impl_no_results() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_and_agents(&mm).await;

    let params = SearchMessagesParams {
        project_slug: project_slug.clone(),
        query: "nonexistent_term_abcxyz".to_string(),
        limit: None,
    };

    let result = messaging::search_messages_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("0 matches"));
}

#[tokio::test]
async fn test_get_thread_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    for i in 1..=3 {
        let msg_c = MessageForCreate {
            project_id,
            sender_id,
            recipient_ids: vec![receiver_id],
            cc_ids: None,
            bcc_ids: None,
            subject: format!("Thread Message {}", i),
            body_md: format!("Message {} in thread.", i),
            thread_id: Some("THREAD-TEST".to_string()),
            importance: None,
            ack_required: false,
        };
        MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();
    }

    let params = GetThreadParams {
        project_slug: project_slug.clone(),
        thread_id: "THREAD-TEST".to_string(),
    };

    let result = messaging::get_thread_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("THREAD-TEST"));
    assert!(text.contains("3 messages"));
}

#[tokio::test]
async fn test_get_thread_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_and_agents(&mm).await;

    let params = GetThreadParams {
        project_slug: project_slug.clone(),
        thread_id: "NONEXISTENT-THREAD".to_string(),
    };

    let result = messaging::get_thread_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("0 messages"));
}

#[tokio::test]
async fn test_reply_message_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![receiver_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Original Message".to_string(),
        body_md: "This is the original.".to_string(),
        thread_id: Some("REPLY-THREAD".to_string()),
        importance: None,
        ack_required: false,
    };
    let original_msg_id = MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

    let params = ReplyMessageParams {
        project_slug: project_slug.clone(),
        sender_name: "receiver_agent".to_string(),
        message_id: original_msg_id,
        body_md: "This is my reply.".to_string(),
        importance: Some("normal".to_string()),
    };

    let result = messaging::reply_message_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Reply sent"));
    assert!(text.contains("Re: Original Message"));
}

#[tokio::test]
async fn test_reply_message_impl_no_thread_id() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![receiver_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "No Thread Message".to_string(),
        body_md: "This message has no thread_id.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    let original_msg_id = MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

    let params = ReplyMessageParams {
        project_slug: project_slug.clone(),
        sender_name: "receiver_agent".to_string(),
        message_id: original_msg_id,
        body_md: "Replying to message with no thread.".to_string(),
        importance: None,
    };

    let result = messaging::reply_message_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Reply sent"));
    assert!(text.contains("Re: No Thread Message"));
}

#[tokio::test]
async fn test_reply_message_impl_already_has_re_prefix() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![receiver_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Re: Already Replied".to_string(),
        body_md: "Previous reply.".to_string(),
        thread_id: Some("RE-THREAD".to_string()),
        importance: None,
        ack_required: false,
    };
    let msg_id = MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

    let params = ReplyMessageParams {
        project_slug: project_slug.clone(),
        sender_name: "receiver_agent".to_string(),
        message_id: msg_id,
        body_md: "Another reply.".to_string(),
        importance: None,
    };

    let result = messaging::reply_message_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Re: Already Replied"));
    assert!(!text.contains("Re: Re:"));
}

#[tokio::test]
async fn test_reply_message_impl_message_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_and_agents(&mm).await;

    let params = ReplyMessageParams {
        project_slug: project_slug.clone(),
        sender_name: "receiver_agent".to_string(),
        message_id: 999999,
        body_md: "Reply to nonexistent.".to_string(),
        importance: None,
    };

    let result = messaging::reply_message_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("Message not found"));
}

#[tokio::test]
async fn test_mark_message_read_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![receiver_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Read Test".to_string(),
        body_md: "Mark me as read.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    let msg_id = MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

    let params = MarkMessageReadParams {
        project_slug: project_slug.clone(),
        agent_name: "receiver_agent".to_string(),
        message_id: msg_id,
    };

    let result = messaging::mark_message_read_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("marked as read"));
    assert!(text.contains("receiver_agent"));
}

#[tokio::test]
async fn test_acknowledge_message_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![receiver_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Ack Required".to_string(),
        body_md: "Please acknowledge.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let msg_id = MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

    let params = AcknowledgeMessageParams {
        project_slug: project_slug.clone(),
        agent_name: "receiver_agent".to_string(),
        message_id: msg_id,
    };

    let result = messaging::acknowledge_message_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("acknowledged"));
    assert!(text.contains("receiver_agent"));
}

#[tokio::test]
async fn test_acknowledge_message_impl_without_capability() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![receiver_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Ack Test".to_string(),
        body_md: "Ack test message.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let msg_id = MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

    let params = AcknowledgeMessageParams {
        project_slug: project_slug.clone(),
        agent_name: "sender_agent".to_string(),
        message_id: msg_id,
    };

    let result = messaging::acknowledge_message_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message
            .contains("does not have 'acknowledge_message' capability")
    );
}

#[tokio::test]
async fn test_list_threads_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    for i in 1..=3 {
        let msg_c = MessageForCreate {
            project_id,
            sender_id,
            recipient_ids: vec![receiver_id],
            cc_ids: None,
            bcc_ids: None,
            subject: format!("Thread {} Subject", i),
            body_md: format!("Message in thread {}.", i),
            thread_id: Some(format!("THREAD-{}", i)),
            importance: None,
            ack_required: false,
        };
        MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();
    }

    let params = ListThreadsParams {
        project_slug: project_slug.clone(),
        limit: Some(50),
    };

    let result = messaging::list_threads_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("THREAD-1"));
    assert!(text.contains("THREAD-2"));
    assert!(text.contains("THREAD-3"));
}

#[tokio::test]
async fn test_list_threads_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, _, project_slug) = setup_project_and_agents(&mm).await;

    let params = ListThreadsParams {
        project_slug: project_slug.clone(),
        limit: None,
    };

    let result = messaging::list_threads_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("(0)"));
}

#[tokio::test]
async fn test_send_message_impl_multiple_recipients() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, _, project_slug) = setup_project_and_agents(&mm).await;

    let agent3_c = AgentForCreate {
        project_id: project_id.into(),
        name: "third_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Third agent".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent3_c).await.unwrap();

    let params = SendMessageParams {
        project_slug: project_slug.clone(),
        sender_name: "sender_agent".to_string(),
        to: "receiver_agent,third_agent".to_string(),
        cc: None,
        bcc: None,
        subject: "Multi-recipient".to_string(),
        body_md: "Sent to multiple.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: None,
    };

    let result = messaging::send_message_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_reply_message_impl_without_capability() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, receiver_id, project_slug) = setup_project_and_agents(&mm).await;

    // Create an agent without send_message capability
    let no_cap_agent_c = AgentForCreate {
        project_id: project_id.into(),
        name: "no_reply_cap_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent without reply capability".to_string(),
    };
    AgentBmc::create(&ctx, &mm, no_cap_agent_c).await.unwrap();

    // Create a message to reply to
    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![receiver_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Test Message".to_string(),
        body_md: "Original message.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    let msg_id = MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

    // Try to reply without send_message capability
    let params = ReplyMessageParams {
        project_slug: project_slug.clone(),
        sender_name: "no_reply_cap_agent".to_string(),
        message_id: msg_id,
        body_md: "Attempted reply.".to_string(),
        importance: None,
    };

    let result = messaging::reply_message_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message
            .contains("does not have 'send_message' capability")
    );
}

#[tokio::test]
async fn test_list_inbox_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListInboxParams {
        project_slug: "nonexistent_project".to_string(),
        agent_name: "some_agent".to_string(),
        limit: None,
    };

    let result = messaging::list_inbox_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_search_messages_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = SearchMessagesParams {
        project_slug: "nonexistent_project".to_string(),
        query: "test".to_string(),
        limit: None,
    };

    let result = messaging::search_messages_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_thread_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = GetThreadParams {
        project_slug: "nonexistent_project".to_string(),
        thread_id: "THREAD-123".to_string(),
    };

    let result = messaging::get_thread_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_threads_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListThreadsParams {
        project_slug: "nonexistent_project".to_string(),
        limit: None,
    };

    let result = messaging::list_threads_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mark_message_read_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = MarkMessageReadParams {
        project_slug: "nonexistent_project".to_string(),
        agent_name: "some_agent".to_string(),
        message_id: 123,
    };

    let result = messaging::mark_message_read_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_acknowledge_message_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = AcknowledgeMessageParams {
        project_slug: "nonexistent_project".to_string(),
        agent_name: "some_agent".to_string(),
        message_id: 123,
    };

    let result = messaging::acknowledge_message_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}
