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
    let recipient_id = AgentBmc::create(&tc.ctx, &tc.mm, recipient_c).await.unwrap();
    
    (project.id, sender_id, recipient_id)
}

/// Test sending a simple message
#[tokio::test]
async fn test_send_message() {
    let tc = TestContext::new().await.expect("Failed to create test context");
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
    };
    
    let msg_id = MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .expect("Failed to send message");
    
    assert!(msg_id > 0, "Message should have valid ID");
}

/// Test getting a specific message
#[tokio::test]
async fn test_get_message() {
    let tc = TestContext::new().await.expect("Failed to create test context");
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
    let tc = TestContext::new().await.expect("Failed to create test context");
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
    };
    let initial_id = MessageBmc::create(&tc.ctx, &tc.mm, initial_msg_c).await.unwrap();
    
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
    };
    let reply_id = MessageBmc::create(&tc.ctx, &tc.mm, reply_msg_c).await.unwrap();
    
    let reply = MessageBmc::get(&tc.ctx, &tc.mm, reply_id).await.unwrap();
    
    assert_eq!(initial.thread_id, reply.thread_id, "Thread IDs should match");
}

/// Test full-text search using FTS5
#[tokio::test]
async fn test_search_messages() {
    let tc = TestContext::new().await.expect("Failed to create test context");
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
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg3_c).await.unwrap();
    
    // Search for "full-text search" - should match msg1 and msg3
    let results = MessageBmc::search(&tc.ctx, &tc.mm, project_id, "full-text search", 10)
        .await
        .expect("Search should succeed");
    
    // Verify search finds relevant messages
    assert!(results.len() >= 1, "Should find at least one message containing 'full-text search'");
    assert!(results.iter().any(|m| m.subject == "Database Migration" || m.subject == "Performance"),
        "Should find messages about FTS");
}

/// Test marking a message as read
#[tokio::test]
async fn test_mark_message_read() {
    let tc = TestContext::new().await.expect("Failed to create test context");
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
    let tc = TestContext::new().await.expect("Failed to create test context");
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
    let tc = TestContext::new().await.expect("Failed to create test context");
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
