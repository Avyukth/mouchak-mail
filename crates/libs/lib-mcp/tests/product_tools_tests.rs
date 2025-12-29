//! Integration tests for product-level cross-project tools.
//!
//! Tests product tools: ensure_product, link/unlink, list_products, product_inbox,
//! search_messages_product, and summarize_thread_product.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use lib_common::config::AppConfig;
use lib_core::ctx::Ctx;
use lib_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    message::{MessageBmc, MessageForCreate},
    product::ProductBmc,
    project::ProjectBmc,
};
use lib_mcp::tools::{
    AgentMailService, EnsureProductParams, LinkProjectToProductParams, ProductInboxParams,
    SearchMessagesProductParams, SummarizeThreadProductParams, ThreadIdInput,
    UnlinkProjectFromProductParams, products,
};
use libsql::Builder;
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

fn extract_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .first()
        .map(|c| format!("{:?}", c))
        .unwrap_or_default()
}

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_products.db");
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

    let app_config = Arc::new(AppConfig::default());
    let mm = ModelManager::new_for_test(conn, archive_root, app_config);
    (Arc::new(mm), temp_dir)
}

#[tokio::test]
async fn test_ensure_product_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let product_uid = format!("PROD-{}", Uuid::new_v4());
    let params = EnsureProductParams {
        product_uid: product_uid.clone(),
        name: "Test Product".to_string(),
    };

    let result = products::ensure_product_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Test Product"));
    assert!(output.contains(&product_uid));
}

#[tokio::test]
async fn test_ensure_product_impl_idempotent() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let product_uid = format!("PROD-{}", Uuid::new_v4());
    let product_name = "Idempotent Product".to_string();

    // First call - creates the product
    let params1 = EnsureProductParams {
        product_uid: product_uid.clone(),
        name: product_name.clone(),
    };
    let result1 = products::ensure_product_impl(&ctx, &mm, params1).await;
    assert!(result1.is_ok());

    // Second call - should return existing product (idempotent)
    let params2 = EnsureProductParams {
        product_uid: product_uid.clone(),
        name: product_name,
    };
    let result2 = products::ensure_product_impl(&ctx, &mm, params2).await;
    assert!(result2.is_ok());

    let output1 = extract_text(&result1.unwrap());
    let output2 = extract_text(&result2.unwrap());
    assert!(output1.contains(&product_uid));
    assert!(output2.contains(&product_uid));
}

#[tokio::test]
async fn test_link_project_to_product_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let product_uid = format!("PROD-{}", Uuid::new_v4());
    ProductBmc::ensure(&ctx, &mm, &product_uid, "Link Test Product")
        .await
        .unwrap();

    let project_slug = format!("link-proj-{}", Uuid::new_v4());
    ProjectBmc::create(&ctx, &mm, &project_slug, "Link Test Project")
        .await
        .unwrap();

    let params = LinkProjectToProductParams {
        product_uid: product_uid.clone(),
        project_slug: project_slug.clone(),
    };

    let result = products::link_project_to_product_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Linked"));
    assert!(output.contains(&project_slug));
    assert!(output.contains(&product_uid));
}

#[tokio::test]
async fn test_link_project_to_product_impl_product_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = format!("orphan-proj-{}", Uuid::new_v4());
    ProjectBmc::create(&ctx, &mm, &project_slug, "Orphan Project")
        .await
        .unwrap();

    let params = LinkProjectToProductParams {
        product_uid: "NONEXISTENT-PROD".to_string(),
        project_slug,
    };

    let result = products::link_project_to_product_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("Product not found"));
}

#[tokio::test]
async fn test_unlink_project_from_product_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let product_uid = format!("PROD-{}", Uuid::new_v4());
    let product = ProductBmc::ensure(&ctx, &mm, &product_uid, "Unlink Test Product")
        .await
        .unwrap();

    let project_slug = format!("unlink-proj-{}", Uuid::new_v4());
    let project_id = ProjectBmc::create(&ctx, &mm, &project_slug, "Unlink Test Project")
        .await
        .unwrap();

    ProductBmc::link_project(&ctx, &mm, product.id, project_id.into())
        .await
        .unwrap();

    let params = UnlinkProjectFromProductParams {
        product_uid: product_uid.clone(),
        project_slug: project_slug.clone(),
    };

    let result = products::unlink_project_from_product_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Unlinked"));
}

