//! CC/BCC recipient tests
//!
//! Tests for CC and BCC recipient functionality in messages.

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::types::ProjectId;
use lib_core::utils::slugify;

/// Helper to set up project with sender and multiple recipients
async fn setup_cc_bcc_messaging(tc: &TestContext) -> (i64, i64, i64, i64, i64) {
    let human_key = "/cc-bcc/test";
    let slug = slugify(human_key);

    // Create project
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    // Create sender
    let sender_c = AgentForCreate {
        project_id: project.id,
        name: "Sender".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender agent".to_string(),
    };
    let sender_id = AgentBmc::create(&tc.ctx, &tc.mm, sender_c).await.unwrap();

    // Create TO recipient
    let to_recipient_c = AgentForCreate {
        project_id: project.id,
        name: "ToRecipient".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "TO recipient agent".to_string(),
    };
    let to_recipient_id = AgentBmc::create(&tc.ctx, &tc.mm, to_recipient_c)
        .await
        .unwrap();

    // Create CC recipient
    let cc_recipient_c = AgentForCreate {
        project_id: project.id,
        name: "CcRecipient".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "CC recipient agent".to_string(),
    };
    let cc_recipient_id = AgentBmc::create(&tc.ctx, &tc.mm, cc_recipient_c)
        .await
        .unwrap();

    // Create BCC recipient
    let bcc_recipient_c = AgentForCreate {
        project_id: project.id,
        name: "BccRecipient".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "BCC recipient agent".to_string(),
    };
    let bcc_recipient_id = AgentBmc::create(&tc.ctx, &tc.mm, bcc_recipient_c)
        .await
        .unwrap();

    (
        project.id.into(),
        sender_id.into(),
        to_recipient_id.into(),
        cc_recipient_id.into(),
        bcc_recipient_id.into(),
    )
}

/// Helper to query recipient_type from database
async fn get_recipient_types(tc: &TestContext, message_id: i64) -> Vec<(i64, String)> {
    let db = tc.mm.db_for_test();
    let stmt = db.prepare(
        "SELECT agent_id, recipient_type FROM message_recipients WHERE message_id = ? ORDER BY agent_id"
    ).await.unwrap();

    let mut rows = stmt.query([message_id]).await.unwrap();
    let mut results = Vec::new();

    while let Some(row) = rows.next().await.unwrap() {
        let agent_id: i64 = row.get(0).unwrap();
        let recipient_type: String = row.get(1).unwrap();
        results.push((agent_id, recipient_type));
    }

    results
}

/// Test sending a message with CC recipients
#[tokio::test]
async fn test_send_message_with_cc() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, to_recipient_id, cc_recipient_id, _bcc_recipient_id) =
        setup_cc_bcc_messaging(&tc).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![to_recipient_id],
        cc_ids: Some(vec![cc_recipient_id]),
        bcc_ids: None,
        subject: "Test with CC".to_string(),
        body_md: "This message has a CC recipient.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Failed to send message with CC");

    // Verify message was created
    assert!(msg_id > 0, "Message should have valid ID");

    // Verify recipient types in database
    let recipient_types = get_recipient_types(&tc, msg_id).await;
    assert_eq!(recipient_types.len(), 2, "Should have 2 recipients");

    // Find the TO recipient
    let to_entry = recipient_types
        .iter()
        .find(|(id, _)| *id == to_recipient_id);
    assert!(to_entry.is_some(), "TO recipient should be in database");
    assert_eq!(
        to_entry.unwrap().1,
        "to",
        "First recipient should have type 'to'"
    );

    // Find the CC recipient
    let cc_entry = recipient_types
        .iter()
        .find(|(id, _)| *id == cc_recipient_id);
    assert!(cc_entry.is_some(), "CC recipient should be in database");
    assert_eq!(
        cc_entry.unwrap().1,
        "cc",
        "Second recipient should have type 'cc'"
    );
}

/// Test sending a message with BCC recipients
#[tokio::test]
async fn test_send_message_with_bcc() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, to_recipient_id, _cc_recipient_id, bcc_recipient_id) =
        setup_cc_bcc_messaging(&tc).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![to_recipient_id],
        cc_ids: None,
        bcc_ids: Some(vec![bcc_recipient_id]),
        subject: "Test with BCC".to_string(),
        body_md: "This message has a BCC recipient.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Failed to send message with BCC");

    // Verify message was created
    assert!(msg_id > 0, "Message should have valid ID");

    // Verify recipient types in database
    let recipient_types = get_recipient_types(&tc, msg_id).await;
    assert_eq!(recipient_types.len(), 2, "Should have 2 recipients");

    // Find the TO recipient
    let to_entry = recipient_types
        .iter()
        .find(|(id, _)| *id == to_recipient_id);
    assert!(to_entry.is_some(), "TO recipient should be in database");
    assert_eq!(
        to_entry.unwrap().1,
        "to",
        "First recipient should have type 'to'"
    );

    // Find the BCC recipient
    let bcc_entry = recipient_types
        .iter()
        .find(|(id, _)| *id == bcc_recipient_id);
    assert!(bcc_entry.is_some(), "BCC recipient should be in database");
    assert_eq!(
        bcc_entry.unwrap().1,
        "bcc",
        "Second recipient should have type 'bcc'"
    );
}

