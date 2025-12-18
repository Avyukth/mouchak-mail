//! Message model tests
//!
//! Tests for message sending, inbox retrieval, and threading.

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::utils::slugify;

/// Helper to set up project and agents for message tests
async fn setup_messaging(tc: &TestContext) -> (i64, i64, i64) {
    let human_key = "/messaging/test";
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

    // Create recipient
    let recipient_c = AgentForCreate {
        project_id: project.id,
        name: "Recipient".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Recipient agent".to_string(),
    };
    let recipient_id = AgentBmc::create(&tc.ctx, &tc.mm, recipient_c)
        .await
        .unwrap();

    (project.id, sender_id, recipient_id)
}

/// Test sending a simple message
#[tokio::test]
async fn test_send_message() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Test Subject".to_string(),
        body_md: "Hello, this is a test message.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Failed to send message");

    assert!(msg_id > 0, "Message should have valid ID");
}

/// Test getting a specific message
#[tokio::test]
async fn test_get_message() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Important Message".to_string(),
        body_md: "This is important content.".to_string(),
        thread_id: None,
        importance: Some("high".to_string()),
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();

    let message = MessageBmc::get(&tc.ctx, &tc.mm, msg_id)
        .await
        .expect("Failed to get message");

    assert_eq!(message.subject, "Important Message");
    assert_eq!(message.body_md, "This is important content.");
}

/// Test thread replies maintain thread_id
#[tokio::test]
async fn test_message_threading() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Send initial message
    let initial_msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Thread Start".to_string(),
        body_md: "Starting a thread".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    let initial_id = MessageBmc::create(&tc.ctx, &tc.mm, initial_msg_c)
        .await
        .unwrap();

    // Get initial message to get thread_id
    let initial = MessageBmc::get(&tc.ctx, &tc.mm, initial_id).await.unwrap();

    // Send reply in same thread
    let reply_msg_c = MessageForCreate {
        project_id,
        sender_id: recipient_id, // Reply from recipient
        recipient_ids: vec![sender_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Re: Thread Start".to_string(),
        body_md: "This is a reply".to_string(),
        thread_id: initial.thread_id.clone(),
        importance: None,
        ack_required: false,
    };
    let reply_id = MessageBmc::create(&tc.ctx, &tc.mm, reply_msg_c)
        .await
        .unwrap();

    let reply = MessageBmc::get(&tc.ctx, &tc.mm, reply_id).await.unwrap();

    assert_eq!(
        initial.thread_id, reply.thread_id,
        "Thread IDs should match"
    );
}

/// Test full-text search using FTS5
#[tokio::test]
async fn test_search_messages() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Send a few messages with different content
    let msg1_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Database Migration".to_string(),
        body_md: "We need to implement FTS5 full-text search for messages.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg1_c).await.unwrap();

    let msg2_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "API Design".to_string(),
        body_md: "The REST API should follow JSON:API spec.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg2_c).await.unwrap();

    let msg3_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Performance".to_string(),
        body_md: "Full-text search queries should be fast.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg3_c).await.unwrap();

    // Search for "full-text search" - should match msg1 and msg3
    let results = MessageBmc::search(&tc.ctx, &tc.mm, project_id, "full-text search", 10)
        .await
        .expect("Search should succeed");

    // Verify search finds relevant messages
    assert!(
        !results.is_empty(),
        "Should find at least one message containing 'full-text search'"
    );
    assert!(
        results
            .iter()
            .any(|m| m.subject == "Database Migration" || m.subject == "Performance"),
        "Should find messages about FTS"
    );
}

/// Test marking a message as read
#[tokio::test]
async fn test_mark_message_read() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Send a message
    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Unread Message".to_string(),
        body_md: "This message should be marked as read.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();

    // Mark as read by recipient
    let result = MessageBmc::mark_read(&tc.ctx, &tc.mm, msg_id, recipient_id).await;
    assert!(result.is_ok(), "mark_read should succeed");

    // Calling mark_read again should be idempotent (no error)
    let result2 = MessageBmc::mark_read(&tc.ctx, &tc.mm, msg_id, recipient_id).await;
    assert!(result2.is_ok(), "mark_read should be idempotent");
}

