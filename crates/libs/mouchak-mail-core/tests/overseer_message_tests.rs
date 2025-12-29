//! Overseer message model tests
//!
//! Tests for human guidance messages from overseers to agents.

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
use mouchak_mail_core::model::overseer_message::{OverseerMessageBmc, OverseerMessageForCreate};
use mouchak_mail_core::model::project::ProjectBmc;
use mouchak_mail_core::types::{AgentId, ProjectId};
use mouchak_mail_core::utils::slugify;

/// Helper to set up a project with an agent
async fn setup_project_and_agent(tc: &TestContext, suffix: &str) -> (ProjectId, AgentId) {
    let human_key = format!("/test/overseer-repo-{}", suffix);
    let slug = slugify(&human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, &human_key)
        .await
        .expect("Failed to create project");

    let agent = AgentForCreate {
        project_id,
        name: format!("overseer-{}", suffix),
        program: "human".to_string(),
        model: "human".to_string(),
        task_description: "Human overseer".to_string(),
    };

    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent)
        .await
        .expect("Failed to create agent");

    (project_id, agent_id)
}

/// Test creating an overseer message
#[tokio::test]
async fn test_create_overseer_message() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, sender_id) = setup_project_and_agent(&tc, "create").await;

    let msg_c = OverseerMessageForCreate {
        project_id: project_id.into(),
        sender_id: sender_id.into(),
        subject: "Priority Change".to_string(),
        body_md: "Please focus on the security fixes first.".to_string(),
        importance: "high".to_string(),
    };

    let msg_id = OverseerMessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Failed to create overseer message");

    assert!(msg_id > 0, "Message ID should be positive");
}

/// Test listing unread overseer messages
#[tokio::test]
async fn test_list_unread_messages() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, sender_id) = setup_project_and_agent(&tc, "list").await;

    // Create multiple messages
    for i in 1..=3 {
        let msg_c = OverseerMessageForCreate {
            project_id: project_id.into(),
            sender_id: sender_id.into(),
            subject: format!("Guidance {}", i),
            body_md: format!("Instruction {} details.", i),
            importance: "normal".to_string(),
        };
        OverseerMessageBmc::create(&tc.ctx, &tc.mm, msg_c)
            .await
            .expect("Failed to create message");
    }

    let unread = OverseerMessageBmc::list_unread(&tc.ctx, &tc.mm, project_id.into())
        .await
        .expect("Failed to list unread messages");

    assert_eq!(unread.len(), 3, "Should have 3 unread messages");
}

/// Test message importance levels
#[tokio::test]
async fn test_message_importance_levels() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, sender_id) = setup_project_and_agent(&tc, "importance").await;

    let importance_levels = vec!["low", "normal", "high", "critical"];

    for importance in &importance_levels {
        let msg_c = OverseerMessageForCreate {
            project_id: project_id.into(),
            sender_id: sender_id.into(),
            subject: format!("{} priority message", importance),
            body_md: "Test body".to_string(),
            importance: importance.to_string(),
        };
        OverseerMessageBmc::create(&tc.ctx, &tc.mm, msg_c)
            .await
            .expect("Failed to create message");
    }

    let unread = OverseerMessageBmc::list_unread(&tc.ctx, &tc.mm, project_id.into())
        .await
        .expect("Failed to list messages");

    assert_eq!(unread.len(), 4, "Should have 4 messages");

    // Verify importance levels are preserved
    let importances: Vec<&str> = unread.iter().map(|m| m.importance.as_str()).collect();
    for level in &importance_levels {
        assert!(
            importances.contains(level),
            "Should contain {} importance",
            level
        );
    }
}

/// Test empty unread list for new project
#[tokio::test]
async fn test_empty_unread_list() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, _) = setup_project_and_agent(&tc, "empty").await;

    let unread = OverseerMessageBmc::list_unread(&tc.ctx, &tc.mm, project_id.into())
        .await
        .expect("Failed to list unread messages");

    assert!(
        unread.is_empty(),
        "New project should have no unread messages"
    );
}

/// Test message ordering and count
#[tokio::test]
async fn test_message_ordering() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, sender_id) = setup_project_and_agent(&tc, "order").await;

    // Create messages in sequence
    for i in 1..=3 {
        let msg_c = OverseerMessageForCreate {
            project_id: project_id.into(),
            sender_id: sender_id.into(),
            subject: format!("Message {}", i),
            body_md: format!("Body {}", i),
            importance: "normal".to_string(),
        };
        OverseerMessageBmc::create(&tc.ctx, &tc.mm, msg_c)
            .await
            .expect("Failed to create message");
    }

    let unread = OverseerMessageBmc::list_unread(&tc.ctx, &tc.mm, project_id.into())
        .await
        .expect("Failed to list messages");

    // Verify all messages are returned
    assert_eq!(unread.len(), 3, "Should have 3 messages");

    // Verify all subjects are present (ordering may vary when timestamps are equal)
    let subjects: Vec<&str> = unread.iter().map(|m| m.subject.as_str()).collect();
    assert!(subjects.contains(&"Message 1"));
    assert!(subjects.contains(&"Message 2"));
    assert!(subjects.contains(&"Message 3"));
}

/// Test message structure
#[tokio::test]
async fn test_message_structure() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, sender_id) = setup_project_and_agent(&tc, "structure").await;

    let msg_c = OverseerMessageForCreate {
        project_id: project_id.into(),
        sender_id: sender_id.into(),
        subject: "Important Update".to_string(),
        body_md: "Please review the changes in PR #123.".to_string(),
        importance: "high".to_string(),
    };

    let msg_id = OverseerMessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Failed to create message");

    let unread = OverseerMessageBmc::list_unread(&tc.ctx, &tc.mm, project_id.into())
        .await
        .expect("Failed to list messages");

    let msg = &unread[0];
    assert_eq!(msg.id, msg_id);
    assert_eq!(msg.project_id, project_id.get());
    assert_eq!(msg.sender_id, sender_id.get());
    assert_eq!(msg.subject, "Important Update");
    assert_eq!(msg.body_md, "Please review the changes in PR #123.");
    assert_eq!(msg.importance, "high");
    assert!(msg.read_ts.is_none(), "New message should be unread");
}