/// Test sending a message with both CC and BCC recipients
#[tokio::test]
async fn test_send_message_with_cc_and_bcc() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, to_recipient_id, cc_recipient_id, bcc_recipient_id) =
        setup_cc_bcc_messaging(&tc).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![to_recipient_id],
        cc_ids: Some(vec![cc_recipient_id]),
        bcc_ids: Some(vec![bcc_recipient_id]),
        subject: "Test with CC and BCC".to_string(),
        body_md: "This message has both CC and BCC recipients.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Failed to send message with CC and BCC");

    // Verify message was created
    assert!(msg_id > 0, "Message should have valid ID");

    // Verify recipient types in database
    let recipient_types = get_recipient_types(&tc, msg_id).await;
    assert_eq!(recipient_types.len(), 3, "Should have 3 recipients");

    // Verify TO recipient
    let to_entry = recipient_types
        .iter()
        .find(|(id, _)| *id == to_recipient_id);
    assert!(to_entry.is_some(), "TO recipient should be in database");
    assert_eq!(
        to_entry.unwrap().1,
        "to",
        "TO recipient should have type 'to'"
    );

    // Verify CC recipient
    let cc_entry = recipient_types
        .iter()
        .find(|(id, _)| *id == cc_recipient_id);
    assert!(cc_entry.is_some(), "CC recipient should be in database");
    assert_eq!(
        cc_entry.unwrap().1,
        "cc",
        "CC recipient should have type 'cc'"
    );

    // Verify BCC recipient
    let bcc_entry = recipient_types
        .iter()
        .find(|(id, _)| *id == bcc_recipient_id);
    assert!(bcc_entry.is_some(), "BCC recipient should be in database");
    assert_eq!(
        bcc_entry.unwrap().1,
        "bcc",
        "BCC recipient should have type 'bcc'"
    );
}

/// Test that CC recipients can see the message in their inbox
#[tokio::test]
async fn test_cc_recipient_can_see_message() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, to_recipient_id, cc_recipient_id, _bcc_recipient_id) =
        setup_cc_bcc_messaging(&tc).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![to_recipient_id],
        cc_ids: Some(vec![cc_recipient_id]),
        bcc_ids: None,
        subject: "CC Visibility Test".to_string(),
        body_md: "CC recipient should see this.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();

    // Check TO recipient's inbox
    let to_inbox =
        MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project_id, to_recipient_id, 10)
            .await
            .expect("Failed to get TO recipient inbox");
    assert_eq!(to_inbox.len(), 1, "TO recipient should have 1 message");
    assert_eq!(
        to_inbox[0].id, msg_id,
        "TO recipient should see the message"
    );

    // Check CC recipient's inbox
    let cc_inbox =
        MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project_id, cc_recipient_id, 10)
            .await
            .expect("Failed to get CC recipient inbox");
    assert_eq!(cc_inbox.len(), 1, "CC recipient should have 1 message");
    assert_eq!(
        cc_inbox[0].id, msg_id,
        "CC recipient should see the message"
    );
}

/// Test that BCC recipients can see the message but aren't visible to others
#[tokio::test]
async fn test_bcc_recipient_can_see_message() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, to_recipient_id, _cc_recipient_id, bcc_recipient_id) =
        setup_cc_bcc_messaging(&tc).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![to_recipient_id],
        cc_ids: None,
        bcc_ids: Some(vec![bcc_recipient_id]),
        subject: "BCC Visibility Test".to_string(),
        body_md: "BCC recipient should see this but others shouldn't know.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();

    // Check TO recipient's inbox
    let to_inbox =
        MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project_id, to_recipient_id, 10)
            .await
            .expect("Failed to get TO recipient inbox");
    assert_eq!(to_inbox.len(), 1, "TO recipient should have 1 message");
    assert_eq!(
        to_inbox[0].id, msg_id,
        "TO recipient should see the message"
    );

    // Check BCC recipient's inbox
    let bcc_inbox =
        MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project_id, bcc_recipient_id, 10)
            .await
            .expect("Failed to get BCC recipient inbox");
    assert_eq!(bcc_inbox.len(), 1, "BCC recipient should have 1 message");
    assert_eq!(
        bcc_inbox[0].id, msg_id,
        "BCC recipient should see the message"
    );

    // Verify BCC recipient is stored with correct type (would not be visible in a real UI)
    let recipient_types = get_recipient_types(&tc, msg_id).await;
    let bcc_entry = recipient_types
        .iter()
        .find(|(id, _)| *id == bcc_recipient_id);
    assert!(
        bcc_entry.is_some(),
        "BCC recipient should exist in database"
    );
    assert_eq!(
        bcc_entry.unwrap().1,
        "bcc",
        "Recipient should be marked as BCC"
    );
}