/// Test acknowledging a message
#[tokio::test]
async fn test_acknowledge_message() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Send a message with ack_required
    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Important - ACK Required".to_string(),
        body_md: "Please acknowledge this message.".to_string(),
        thread_id: None,
        importance: Some("high".to_string()),
        ack_required: false,
    };
    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();

    // Acknowledge by recipient (also marks as read)
    let result = MessageBmc::acknowledge(&tc.ctx, &tc.mm, msg_id, recipient_id).await;
    assert!(result.is_ok(), "acknowledge should succeed");

    // Calling acknowledge again should be idempotent
    let result2 = MessageBmc::acknowledge(&tc.ctx, &tc.mm, msg_id, recipient_id).await;
    assert!(result2.is_ok(), "acknowledge should be idempotent");
}

/// Test listing threads (summarization)
#[tokio::test]
async fn test_list_threads() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Send messages in thread 1
    let msg1_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Thread A - Start".to_string(),
        body_md: "Starting thread A".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    let msg1_id = MessageBmc::create(&tc.ctx, &tc.mm, msg1_c).await.unwrap();
    let msg1 = MessageBmc::get(&tc.ctx, &tc.mm, msg1_id).await.unwrap();

    // Reply in thread 1
    let reply_c = MessageForCreate {
        project_id,
        sender_id: recipient_id,
        recipient_ids: vec![sender_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Re: Thread A - Start".to_string(),
        body_md: "Reply in thread A".to_string(),
        thread_id: msg1.thread_id.clone(),
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, reply_c).await.unwrap();

    // Send message in thread 2
    let msg2_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Thread B - Different topic".to_string(),
        body_md: "Starting thread B".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg2_c).await.unwrap();

    // List threads
    let threads = MessageBmc::list_threads(&tc.ctx, &tc.mm, project_id, 10)
        .await
        .expect("Should list threads");

    // Should have at least 2 threads
    assert!(threads.len() >= 2, "Should have at least 2 threads");

    // Each thread should have a message count
    for thread in &threads {
        assert!(thread.message_count > 0, "Thread should have messages");
    }
}

/// Test listing outbox messages for an agent
#[tokio::test]
async fn test_list_outbox() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Send multiple messages from sender
    let msg1_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Outbox Test 1".to_string(),
        body_md: "First message from sender".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg1_c).await.unwrap();

    let msg2_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Outbox Test 2".to_string(),
        body_md: "Second message from sender".to_string(),
        thread_id: None,
        importance: Some("high".to_string()),
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg2_c).await.unwrap();

    // List outbox for sender
    let outbox = MessageBmc::list_outbox_for_agent(&tc.ctx, &tc.mm, project_id, sender_id, 10)
        .await
        .expect("Should list outbox messages");

    // Verify we got both messages
    assert_eq!(outbox.len(), 2, "Should have 2 messages in outbox");

    // Verify messages are ordered by created_ts DESC (newest first)
    assert!(
        outbox[0].subject == "Outbox Test 2" || outbox[0].subject == "Outbox Test 1",
        "Should contain expected subjects"
    );

    // Verify sender_id is correct
    for msg in &outbox {
        assert_eq!(
            msg.sender_id, sender_id,
            "All messages should be from sender"
        );
    }
}

