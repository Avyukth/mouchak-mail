//! Query locality tests
//!
//! Verifies that database queries use the correct indexes and perform well.
//! Uses EXPLAIN QUERY PLAN to validate implementation-level index usage.

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::Result;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::project::ProjectBmc;
use serial_test::serial;

/// Helper to setup data for queries
async fn setup_data(tc: &TestContext) -> (i64, i64) {
    let p_id = ProjectBmc::create(&tc.ctx, &tc.mm, "q-proj", "Query Project")
        .await
        .unwrap();
    let a_id = AgentBmc::create(
        &tc.ctx,
        &tc.mm,
        AgentForCreate {
            project_id: p_id,
            name: "q-agent".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "test".into(),
        },
    )
    .await
    .unwrap();

    // Create some messages
    for i in 0..5 {
        MessageBmc::create(
            &tc.ctx,
            &tc.mm,
            MessageForCreate {
                project_id: p_id,
                sender_id: a_id,
                recipient_ids: vec![],
                cc_ids: None,
                bcc_ids: None,
                subject: format!("Test Message {}", i),
                body_md: "Sample body content for indexing".into(),
                thread_id: Some("thread-1".into()),
                importance: None,
                ack_required: false,
            },
        )
        .await
        .unwrap();
    }

    (p_id, a_id)
}

#[tokio::test]
#[serial]
async fn test_thread_list_query_uses_indexes() -> Result<()> {
    let tc = TestContext::new().await?;
    let (p_id, _) = setup_data(&tc).await;

    // The query used in MessageBmc::list_threads (approximately)
    let sql = r#"
        SELECT
            m.thread_id,
            MIN(m.subject) as subject,
            COUNT(*) as message_count,
            MAX(m.created_ts) as last_message_ts
        FROM messages AS m
        WHERE m.project_id = ? AND m.thread_id IS NOT NULL
        GROUP BY m.thread_id
        ORDER BY last_message_ts DESC
        LIMIT 10
    "#;

    let plans = tc.explain_query_plan(sql).await?;

    // We expect "USING INDEX" for the WHERE clause (project_id)
    // and potentially for the GROUP BY/ORDER BY
    let uses_index = plans
        .iter()
        .any(|p| p.contains("USING INDEX") || p.contains("USING COVERING INDEX"));

    assert!(
        uses_index,
        "Query plan for list_threads should use an index. Plans: {:?}",
        plans
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_thread_messages_all_query_uses_index() -> Result<()> {
    let tc = TestContext::new().await?;
    let (p_id, _) = setup_data(&tc).await;

    // list_by_thread(p_id, "thread-1")
    let sql = r#"
        SELECT
            m.id, m.project_id, m.sender_id, ag.name as sender_name, m.thread_id, m.subject, m.body_md,
            m.importance, m.ack_required, m.created_ts, m.attachments
        FROM messages AS m
        JOIN agents AS ag ON m.sender_id = ag.id
        WHERE m.project_id = ? AND m.thread_id = ?
        ORDER BY m.created_ts ASC
    "#;

    let plans = tc.explain_query_plan(sql).await?;

    let uses_index = plans
        .iter()
        .any(|p| p.contains("USING INDEX") || p.contains("USING COVERING INDEX"));

    assert!(
        uses_index,
        "Query plan for list_by_thread should use an index. Plans: {:?}",
        plans
    );

    // Specifically check that it uses indices on messages table, not just a scan and filter
    let uses_message_index = plans
        .iter()
        .any(|p| p.contains("TABLE messages USING INDEX"));
    assert!(
        uses_message_index,
        "Should specifically use an index on messages table. Plans: {:?}",
        plans
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_fts_search_query_uses_fts_index() -> Result<()> {
    let tc = TestContext::new().await?;
    let (p_id, _) = setup_data(&tc).await;

    // MessageBmc::search(p_id, "query")
    // Note: FTS5 queries use MATCH
    let sql = r#"
        SELECT id FROM messages_fts WHERE body_md MATCH 'sample'
    "#;

    let plans = tc.explain_query_plan(sql).await?;

    let uses_fts = plans.iter().any(|p| p.contains("VIRTUAL TABLE INDEX 1:"));

    assert!(
        uses_fts,
        "FTS query should use virtual table index. Plans: {:?}",
        plans
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_message_detail_query_uses_primary_key() -> Result<()> {
    let tc = TestContext::new().await?;
    let (p_id, a_id) = setup_data(&tc).await;

    // MessageBmc::get(1)
    let sql = r#"
        SELECT m.id FROM messages AS m WHERE m.id = ?
    "#;

    let plans = tc.explain_query_plan(sql).await?;

    let uses_pk = plans
        .iter()
        .any(|p| p.contains("USING INTEGER PRIMARY KEY"));

    assert!(
        uses_pk,
        "Single message lookup should use Primary Key. Plans: {:?}",
        plans
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_query_scalability_with_limits() -> Result<()> {
    let tc = TestContext::new().await?;
    let (p_id, _) = setup_data(&tc).await;

    // Queries with limits should still use indices
    let sql = r#"
        SELECT id FROM messages WHERE project_id = ? ORDER BY created_ts DESC LIMIT 1
    "#;

    let plans = tc.explain_query_plan(sql).await?;

    let uses_index = plans.iter().any(|p| p.contains("USING INDEX"));

    assert!(
        uses_index,
        "Limited query should use index. Plans: {:?}",
        plans
    );

    Ok(())
}
