//! Unified Inbox BMC Tests
//!
//! Tests for the global unified inbox that shows messages across ALL projects.
//! Following EXTREME TDD: these tests are written FIRST (RED phase).

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use mouchak_mail_core::model::agent::{AgentBmc, AgentForCreate};
use mouchak_mail_core::model::message::{ImportanceFilter, MessageBmc, MessageForCreate};
use mouchak_mail_core::model::project::ProjectBmc;
use mouchak_mail_core::utils::slugify;

/// Helper to set up a project with agents for testing
async fn setup_project_with_agents(tc: &TestContext, human_key: &str) -> (i64, i64, i64) {
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    let sender_c = AgentForCreate {
        project_id: project.id,
        name: "Sender".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender agent".to_string(),
    };
    let sender_id: i64 = AgentBmc::create(&tc.ctx, &tc.mm, sender_c)
        .await
        .unwrap()
        .into();

    let recipient_c = AgentForCreate {
        project_id: project.id,
        name: "Recipient".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Recipient agent".to_string(),
    };
    let recipient_id: i64 = AgentBmc::create(&tc.ctx, &tc.mm, recipient_c)
        .await
        .unwrap()
        .into();

    (project.id.get(), sender_id, recipient_id)
}

/// Test that unified inbox returns messages from ALL projects
#[tokio::test]
async fn test_list_unified_inbox_returns_all_projects() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Setup project 1
    let (project1_id, sender1_id, recipient1_id) =
        setup_project_with_agents(&tc, "/unified/project1").await;

    // Setup project 2
    let (project2_id, sender2_id, recipient2_id) =
        setup_project_with_agents(&tc, "/unified/project2").await;

    // Send message in project 1
    let msg1_c = MessageForCreate {
        project_id: project1_id,
        sender_id: sender1_id,
        recipient_ids: vec![recipient1_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Project 1 Message".to_string(),
        body_md: "Message from project 1".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg1_c).await.unwrap();

    // Send message in project 2
    let msg2_c = MessageForCreate {
        project_id: project2_id,
        sender_id: sender2_id,
        recipient_ids: vec![recipient2_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Project 2 Message".to_string(),
        body_md: "Message from project 2".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg2_c).await.unwrap();

    // Call unified inbox - should return messages from BOTH projects
    let messages = MessageBmc::list_unified_inbox(&tc.ctx, &tc.mm, ImportanceFilter::All, 50)
        .await
        .expect("list_unified_inbox should succeed");

    assert_eq!(
        messages.len(),
        2,
        "Should return messages from both projects"
    );

    let subjects: Vec<&str> = messages.iter().map(|m| m.subject.as_str()).collect();
    assert!(subjects.contains(&"Project 1 Message"));
    assert!(subjects.contains(&"Project 2 Message"));
}

/// Test filtering by high importance
#[tokio::test]
async fn test_list_unified_inbox_filters_by_high_importance() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, sender_id, recipient_id) =
        setup_project_with_agents(&tc, "/unified/importance").await;

    // Send high importance message
    let high_msg = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "URGENT Task".to_string(),
        body_md: "This is urgent".to_string(),
        thread_id: None,
        importance: Some("high".to_string()),
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, high_msg).await.unwrap();

    // Send normal importance message
    let normal_msg = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Regular Update".to_string(),
        body_md: "Normal update".to_string(),
        thread_id: None,
        importance: Some("normal".to_string()),
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, normal_msg)
        .await
        .unwrap();

    // Filter by high importance
    let high_messages = MessageBmc::list_unified_inbox(&tc.ctx, &tc.mm, ImportanceFilter::High, 50)
        .await
        .expect("list_unified_inbox with High filter should succeed");

    assert_eq!(
        high_messages.len(),
        1,
        "Should only return high importance messages"
    );
    assert_eq!(high_messages[0].subject, "URGENT Task");
}

/// Test that limit is respected
#[tokio::test]
async fn test_list_unified_inbox_respects_limit() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, sender_id, recipient_id) =
        setup_project_with_agents(&tc, "/unified/limit").await;

    // Send 5 messages
    for i in 1..=5 {
        let msg = MessageForCreate {
            project_id,
            sender_id,
            recipient_ids: vec![recipient_id],
            cc_ids: None,
            bcc_ids: None,
            subject: format!("Message {}", i),
            body_md: format!("Body {}", i),
            thread_id: None,
            importance: None,
            ack_required: false,
        };
        MessageBmc::create(&tc.ctx, &tc.mm, msg).await.unwrap();
    }

    // Request with limit of 3
    let messages = MessageBmc::list_unified_inbox(&tc.ctx, &tc.mm, ImportanceFilter::All, 3)
        .await
        .expect("list_unified_inbox with limit should succeed");

    assert_eq!(messages.len(), 3, "Should respect limit of 3");
}

/// Test empty inbox returns empty vector (not error)
#[tokio::test]
async fn test_list_unified_inbox_empty() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // No messages created - should return empty vec
    let messages = MessageBmc::list_unified_inbox(&tc.ctx, &tc.mm, ImportanceFilter::All, 50)
        .await
        .expect("list_unified_inbox on empty db should succeed");

    assert!(messages.is_empty(), "Empty inbox should return empty vec");
}