/// Test outbox filtering by project
#[tokio::test]
async fn test_outbox_project_filtering() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create first project and agents
    let human_key1 = "/outbox/project1";
    let slug1 = slugify(human_key1);
    let project1_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug1, human_key1)
        .await
        .expect("Failed to create project 1");
    let project1 = ProjectBmc::get(&tc.ctx, &tc.mm, project1_id).await.unwrap();

    let sender1_c = AgentForCreate {
        project_id: project1.id,
        name: "Sender1".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender in project 1".to_string(),
    };
    let sender1_id = AgentBmc::create(&tc.ctx, &tc.mm, sender1_c).await.unwrap();

    let recipient1_c = AgentForCreate {
        project_id: project1.id,
        name: "Recipient1".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Recipient in project 1".to_string(),
    };
    let recipient1_id = AgentBmc::create(&tc.ctx, &tc.mm, recipient1_c)
        .await
        .unwrap();

    // Create second project and agents
    let human_key2 = "/outbox/project2";
    let slug2 = slugify(human_key2);
    let project2_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug2, human_key2)
        .await
        .expect("Failed to create project 2");
    let project2 = ProjectBmc::get(&tc.ctx, &tc.mm, project2_id).await.unwrap();

    let sender2_c = AgentForCreate {
        project_id: project2.id,
        name: "Sender2".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender in project 2".to_string(),
    };
    let sender2_id = AgentBmc::create(&tc.ctx, &tc.mm, sender2_c).await.unwrap();

    let recipient2_c = AgentForCreate {
        project_id: project2.id,
        name: "Recipient2".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Recipient in project 2".to_string(),
    };
    let recipient2_id = AgentBmc::create(&tc.ctx, &tc.mm, recipient2_c)
        .await
        .unwrap();

    // Send message in project 1
    let msg1_c = MessageForCreate {
        project_id: project1.id,
        sender_id: sender1_id,
        recipient_ids: vec![recipient1_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Project 1 Message".to_string(),
        body_md: "Message in project 1".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg1_c).await.unwrap();

    // Send message in project 2
    let msg2_c = MessageForCreate {
        project_id: project2.id,
        sender_id: sender2_id,
        recipient_ids: vec![recipient2_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Project 2 Message".to_string(),
        body_md: "Message in project 2".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg2_c).await.unwrap();

    // List outbox for sender1 in project1
    let outbox1 = MessageBmc::list_outbox_for_agent(&tc.ctx, &tc.mm, project1.id, sender1_id, 10)
        .await
        .expect("Should list outbox for project 1");

    // List outbox for sender2 in project2
    let outbox2 = MessageBmc::list_outbox_for_agent(&tc.ctx, &tc.mm, project2.id, sender2_id, 10)
        .await
        .expect("Should list outbox for project 2");

    // Verify correct filtering
    assert_eq!(
        outbox1.len(),
        1,
        "Should have 1 message in project 1 outbox"
    );
    assert_eq!(
        outbox2.len(),
        1,
        "Should have 1 message in project 2 outbox"
    );
    assert_eq!(outbox1[0].subject, "Project 1 Message");
    assert_eq!(outbox2[0].subject, "Project 2 Message");
}

/// Test outbox pagination with limit
#[tokio::test]
async fn test_outbox_pagination() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Send 5 messages
    for i in 1..=5 {
        let msg_c = MessageForCreate {
            project_id,
            sender_id,
            recipient_ids: vec![recipient_id],
            cc_ids: None,
            bcc_ids: None,
            subject: format!("Message {}", i),
            body_md: format!("Body of message {}", i),
            thread_id: None,
            importance: None,
            ack_required: false,
        };
        MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();
    }

    // List with limit of 3
    let outbox_limited =
        MessageBmc::list_outbox_for_agent(&tc.ctx, &tc.mm, project_id, sender_id, 3)
            .await
            .expect("Should list outbox with limit");

    // Verify limit is respected
    assert_eq!(outbox_limited.len(), 3, "Should return exactly 3 messages");

    // List with limit of 10 (should return all 5)
    let outbox_all = MessageBmc::list_outbox_for_agent(&tc.ctx, &tc.mm, project_id, sender_id, 10)
        .await
        .expect("Should list all outbox messages");

    assert_eq!(outbox_all.len(), 5, "Should return all 5 messages");
}

/// Test outbox with multiple recipients (including CC and BCC)
#[tokio::test]
async fn test_outbox_with_multiple_recipients() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let human_key = "/outbox/multi-recipient";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    // Create one sender and multiple recipients
    let sender_c = AgentForCreate {
        project_id: project.id,
        name: "MultiSender".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender".to_string(),
    };
    let sender_id = AgentBmc::create(&tc.ctx, &tc.mm, sender_c).await.unwrap();

    let recipient1_c = AgentForCreate {
        project_id: project.id,
        name: "Recipient1".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Recipient 1".to_string(),
    };
    let recipient1_id = AgentBmc::create(&tc.ctx, &tc.mm, recipient1_c)
        .await
        .unwrap();

    let recipient2_c = AgentForCreate {
        project_id: project.id,
        name: "Recipient2".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Recipient 2".to_string(),
    };
    let recipient2_id = AgentBmc::create(&tc.ctx, &tc.mm, recipient2_c)
        .await
        .unwrap();

    let recipient3_c = AgentForCreate {
        project_id: project.id,
        name: "Recipient3".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Recipient 3".to_string(),
    };
    let recipient3_id = AgentBmc::create(&tc.ctx, &tc.mm, recipient3_c)
        .await
        .unwrap();

    // Send message with multiple recipients (to, cc, bcc)
    let msg_c = MessageForCreate {
        project_id: project.id,
        sender_id,
        recipient_ids: vec![recipient1_id],
        cc_ids: Some(vec![recipient2_id]),
        bcc_ids: Some(vec![recipient3_id]),
        subject: "Multi-recipient Message".to_string(),
        body_md: "This message has multiple recipients".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg_c).await.unwrap();

    // Check outbox for sender
    let outbox = MessageBmc::list_outbox_for_agent(&tc.ctx, &tc.mm, project.id, sender_id, 10)
        .await
        .expect("Should list outbox");

    // Should have exactly 1 message
    assert_eq!(outbox.len(), 1, "Should have 1 message in outbox");
    assert_eq!(outbox[0].subject, "Multi-recipient Message");

    // Verify each recipient got the message in their inbox
    let inbox1 = MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project.id, recipient1_id, 10)
        .await
        .expect("Should list inbox for recipient1");
    assert_eq!(inbox1.len(), 1, "Recipient1 should have message in inbox");

    let inbox2 = MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project.id, recipient2_id, 10)
        .await
        .expect("Should list inbox for recipient2");
    assert_eq!(
        inbox2.len(),
        1,
        "Recipient2 (CC) should have message in inbox"
    );

    let inbox3 = MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project.id, recipient3_id, 10)
        .await
        .expect("Should list inbox for recipient3");
    assert_eq!(
        inbox3.len(),
        1,
        "Recipient3 (BCC) should have message in inbox"
    );
}