#[tokio::test]
async fn test_unlink_project_from_product_impl_not_linked() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let product_uid = format!("PROD-{}", Uuid::new_v4());
    ProductBmc::ensure(&ctx, &mm, &product_uid, "NotLinked Test Product")
        .await
        .unwrap();

    let project_slug = format!("notlinked-proj-{}", Uuid::new_v4());
    ProjectBmc::create(&ctx, &mm, &project_slug, "NotLinked Test Project")
        .await
        .unwrap();

    let params = UnlinkProjectFromProductParams {
        product_uid: product_uid.clone(),
        project_slug: project_slug.clone(),
    };

    let result = products::unlink_project_from_product_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("was not linked"));
}

#[tokio::test]
async fn test_list_products_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    ProductBmc::ensure(&ctx, &mm, "PROD-LIST-1", "List Product 1")
        .await
        .unwrap();
    ProductBmc::ensure(&ctx, &mm, "PROD-LIST-2", "List Product 2")
        .await
        .unwrap();

    let result = products::list_products_impl(&ctx, &mm).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Products"));
    assert!(output.contains("PROD-LIST-1"));
    assert!(output.contains("PROD-LIST-2"));
}

#[tokio::test]
async fn test_list_products_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let result = products::list_products_impl(&ctx, &mm).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Products (0)"));
}

#[tokio::test]
async fn test_product_inbox_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let product_uid = format!("PROD-{}", Uuid::new_v4());
    let product = ProductBmc::ensure(&ctx, &mm, &product_uid, "Inbox Test Product")
        .await
        .unwrap();

    let project_slug = format!("inbox-proj-{}", Uuid::new_v4());
    let project_id = ProjectBmc::create(&ctx, &mm, &project_slug, "Inbox Test Project")
        .await
        .unwrap();

    ProductBmc::link_project(&ctx, &mm, product.id, project_id.into())
        .await
        .unwrap();

    let agent_id = AgentBmc::create(
        &ctx,
        &mm,
        AgentForCreate {
            project_id,
            name: "inbox_agent".to_string(),
            program: "test".to_string(),
            model: "gpt-4".to_string(),
            task_description: "test agent".to_string(),
        },
    )
    .await
    .unwrap();

    MessageBmc::create(
        &ctx,
        &mm,
        MessageForCreate {
            project_id: project_id.into(),
            sender_id: agent_id.into(),
            recipient_ids: vec![agent_id.into()],
            cc_ids: None,
            bcc_ids: None,
            subject: "Product Inbox Test Message".to_string(),
            body_md: "Test body".to_string(),
            thread_id: None,
            importance: None,
            ack_required: false,
        },
    )
    .await
    .unwrap();

    let params = ProductInboxParams {
        product_uid,
        limit: Some(10),
    };

    let result = products::product_inbox_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Product Inbox"));
    assert!(output.contains("Product Inbox Test Message"));
}

#[tokio::test]
async fn test_product_inbox_impl_product_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ProductInboxParams {
        product_uid: "NONEXISTENT".to_string(),
        limit: None,
    };

    let result = products::product_inbox_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("Product not found"));
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

    ProductBmc::link_project(&ctx, &mm, product.id, project1_id.into()).await?;
    ProductBmc::link_project(&ctx, &mm, product.id, project2_id.into()).await?;

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
            project_id: project1_id.into(),
            sender_id: agent1_id.into(),
            recipient_ids: vec![agent1_id.into()],
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
            project_id: project2_id.into(),
            sender_id: agent2_id.into(),
            recipient_ids: vec![agent2_id.into()],
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

    ProductBmc::link_project(&ctx, &mm, product.id, project1_id.into()).await?;
    ProductBmc::link_project(&ctx, &mm, product.id, project2_id.into()).await?;

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
            project_id: project1_id.into(),
            sender_id: agent1_id.into(),
            recipient_ids: vec![agent1_id.into()],
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
            project_id: project2_id.into(),
            sender_id: agent2_id.into(),
            recipient_ids: vec![agent2_id.into()],
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
    ProductBmc::link_project(&ctx, &mm, product.id, project_id.into()).await?;

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
    ProductBmc::link_project(&ctx, &mm, product.id, project_id.into()).await?;

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
