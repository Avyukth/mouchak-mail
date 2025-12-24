// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

use lib_core::Result;
use lib_core::ctx::Ctx;
use lib_core::model::ModelManager;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::types::ProjectId;
use serial_test::serial;

// --- Test Setup Helper ---
async fn create_test_mm() -> ModelManager {
    // In-memory DB with migrations
    // FTS tables created automatically by migrations in ModelManager::new()
    ModelManager::new(std::sync::Arc::new(lib_common::config::AppConfig::default()))
        .await
        .unwrap()
}

async fn setup_project_and_agent(ctx: &Ctx, mm: &ModelManager, suffix: &str) -> (i64, i64) {
    // Start with "ftsproj-" so it is slug-safe (lowercase, etc)
    // Avoid underscores in slug usually, but uuid has hyphens.
    let random_id = uuid::Uuid::new_v4().to_string();
    let p_slug = format!("ftsproj-{}-{}", suffix, random_id);
    // human_key can be anything
    let p_id = ProjectBmc::create(ctx, mm, &p_slug, "FTS Project")
        .await
        .unwrap();

    let a_id = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id: ProjectId(p_id),
            name: format!("agent-{}", suffix),
            program: "test".into(),
            model: "test".into(),
            task_description: "test".into(),
        },
    )
    .await
    .unwrap();

    (p_id, a_id.into())
}

#[tokio::test]
#[serial]
async fn test_fts_wildcard_search() -> Result<()> {
    let mm = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (p_id, a_id) = setup_project_and_agent(&ctx, &mm, "wild").await;

    // 1. Create a message with specific word
    MessageBmc::create(
        &ctx,
        &mm,
        MessageForCreate {
            project_id: p_id,
            sender_id: a_id,
            recipient_ids: vec![],
            cc_ids: None,
            bcc_ids: None,
            subject: "Test Message".to_string(),
            body_md: "The quick brown fox jumps over the lazy dog".to_string(),
            thread_id: None,
            importance: None,
            ack_required: false,
        },
    )
    .await?;

    // 2. Search with prefix wildcard (standard FTS5)
    // "quick*" should match "quick" if we allow wildcards
    // Currently fails because we quote it as "quick*"
    let res = MessageBmc::search(&ctx, &mm, p_id, "quick*", 10).await?;
    assert_eq!(res.len(), 1, "Should match 'quick*' (prefix)");

    // 3. Search with phrase to ensure we don't break normal phrases
    let res2 = MessageBmc::search(&ctx, &mm, p_id, "\"brown fox\"", 10).await?;
    assert_eq!(res2.len(), 1, "Should match phrase \"brown fox\"");

    // 4. Leading wildcard (FTS5 syntax error typically)
    // We want to return empty (graceful) instead of error
    let res3 = MessageBmc::search(&ctx, &mm, p_id, "*dog", 10).await;
    // Assertion: Should be Ok(empty) or Ok(results) if supported, but NOT Err
    assert!(res3.is_ok(), "Should handle '*dog' gracefully (no crash)");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_fts_malformed_query_graceful() -> Result<()> {
    let mm = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (p_id, _) = setup_project_and_agent(&ctx, &mm, "err").await;

    // Unclosed quote - FTS5 throws error if passed raw
    // We want graceful empty result
    let res = MessageBmc::search(&ctx, &mm, p_id, "\"unclosed phrase", 10).await;

    // We expect OK (handled) and empty
    assert!(res.is_ok(), "Should return Ok for malformed FTS query");
    let messages = res.unwrap();
    assert!(
        messages.is_empty(),
        "Should return empty list for malformed query"
    );

    Ok(())
}