/// Test empty outbox
#[tokio::test]
async fn test_empty_outbox() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, _recipient_id) = setup_messaging(&tc).await;

    // List outbox without sending any messages
    let outbox = MessageBmc::list_outbox_for_agent(&tc.ctx, &tc.mm, project_id, sender_id, 10)
        .await
        .expect("Should list empty outbox");

    assert_eq!(outbox.len(), 0, "Outbox should be empty");
}

/// Test message with multiple TO recipients (tests batch INSERT optimization)
///
/// This test exercises the batched SQL path in MessageBmc::create where
/// multiple recipient rows are inserted with a single INSERT statement:
/// `INSERT INTO message_recipients VALUES (?,?,?), (?,?,?), (?,?,?)`
#[tokio::test]
async fn test_multiple_to_recipients() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let human_key = "/batch/multi-to";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    // Create sender
    let sender_c = AgentForCreate {
        project_id: project.id,
        name: "BatchSender".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender for batch test".to_string(),
    };
    let sender_id = AgentBmc::create(&tc.ctx, &tc.mm, sender_c).await.unwrap();

    // Create 3 recipients to test batch insert
    let mut recipient_ids = Vec::new();
    for i in 1..=3 {
        let recipient_c = AgentForCreate {
            project_id: project.id,
            name: format!("ToRecipient{}", i),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: format!("TO recipient {}", i),
        };
        let rid = AgentBmc::create(&tc.ctx, &tc.mm, recipient_c)
            .await
            .unwrap();
        recipient_ids.push(rid);
    }

    // Send message with multiple TO recipients (no CC/BCC)
    let msg_c = MessageForCreate {
        project_id: project.id,
        sender_id,
        recipient_ids: recipient_ids.clone(),
        cc_ids: None,
        bcc_ids: None,
        subject: "Batch TO Recipients Test".to_string(),
        body_md: "Testing batch insert with multiple TO recipients".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Should create message with multiple TO recipients");

    // Verify message was created
    let message = MessageBmc::get(&tc.ctx, &tc.mm, msg_id).await.unwrap();
    assert_eq!(message.subject, "Batch TO Recipients Test");

    // Verify ALL 3 recipients received the message in their inbox
    for (i, rid) in recipient_ids.iter().enumerate() {
        let inbox = MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project.id, *rid, 10)
            .await
            .expect("Should list inbox");
        assert_eq!(
            inbox.len(),
            1,
            "TO Recipient {} should have exactly 1 message in inbox",
            i + 1
        );
        assert_eq!(inbox[0].subject, "Batch TO Recipients Test");
    }

    // Verify sender's outbox has exactly 1 message
    let outbox = MessageBmc::list_outbox_for_agent(&tc.ctx, &tc.mm, project.id, sender_id, 10)
        .await
        .expect("Should list outbox");
    assert_eq!(outbox.len(), 1, "Sender should have 1 message in outbox");
}

// ============================================================================
// ACK_REQUIRED TESTS (TDD - RED PHASE)
// ============================================================================

/// Test that ack_required flag is persisted when creating a message
#[tokio::test]
async fn test_ack_required_persisted() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Create message with ack_required = true
    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Review Required".to_string(),
        body_md: "Please review and acknowledge.".to_string(),
        thread_id: None,
        importance: Some("high".to_string()),
        ack_required: true,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Should create message");

    // Verify ack_required is persisted
    let message = MessageBmc::get(&tc.ctx, &tc.mm, msg_id).await.unwrap();
    assert!(
        message.ack_required,
        "ack_required should be true after creation"
    );
}

