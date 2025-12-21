//! Integration tests for product-level cross-project tools.
//!
//! Tests `search_messages_product` and `summarize_thread_product` tools.

use lib_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    message::{MessageBmc, MessageForCreate},
    product::ProductBmc,
    project::ProjectBmc,
};
use lib_mcp::tools::{
    AgentMailService, SearchMessagesProductParams, SummarizeThreadProductParams, ThreadIdInput,
};
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;
use uuid::Uuid;

/// Helper to extract text from CallToolResult content via Debug format
fn extract_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .first()
        .map(|c| format!("{:?}", c))
        .unwrap_or_default()
}

#[tokio::test]
async fn test_search_messages_product() -> anyhow::Result<()> {
    let mm = Arc::new(
        ModelManager::new(std::sync::Arc::new(lib_common::config::AppConfig::default())).await?,
    );
    let ctx = lib_core::ctx::Ctx::root_ctx();

    // Create product (name must be unique)
    let product_uid = format!("PROD-{}", Uuid::new_v4());
    let product_name = format!("Cross-Project Search Test {}", Uuid::new_v4());
    let product = ProductBmc::ensure(&ctx, &mm, &product_uid, &product_name).await?;

    // Create two projects and link them to the product
    let project1_slug = format!("search-proj1-{}", Uuid::new_v4());
    let project2_slug = format!("search-proj2-{}", Uuid::new_v4());
    let project1_id = ProjectBmc::create(&ctx, &mm, &project1_slug, "Search Project 1").await?;
    let project2_id = ProjectBmc::create(&ctx, &mm, &project2_slug, "Search Project 2").await?;

    ProductBmc::link_project(&ctx, &mm, product.id, project1_id).await?;
    ProductBmc::link_project(&ctx, &mm, product.id, project2_id).await?;

    // Create agents in each project
    let agent1_id = AgentBmc::create(
        &ctx,
        &mm,
        AgentForCreate {
            project_id: project1_id,
            name: "alice".to_string(),
            program: "test".to_string(),
            model: "gpt-4".to_string(),
            task_description: "test agent".to_string(),
        },
    )
    .await?;

    let agent2_id = AgentBmc::create(
        &ctx,
        &mm,
        AgentForCreate {
            project_id: project2_id,
            name: "bob".to_string(),
            program: "test".to_string(),
            model: "gpt-4".to_string(),
            task_description: "test agent".to_string(),
        },
    )
    .await?;

    // Create messages with searchable content (FTS5 indexes body_md, not subject)
    MessageBmc::create(
        &ctx,
        &mm,
        MessageForCreate {
            project_id: project1_id,
            sender_id: agent1_id,
            recipient_ids: vec![agent1_id],
            cc_ids: None,
            bcc_ids: None,
            subject: "Message in project one".to_string(),
            body_md: "This contains UNIQUESEARCHTERM in project one body".to_string(),
            thread_id: Some("T-SEARCH-1".to_string()),
            importance: Some("normal".to_string()),
            ack_required: false,
        },
    )
    .await?;

    MessageBmc::create(
        &ctx,
        &mm,
        MessageForCreate {
            project_id: project2_id,
            sender_id: agent2_id,
            recipient_ids: vec![agent2_id],
            cc_ids: None,
            bcc_ids: None,
            subject: "Message in project two".to_string(),
            body_md: "This contains UNIQUESEARCHTERM in project two body".to_string(),
            thread_id: Some("T-SEARCH-2".to_string()),
            importance: Some("high".to_string()),
            ack_required: false,
        },
    )
    .await?;

    // Test search_messages_product
    let service = AgentMailService::new_with_mm(mm.clone(), false);
    let result = service
        .search_messages_product_impl(Parameters(SearchMessagesProductParams {
            product_uid: product_uid.clone(),
            query: "UNIQUESEARCHTERM".to_string(),
            limit: Some(10),
        }))
        .await?;

    let output = extract_text(&result);

    // Should find matches in both projects (check for project slugs or human names)
    assert!(
        output.contains("Search Project 1") || output.contains(&project1_slug),
        "Should find message in project 1: {}",
        output
    );
    assert!(
        output.contains("Search Project 2") || output.contains(&project2_slug),
        "Should find message in project 2: {}",
        output
    );
    assert!(
        output.contains("2 matches") || output.contains("Total: 2"),
        "Should show total matches: {}",
        output
    );

    Ok(())
}