/// Test multiple CC recipients
#[tokio::test]
async fn test_multiple_cc_recipients() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, to_recipient_id, cc_recipient_id, _bcc_recipient_id) =
        setup_cc_bcc_messaging(&tc).await;

    // Create a second CC recipient
    let cc_recipient2_c = AgentForCreate {
        project_id: ProjectId(project_id),
        name: "CcRecipient2".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Second CC recipient agent".to_string(),
    };
    let cc_recipient2_id = AgentBmc::create(&tc.ctx, &tc.mm, cc_recipient2_c)
        .await
        .unwrap();

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![to_recipient_id],
        cc_ids: Some(vec![cc_recipient_id, cc_recipient2_id.into()]),
        bcc_ids: None,
        subject: "Multiple CC Test".to_string(),
        body_md: "This has multiple CC recipients.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();

    // Verify recipient types in database
    let recipient_types = get_recipient_types(&tc, msg_id).await;
    assert_eq!(recipient_types.len(), 3, "Should have 3 recipients total");

    // Count CC recipients
    let cc_count = recipient_types
        .iter()
        .filter(|(_, typ)| typ == "cc")
        .count();
    assert_eq!(cc_count, 2, "Should have 2 CC recipients");

    // Both CC recipients should see the message
    let cc1_inbox =
        MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project_id, cc_recipient_id, 10)
            .await
            .unwrap();
    assert_eq!(
        cc1_inbox.len(),
        1,
        "First CC recipient should have 1 message"
    );

    let cc2_inbox =
        MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project_id, cc_recipient2_id.into(), 10)
            .await
            .unwrap();
    assert_eq!(
        cc2_inbox.len(),
        1,
        "Second CC recipient should have 1 message"
    );
}

/// Test multiple BCC recipients
#[tokio::test]
async fn test_multiple_bcc_recipients() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, to_recipient_id, _cc_recipient_id, bcc_recipient_id) =
        setup_cc_bcc_messaging(&tc).await;

    // Create a second BCC recipient
    let bcc_recipient2_c = AgentForCreate {
        project_id: ProjectId(project_id),
        name: "BccRecipient2".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Second BCC recipient agent".to_string(),
    };
    let bcc_recipient2_id = AgentBmc::create(&tc.ctx, &tc.mm, bcc_recipient2_c)
        .await
        .unwrap();

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![to_recipient_id],
        cc_ids: None,
        bcc_ids: Some(vec![bcc_recipient_id, bcc_recipient2_id.into()]),
        subject: "Multiple BCC Test".to_string(),
        body_md: "This has multiple BCC recipients.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();

    // Verify recipient types in database
    let recipient_types = get_recipient_types(&tc, msg_id).await;
    assert_eq!(recipient_types.len(), 3, "Should have 3 recipients total");

    // Count BCC recipients
    let bcc_count = recipient_types
        .iter()
        .filter(|(_, typ)| typ == "bcc")
        .count();
    assert_eq!(bcc_count, 2, "Should have 2 BCC recipients");

    // Both BCC recipients should see the message
    let bcc1_inbox =
        MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project_id, bcc_recipient_id, 10)
            .await
            .unwrap();
    assert_eq!(
        bcc1_inbox.len(),
        1,
        "First BCC recipient should have 1 message"
    );

    let bcc2_inbox =
        MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project_id, bcc_recipient2_id.into(), 10)
            .await
            .unwrap();
    assert_eq!(
        bcc2_inbox.len(),
        1,
        "Second BCC recipient should have 1 message"
    );
}

/// Test that CC recipients can mark message as read
#[tokio::test]
async fn test_cc_recipient_mark_as_read() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, to_recipient_id, cc_recipient_id, _bcc_recipient_id) =
        setup_cc_bcc_messaging(&tc).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![to_recipient_id],
        cc_ids: Some(vec![cc_recipient_id]),
        bcc_ids: None,
        subject: "CC Read Test".to_string(),
        body_md: "CC recipient can mark as read.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();

    // CC recipient marks as read
    let result = MessageBmc::mark_read(&tc.ctx, &tc.mm, msg_id, cc_recipient_id).await;
    assert!(
        result.is_ok(),
        "CC recipient should be able to mark message as read"
    );
}

/// Test that BCC recipients can acknowledge messages
#[tokio::test]
async fn test_bcc_recipient_acknowledge() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, to_recipient_id, _cc_recipient_id, bcc_recipient_id) =
        setup_cc_bcc_messaging(&tc).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![to_recipient_id],
        cc_ids: None,
        bcc_ids: Some(vec![bcc_recipient_id]),
        subject: "BCC Acknowledge Test".to_string(),
        body_md: "BCC recipient can acknowledge.".to_string(),
        thread_id: None,
        importance: Some("high".to_string()),
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();

    // BCC recipient acknowledges
    let result = MessageBmc::acknowledge(&tc.ctx, &tc.mm, msg_id, bcc_recipient_id).await;
    assert!(
        result.is_ok(),
        "BCC recipient should be able to acknowledge message"
    );
}