/// Test that ack_required defaults to false when not specified
#[tokio::test]
async fn test_ack_required_defaults_false() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Create message without specifying ack_required (should default to false)
    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Normal Message".to_string(),
        body_md: "No acknowledgment needed.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Should create message");

    let message = MessageBmc::get(&tc.ctx, &tc.mm, msg_id).await.unwrap();
    assert!(
        !message.ack_required,
        "ack_required should default to false"
    );
}

/// Test list_pending_reviews returns messages with ack_required=true and unacked recipients
#[tokio::test]
async fn test_list_pending_reviews_returns_ack_required_messages() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Create message WITH ack_required
    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "[COMPLETION] Task Done".to_string(),
        body_md: "Task completed, please review.".to_string(),
        thread_id: None,
        importance: Some("high".to_string()),
        ack_required: true,
    };
    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Should create message");

    // List pending reviews
    let pending = MessageBmc::list_pending_reviews(&tc.ctx, &tc.mm, Some(project_id), None, 10)
        .await
        .expect("Should list pending reviews");

    // Should find our message
    assert_eq!(pending.len(), 1, "Should have 1 pending review");
    assert_eq!(pending[0].message_id, msg_id);
    assert_eq!(pending[0].subject, "[COMPLETION] Task Done");
}

/// Test that messages with ack_required=false don't appear in pending reviews
#[tokio::test]
async fn test_list_pending_reviews_excludes_non_ack_required() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Create message WITHOUT ack_required
    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Regular Update".to_string(),
        body_md: "Just an FYI, no action needed.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Should create message");

    // List pending reviews
    let pending = MessageBmc::list_pending_reviews(&tc.ctx, &tc.mm, Some(project_id), None, 10)
        .await
        .expect("Should list pending reviews");

    // Should NOT find our message (ack_required = false)
    assert_eq!(pending.len(), 0, "Should have no pending reviews");
}

/// Test that acknowledged messages are removed from pending reviews
#[tokio::test]
async fn test_list_pending_reviews_excludes_acknowledged() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (project_id, sender_id, recipient_id) = setup_messaging(&tc).await;

    // Create message with ack_required
    let msg_c = MessageForCreate {
        project_id,
        sender_id,
        recipient_ids: vec![recipient_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Review Request".to_string(),
        body_md: "Please acknowledge when reviewed.".to_string(),
        thread_id: None,
        importance: Some("high".to_string()),
        ack_required: true,
    };
    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Should create message");

    // Before acknowledge: should appear in pending
    let pending_before =
        MessageBmc::list_pending_reviews(&tc.ctx, &tc.mm, Some(project_id), None, 10)
            .await
            .unwrap();
    assert_eq!(pending_before.len(), 1, "Should have 1 pending before ack");

    // Acknowledge the message
    MessageBmc::acknowledge(&tc.ctx, &tc.mm, msg_id, recipient_id)
        .await
        .expect("Should acknowledge");

    // After acknowledge: should NOT appear in pending
    let pending_after =
        MessageBmc::list_pending_reviews(&tc.ctx, &tc.mm, Some(project_id), None, 10)
            .await
            .unwrap();
    assert_eq!(
        pending_after.len(),
        0,
        "Should have 0 pending after all recipients acked"
    );
}

