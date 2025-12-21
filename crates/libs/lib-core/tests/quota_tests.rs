//! Quota enforcement tests
//!
//! Tests for quota limits on attachments and inbox messages.

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_common::config::{AppConfig, QuotaConfig};
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::attachment::{AttachmentBmc, AttachmentForCreate};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::utils::slugify;

/// Helper to set up a project
async fn setup_project(tc: &TestContext) -> (i64, String) {
    let human_key = "Test Quota Project";
    let slug = slugify(human_key) + &uuid::Uuid::new_v4().to_string(); // Ensure unique

    let id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");
    (id, slug)
}

async fn setup_agent(tc: &TestContext, project_slug: &str, name: &str) -> (i64, String) {
    let project = ProjectBmc::get_by_slug(&tc.ctx, &tc.mm, project_slug)
        .await
        .expect("Project not found");

    let agent_c = AgentForCreate {
        project_id: project.id,
        name: name.to_string(),
        program: "test-Program".to_string(),
        model: "gpt-4".to_string(),
        task_description: "".to_string(),
    };

    let id = AgentBmc::create(&tc.ctx, &tc.mm, agent_c)
        .await
        .expect("Failed to create agent");
    (id, name.to_string())
}

#[tokio::test]
async fn test_quota_attachment_limit_exceeded() {
    let mut config = AppConfig::default();
    config.quota.enabled = true;
    config.quota.attachments_limit_bytes = 1000; // 1KB limit

    // Use new_with_config
    let tc = TestContext::new_with_config(config)
        .await
        .expect("Failed to create test context");

    let (pid, _pslug) = setup_project(&tc).await;

    // 1. Create attachment below limit (500 bytes)
    let att1 = AttachmentForCreate {
        project_id: pid,
        agent_id: None,
        filename: "small.txt".into(),
        stored_path: "/tmp/small.txt".into(),
        media_type: "text/plain".into(),
        size_bytes: 500,
    };
    AttachmentBmc::create(&tc.ctx, &tc.mm, att1)
        .await
        .expect("Should succeed");

    // 2. Create another that puts it over limit (500 + 600 = 1100 > 1000)
    let att2 = AttachmentForCreate {
        project_id: pid,
        agent_id: None,
        filename: "large.txt".into(),
        stored_path: "/tmp/large.txt".into(),
        media_type: "text/plain".into(),
        size_bytes: 600,
    };

    let res = AttachmentBmc::create(&tc.ctx, &tc.mm, att2).await;
    match res {
        Err(lib_core::Error::QuotaExceeded(msg)) => {
            assert!(msg.contains("Attachments limit reached"));
        }
        _ => panic!("Expected QuotaExceeded error, got {:?}", res),
    }
}

#[tokio::test]
async fn test_quota_inbox_limit_exceeded() {
    let mut config = AppConfig::default();
    config.quota.enabled = true;
    config.quota.inbox_limit_count = 2; // Max 2 messages

    let tc = TestContext::new_with_config(config)
        .await
        .expect("Failed to create test context");

    let (pid, p_slug) = setup_project(&tc).await;
    let (aid_sender, _) = setup_agent(&tc, &p_slug, "sender").await;
    let (aid_recipient, _) = setup_agent(&tc, &p_slug, "recipient").await;

    // 1. Send first message
    let msg1 = MessageForCreate {
        project_id: pid,
        sender_id: aid_sender,
        recipient_ids: vec![aid_recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Msg 1".into(),
        body_md: "Body 1".into(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg1)
        .await
        .expect("Should succeed");

    // 2. Send second message (Limit is 2, current count is 1. 1 >= 2 is False. Succeeds)
    let msg2 = MessageForCreate {
        project_id: pid,
        sender_id: aid_sender,
        recipient_ids: vec![aid_recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Msg 2".into(),
        body_md: "Body 2".into(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg2)
        .await
        .expect("Should succeed");

    // 3. Send third message (Current count is 2. 2 >= 2 is True. Fails)
    let msg3 = MessageForCreate {
        project_id: pid,
        sender_id: aid_sender,
        recipient_ids: vec![aid_recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Msg 3".into(),
        body_md: "Body 3".into(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let res = MessageBmc::create(&tc.ctx, &tc.mm, msg3).await;
    match res {
        Err(lib_core::Error::QuotaExceeded(msg)) => {
            assert!(msg.contains("Inbox limit reached"));
        }
        _ => panic!("Expected QuotaExceeded error, got {:?}", res),
    }
}
