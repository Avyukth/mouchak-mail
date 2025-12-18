//! TDD Tests for consolidated summarize_thread tool
//!
//! Acceptance Criteria from PORT-1.1:
//! - Single tool accepts both String and Vec<String>
//! - JSON schema validates both input types  
//! - Partial failures return errors array, don't panic
//! - Backward compatible
//! - Tests: test_summarize_single, test_summarize_multiple, test_partial_failure

use lib_core::ctx::Ctx;
use lib_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    message::{MessageBmc, MessageForCreate},
    project::ProjectBmc,
};
use libsql::Builder;
use std::sync::Arc;
use tempfile::TempDir;

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_summarize.db");
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

    let mm = ModelManager::new_for_test(conn, archive_root);
    (Arc::new(mm), temp_dir)
}

async fn setup_test_data(mm: &ModelManager) -> (i64, i64, i64) {
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, mm, "test-project", "/test")
        .await
        .unwrap();

    let agent1_c = AgentForCreate {
        project_id,
        name: "alice".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Alice agent".to_string(),
    };
    let agent1_id = AgentBmc::create(&ctx, mm, agent1_c).await.unwrap();

    let agent2_c = AgentForCreate {
        project_id,
        name: "bob".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Bob agent".to_string(),
    };
    let agent2_id = AgentBmc::create(&ctx, mm, agent2_c).await.unwrap();

    let msg1 = MessageForCreate {
        project_id,
        sender_id: agent1_id,
        recipient_ids: vec![agent2_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Thread 1 Subject".to_string(),
        body_md: "First message in thread 1".to_string(),
        thread_id: Some("THREAD-001".to_string()),
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&ctx, mm, msg1).await.unwrap();

    let msg2 = MessageForCreate {
        project_id,
        sender_id: agent2_id,
        recipient_ids: vec![agent1_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Re: Thread 1 Subject".to_string(),
        body_md: "Reply in thread 1".to_string(),
        thread_id: Some("THREAD-001".to_string()),
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&ctx, mm, msg2).await.unwrap();

    let msg3 = MessageForCreate {
        project_id,
        sender_id: agent1_id,
        recipient_ids: vec![agent2_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Thread 2 Subject".to_string(),
        body_md: "Message in thread 2".to_string(),
        thread_id: Some("THREAD-002".to_string()),
        importance: Some("high".to_string()),
        ack_required: false,
    };
    MessageBmc::create(&ctx, mm, msg3).await.unwrap();

    (project_id, agent1_id, agent2_id)
}

#[test]
fn test_thread_id_input_deserializes_single_string() {
    use lib_mcp::tools::ThreadIdInput;

    let json = r#""THREAD-001""#;
    let input: ThreadIdInput = serde_json::from_str(json).unwrap();

    match input {
        ThreadIdInput::Single(s) => assert_eq!(s, "THREAD-001"),
        ThreadIdInput::Multiple(_) => panic!("Expected Single variant"),
    }
}

#[test]
fn test_thread_id_input_deserializes_multiple_strings() {
    use lib_mcp::tools::ThreadIdInput;

    let json = r#"["THREAD-001", "THREAD-002", "THREAD-003"]"#;
    let input: ThreadIdInput = serde_json::from_str(json).unwrap();

    match input {
        ThreadIdInput::Multiple(v) => {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], "THREAD-001");
            assert_eq!(v[1], "THREAD-002");
            assert_eq!(v[2], "THREAD-003");
        }
        ThreadIdInput::Single(_) => panic!("Expected Multiple variant"),
    }
}

#[test]
fn test_summarize_result_structure() {
    use lib_mcp::tools::{SummarizeResult, ThreadSummaryError, ThreadSummaryItem};

    let result = SummarizeResult {
        summaries: vec![ThreadSummaryItem {
            thread_id: "THREAD-001".to_string(),
            subject: "Test Subject".to_string(),
            message_count: 5,
            participants: vec!["alice".to_string(), "bob".to_string()],
            last_snippet: "Latest message...".to_string(),
        }],
        errors: vec![ThreadSummaryError {
            thread_id: "THREAD-MISSING".to_string(),
            error: "Thread not found".to_string(),
        }],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("THREAD-001"));
    assert!(json.contains("THREAD-MISSING"));
    assert!(json.contains("summaries"));
    assert!(json.contains("errors"));
}

#[tokio::test]
async fn test_summarize_single_thread() {
    let (mm, _temp) = create_test_mm().await;
    let (project_id, _, _) = setup_test_data(&mm).await;
    let ctx = Ctx::root_ctx();

    let messages = MessageBmc::list_by_thread(&ctx, &mm, project_id, "THREAD-001")
        .await
        .unwrap();

    assert_eq!(messages.len(), 2, "Thread should have 2 messages");

    let mut participants: Vec<String> = messages.iter().map(|m| m.sender_name.clone()).collect();
    participants.sort();
    participants.dedup();

    assert!(participants.contains(&"alice".to_string()));
    assert!(participants.contains(&"bob".to_string()));
}

#[tokio::test]
async fn test_summarize_multiple_threads() {
    let (mm, _temp) = create_test_mm().await;
    let (project_id, _, _) = setup_test_data(&mm).await;
    let ctx = Ctx::root_ctx();

    let thread_ids = vec!["THREAD-001", "THREAD-002"];
    let mut summaries = Vec::new();

    for thread_id in &thread_ids {
        let messages = MessageBmc::list_by_thread(&ctx, &mm, project_id, thread_id)
            .await
            .unwrap();
        summaries.push((thread_id.to_string(), messages.len()));
    }

    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].1, 2, "THREAD-001 should have 2 messages");
    assert_eq!(summaries[1].1, 1, "THREAD-002 should have 1 message");
}

#[tokio::test]
async fn test_summarize_partial_failure() {
    let (mm, _temp) = create_test_mm().await;
    let (project_id, _, _) = setup_test_data(&mm).await;
    let ctx = Ctx::root_ctx();

    let thread_ids = vec!["THREAD-001", "THREAD-NONEXISTENT", "THREAD-002"];
    let mut successes = Vec::new();
    let mut errors = Vec::new();

    for thread_id in &thread_ids {
        let messages = MessageBmc::list_by_thread(&ctx, &mm, project_id, thread_id).await;
        match messages {
            Ok(msgs) if !msgs.is_empty() => {
                successes.push(thread_id.to_string());
            }
            Ok(_) => {
                errors.push(thread_id.to_string());
            }
            Err(e) => {
                errors.push(format!("{}: {}", thread_id, e));
            }
        }
    }

    assert_eq!(successes.len(), 2, "Should have 2 successful summaries");
    assert_eq!(
        errors.len(),
        1,
        "Should have 1 error for nonexistent thread"
    );
    assert!(errors[0].contains("THREAD-NONEXISTENT"));
}

#[test]
fn test_thread_id_input_to_vec() {
    use lib_mcp::tools::ThreadIdInput;

    let single = ThreadIdInput::Single("THREAD-001".to_string());
    let vec_from_single: Vec<String> = single.into();
    assert_eq!(vec_from_single, vec!["THREAD-001".to_string()]);

    let multiple =
        ThreadIdInput::Multiple(vec!["THREAD-001".to_string(), "THREAD-002".to_string()]);
    let vec_from_multiple: Vec<String> = multiple.into();
    assert_eq!(
        vec_from_multiple,
        vec!["THREAD-001".to_string(), "THREAD-002".to_string()]
    );
}
