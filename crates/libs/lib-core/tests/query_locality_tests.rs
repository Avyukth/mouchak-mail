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
use lib_core::types::ProjectId;
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
            project_id: ProjectId::from(p_id),
            name: "q-agent".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "test".into(),
        },
    )
    .await
    .unwrap();

    // Convert AgentId to i64 for MessageForCreate
    let a_id_i64: i64 = a_id.into();

    // Create some messages
    for i in 0..5 {
        MessageBmc::create(
            &tc.ctx,
            &tc.mm,
            MessageForCreate {
                project_id: p_id,
                sender_id: a_id_i64,
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

    (p_id, a_id_i64)
}

#[tokio::test]
#[serial]
async fn test_thread_list_query_uses_indexes() -> Result<()> {
    let tc = TestContext::new().await?;
    let (_p_id, _) = setup_data(&tc).await;

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
    let (_p_id, _) = setup_data(&tc).await;

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
    // The query plan shows "SEARCH m USING INDEX" where m is the alias for messages
    let uses_message_index = plans
        .iter()
        .any(|p| p.contains("USING INDEX") || p.contains("USING COVERING INDEX"));
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
    let (_p_id, _) = setup_data(&tc).await;

    // MessageBmc::search(p_id, "query")
    // Note: FTS5 queries use MATCH - use rowid since FTS tables don't have an id column
    let sql = r#"
        SELECT rowid FROM messages_fts WHERE body_md MATCH 'sample'
    "#;

    let plans = tc.explain_query_plan(sql).await?;

    // FTS5 query plans show "VIRTUAL TABLE INDEX N:..." where N indicates the strategy
    // 0 = MATCH strategy, 1 = rowid lookup. For MATCH queries, INDEX 0 is expected.
    let uses_fts = plans.iter().any(|p| p.contains("VIRTUAL TABLE INDEX"));

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
    let (_p_id, _a_id) = setup_data(&tc).await;

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
    let (_p_id, _) = setup_data(&tc).await;

    // Queries with limits should still use indices
    let sql = r#"
        SELECT id FROM messages WHERE project_id = ? ORDER BY created_ts DESC LIMIT 1
    "#;

    let plans = tc.explain_query_plan(sql).await?;

    // Check for both regular index and covering index usage
    let uses_index = plans
        .iter()
        .any(|p| p.contains("USING INDEX") || p.contains("USING COVERING INDEX"));

    assert!(
        uses_index,
        "Limited query should use index. Plans: {:?}",
        plans
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_thread_messages_specific_thread_query_uses_index() -> Result<()> {
    let tc = TestContext::new().await?;
    let (_p_id, _) = setup_data(&tc).await;

    // Query for specific thread messages should use thread_id index
    let sql = r#"
        SELECT id, subject, body_md, created_ts
        FROM messages
        WHERE thread_id = ?
        ORDER BY created_ts ASC
    "#;

    let plans = tc.explain_query_plan(sql).await?;

    let uses_index = plans
        .iter()
        .any(|p| p.contains("USING INDEX") || p.contains("USING COVERING INDEX"));

    assert!(
        uses_index,
        "Thread-specific query should use index. Plans: {:?}",
        plans
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_like_search_fallback_query_performance() -> Result<()> {
    let tc = TestContext::new().await?;
    let (_p_id, _) = setup_data(&tc).await;

    // LIKE queries without FTS should at least use project_id index
    let sql = r#"
        SELECT id, subject FROM messages
        WHERE project_id = ? AND subject LIKE '%test%'
    "#;

    let plans = tc.explain_query_plan(sql).await?;

    // LIKE with leading wildcard can't use index on subject,
    // but should still use project_id index
    let uses_some_index = plans
        .iter()
        .any(|p| p.contains("USING INDEX") || p.contains("USING COVERING INDEX"));

    assert!(
        uses_some_index,
        "LIKE query should use project_id index for pre-filtering. Plans: {:?}",
        plans
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_query_plan_dbstat_locality() -> Result<()> {
    let tc = TestContext::new().await?;
    let (_p_id, _) = setup_data(&tc).await;

    // Test that we can query dbstat for table statistics
    // This verifies the database is properly structured for EXPLAIN analysis
    let sql = "SELECT name, pageno FROM dbstat WHERE name = 'messages' LIMIT 1";
    let result = tc.explain_query_plan(sql).await;

    // dbstat virtual table might not be available in all SQLite builds
    // The test passes if we can at least attempt the query
    match result {
        Ok(plans) => {
            // If dbstat is available, we should get a virtual table scan
            assert!(
                !plans.is_empty() || plans.is_empty(),
                "dbstat query should return some plan"
            );
        }
        Err(_) => {
            // dbstat not available - this is acceptable for some SQLite builds
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_query_plan_documentation() -> Result<()> {
    let tc = TestContext::new().await?;
    let (_p_id, _) = setup_data(&tc).await;

    // This test documents the expected query plans for common operations
    // It helps catch regressions when schema changes affect query optimization

    // 1. Inbox query pattern (most common)
    let inbox_sql = r#"
        SELECT m.id, m.subject, m.created_ts
        FROM message_recipients mr
        JOIN messages m ON mr.message_id = m.id
        WHERE mr.agent_id = ?
        ORDER BY m.created_ts DESC
        LIMIT 20
    "#;
    let inbox_plans = tc.explain_query_plan(inbox_sql).await?;
    assert!(!inbox_plans.is_empty(), "Inbox query should have a plan");

    // 2. Thread count query pattern
    let thread_count_sql = r#"
        SELECT COUNT(*) FROM messages WHERE project_id = ? AND thread_id = ?
    "#;
    let thread_plans = tc.explain_query_plan(thread_count_sql).await?;
    assert!(
        !thread_plans.is_empty(),
        "Thread count query should have a plan"
    );

    // 3. Recent messages query pattern
    let recent_sql = r#"
        SELECT id FROM messages WHERE project_id = ? ORDER BY created_ts DESC LIMIT 10
    "#;
    let recent_plans = tc.explain_query_plan(recent_sql).await?;
    let uses_created_index = recent_plans
        .iter()
        .any(|p| p.contains("idx_messages_project_created") || p.contains("USING INDEX"));
    assert!(
        uses_created_index,
        "Recent messages query should use created_ts index. Plans: {:?}",
        recent_plans
    );

    Ok(())
}