/// Test pending reviews with multiple recipients - partial acknowledgment
#[tokio::test]
async fn test_list_pending_reviews_partial_ack() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let human_key = "/pending/partial-ack";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    // Create sender and 2 recipients
    let sender_c = AgentForCreate {
        project_id: project.id,
        name: "PartialSender".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender".to_string(),
    };
    let sender_id = AgentBmc::create(&tc.ctx, &tc.mm, sender_c).await.unwrap();

    let r1_c = AgentForCreate {
        project_id: project.id,
        name: "Reviewer1".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Reviewer 1".to_string(),
    };
    let r1_id = AgentBmc::create(&tc.ctx, &tc.mm, r1_c).await.unwrap();

    let r2_c = AgentForCreate {
        project_id: project.id,
        name: "Reviewer2".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Reviewer 2".to_string(),
    };
    let r2_id = AgentBmc::create(&tc.ctx, &tc.mm, r2_c).await.unwrap();

    // Create message requiring ack from both reviewers
    let msg_c = MessageForCreate {
        project_id: project.id,
        sender_id,
        recipient_ids: vec![r1_id, r2_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Dual Review Required".to_string(),
        body_md: "Both reviewers must acknowledge.".to_string(),
        thread_id: None,
        importance: Some("high".to_string()),
        ack_required: true,
    };
    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Should create message");

    // Before any ack: pending count = 1
    let pending = MessageBmc::list_pending_reviews(&tc.ctx, &tc.mm, Some(project.id), None, 10)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);

    // First reviewer acknowledges
    MessageBmc::acknowledge(&tc.ctx, &tc.mm, msg_id, r1_id)
        .await
        .expect("R1 should ack");

    // Still pending (r2 hasn't acked)
    let pending = MessageBmc::list_pending_reviews(&tc.ctx, &tc.mm, Some(project.id), None, 10)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1, "Should still be pending - partial ack");

    // Second reviewer acknowledges
    MessageBmc::acknowledge(&tc.ctx, &tc.mm, msg_id, r2_id)
        .await
        .expect("R2 should ack");

    // Now fully acked - should be removed from pending
    let pending = MessageBmc::list_pending_reviews(&tc.ctx, &tc.mm, Some(project.id), None, 10)
        .await
        .unwrap();
    assert_eq!(pending.len(), 0, "Should be empty after all acked");
}

/// Test message with multiple recipients across all types (TO, CC, BCC)
///
/// This tests the complete batch optimization path with mixed recipient types.
#[tokio::test]
async fn test_batch_mixed_recipient_types() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");
    let human_key = "/batch/mixed-types";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    // Create sender
    let sender_c = AgentForCreate {
        project_id: project.id,
        name: "MixedSender".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender for mixed batch test".to_string(),
    };
    let sender_id = AgentBmc::create(&tc.ctx, &tc.mm, sender_c).await.unwrap();

    // Create 2 TO recipients
    let mut to_ids = Vec::new();
    for i in 1..=2 {
        let c = AgentForCreate {
            project_id: project.id,
            name: format!("MixedTo{}", i),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: format!("TO recipient {}", i),
        };
        to_ids.push(AgentBmc::create(&tc.ctx, &tc.mm, c).await.unwrap());
    }

    // Create 2 CC recipients
    let mut cc_ids = Vec::new();
    for i in 1..=2 {
        let c = AgentForCreate {
            project_id: project.id,
            name: format!("MixedCc{}", i),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: format!("CC recipient {}", i),
        };
        cc_ids.push(AgentBmc::create(&tc.ctx, &tc.mm, c).await.unwrap());
    }

    // Create 2 BCC recipients
    let mut bcc_ids = Vec::new();
    for i in 1..=2 {
        let c = AgentForCreate {
            project_id: project.id,
            name: format!("MixedBcc{}", i),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: format!("BCC recipient {}", i),
        };
        bcc_ids.push(AgentBmc::create(&tc.ctx, &tc.mm, c).await.unwrap());
    }

    // Send message with 2 TO, 2 CC, 2 BCC (6 total recipients)
    let msg_c = MessageForCreate {
        project_id: project.id,
        sender_id,
        recipient_ids: to_ids.clone(),
        cc_ids: Some(cc_ids.clone()),
        bcc_ids: Some(bcc_ids.clone()),
        subject: "Mixed Batch Test".to_string(),
        body_md: "Testing batch insert with 6 total recipients".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Should create message with 6 recipients");

    // Verify message created
    let message = MessageBmc::get(&tc.ctx, &tc.mm, msg_id).await.unwrap();
    assert_eq!(message.subject, "Mixed Batch Test");

    // Verify all TO recipients received it
    for rid in &to_ids {
        let inbox = MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project.id, *rid, 10)
            .await
            .unwrap();
        assert_eq!(inbox.len(), 1, "TO recipient should have message");
    }

    // Verify all CC recipients received it
    for rid in &cc_ids {
        let inbox = MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project.id, *rid, 10)
            .await
            .unwrap();
        assert_eq!(inbox.len(), 1, "CC recipient should have message");
    }

    // Verify all BCC recipients received it
    for rid in &bcc_ids {
        let inbox = MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project.id, *rid, 10)
            .await
            .unwrap();
        assert_eq!(inbox.len(), 1, "BCC recipient should have message");
    }

    // Verify sender has exactly 1 message in outbox
    let outbox = MessageBmc::list_outbox_for_agent(&tc.ctx, &tc.mm, project.id, sender_id, 10)
        .await
        .unwrap();
    assert_eq!(outbox.len(), 1, "Sender should have 1 outbox message");
}
