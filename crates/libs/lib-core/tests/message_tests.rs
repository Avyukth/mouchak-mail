//! Message model tests
//!
//! Tests for message sending, inbox retrieval, and threading.

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
        subject: "Re: Thread Start".to_string(),
        body_md: "This is a reply".to_string(),
        thread_id: initial.thread_id.clone(),
        importance: None,
    };
    let reply_id = MessageBmc::create(&tc.ctx, &tc.mm, reply_msg_c).await.unwrap();
    
    let reply = MessageBmc::get(&tc.ctx, &tc.mm, reply_id).await.unwrap();
    
    assert_eq!(initial.thread_id, reply.thread_id, "Thread IDs should match");
}