#[tokio::test]
async fn test_summarize_thread_product() -> anyhow::Result<()> {
    let mm = Arc::new(
        ModelManager::new(std::sync::Arc::new(lib_common::config::AppConfig::default())).await?,
    );
    let ctx = lib_core::ctx::Ctx::root_ctx();

    // Create product (name must be unique)
    let product_uid = format!("PROD-{}", Uuid::new_v4());
    let product_name = format!("Thread Summary Test {}", Uuid::new_v4());
    let product = ProductBmc::ensure(&ctx, &mm, &product_uid, &product_name).await?;

    // Create two projects and link them
    let project1_slug = format!("sum-proj1-{}", Uuid::new_v4());
    let project2_slug = format!("sum-proj2-{}", Uuid::new_v4());
    let project1_id = ProjectBmc::create(&ctx, &mm, &project1_slug, "Summary Project 1").await?;
    let project2_id = ProjectBmc::create(&ctx, &mm, &project2_slug, "Summary Project 2").await?;

    ProductBmc::link_project(&ctx, &mm, product.id, project1_id).await?;
    ProductBmc::link_project(&ctx, &mm, product.id, project2_id).await?;

    // Create agents
    let agent1_id = AgentBmc::create(
        &ctx,
        &mm,
        AgentForCreate {
            project_id: project1_id,
            name: "charlie".to_string(),
            program: "test".to_string(),
            model: "gpt-4".to_string(),
            task_description: "test".to_string(),
        },
    )
    .await?;

    let agent2_id = AgentBmc::create(
        &ctx,
        &mm,
        AgentForCreate {
            project_id: project2_id,
            name: "diana".to_string(),
            program: "test".to_string(),
            model: "gpt-4".to_string(),
            task_description: "test".to_string(),
        },
    )
    .await?;

    // Create messages in the SAME thread across BOTH projects
    let shared_thread_id = "T-CROSS-PROJECT".to_string();

    MessageBmc::create(
        &ctx,
        &mm,
        MessageForCreate {
            project_id: project1_id,
            sender_id: agent1_id,
            recipient_ids: vec![agent1_id],
            cc_ids: None,
            bcc_ids: None,
            subject: "Cross-Project Thread Message".to_string(),
            body_md: "Message from project 1 in shared thread".to_string(),
            thread_id: Some(shared_thread_id.clone()),
            importance: Some("normal".to_string()),
            ack_required: false,
        },
    )
    .await?;

    MessageBmc::create(
        &ctx,
        &mm,
        MessageForCreate {
            project_id: project2_id,
            sender_id: agent2_id,
            recipient_ids: vec![agent2_id],
            cc_ids: None,
            bcc_ids: None,
            subject: "Cross-Project Thread Reply".to_string(),
            body_md: "Reply from project 2 in shared thread".to_string(),
            thread_id: Some(shared_thread_id.clone()),
            importance: Some("normal".to_string()),
            ack_required: false,
        },
    )
    .await?;

    // Test summarize_thread_product
    let service = AgentMailService::new_with_mm(mm.clone(), false);
    let result = service
        .summarize_thread_product_impl(Parameters(SummarizeThreadProductParams {
            product_uid: product_uid.clone(),
            thread_id: ThreadIdInput::Single(shared_thread_id.clone()),
        }))
        .await?;

    let output = extract_text(&result);

    // Should aggregate messages from both projects
    assert!(
        output.contains(&shared_thread_id),
        "Should contain thread ID: {}",
        output
    );
    assert!(
        output.contains("charlie") || output.contains("diana"),
        "Should list participants: {}",
        output
    );
    // The subject should indicate it came from multiple projects
    assert!(
        output.contains("message_count") || output.contains("2"),
        "Should aggregate message count: {}",
        output
    );

    Ok(())
}

#[tokio::test]
async fn test_search_messages_product_no_matches() -> anyhow::Result<()> {
    let mm = Arc::new(
        ModelManager::new(std::sync::Arc::new(lib_common::config::AppConfig::default())).await?,
    );
    let ctx = lib_core::ctx::Ctx::root_ctx();

    // Create product with a project but no matching messages (name must be unique)
    let product_uid = format!("PROD-{}", Uuid::new_v4());
    let product_name = format!("No Match Test {}", Uuid::new_v4());
    let product = ProductBmc::ensure(&ctx, &mm, &product_uid, &product_name).await?;

    let project_slug = format!("nomatch-{}", Uuid::new_v4());
    let project_id = ProjectBmc::create(&ctx, &mm, &project_slug, "No Match Project").await?;
    ProductBmc::link_project(&ctx, &mm, product.id, project_id).await?;

    let service = AgentMailService::new_with_mm(mm.clone(), false);
    let result = service
        .search_messages_product_impl(Parameters(SearchMessagesProductParams {
            product_uid,
            query: "NONEXISTENT_QUERY_12345".to_string(),
            limit: Some(10),
        }))
        .await?;

    let output = extract_text(&result);

    assert!(
        output.contains("No matches found"),
        "Should indicate no matches: {}",
        output
    );

    Ok(())
}

#[tokio::test]
async fn test_summarize_thread_product_not_found() -> anyhow::Result<()> {
    let mm = Arc::new(
        ModelManager::new(std::sync::Arc::new(lib_common::config::AppConfig::default())).await?,
    );
    let ctx = lib_core::ctx::Ctx::root_ctx();

    // Create product with a project but no matching thread (name must be unique)
    let product_uid = format!("PROD-{}", Uuid::new_v4());
    let product_name = format!("Thread Not Found Test {}", Uuid::new_v4());
    let product = ProductBmc::ensure(&ctx, &mm, &product_uid, &product_name).await?;

    let project_slug = format!("nothread-{}", Uuid::new_v4());
    let project_id = ProjectBmc::create(&ctx, &mm, &project_slug, "No Thread Project").await?;
    ProductBmc::link_project(&ctx, &mm, product.id, project_id).await?;

    let service = AgentMailService::new_with_mm(mm.clone(), false);
    let result = service
        .summarize_thread_product_impl(Parameters(SummarizeThreadProductParams {
            product_uid,
            thread_id: ThreadIdInput::Single("NONEXISTENT_THREAD".to_string()),
        }))
        .await?;

    let output = extract_text(&result);

    assert!(
        output.contains("errors") && output.contains("not found"),
        "Should indicate thread not found: {}",
        output
    );

    Ok(())
}
