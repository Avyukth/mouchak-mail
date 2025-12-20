use lib_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    message::{MessageBmc, MessageForCreate},
    product::ProductBmc,
    project::ProjectBmc,
};
use lib_mcp::tools::AgentMailService;
use rmcp::model::ReadResourceRequestParam;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_mcp_resource_api_schemes() -> anyhow::Result<()> {
    // 1. Setup Data
    let mm = Arc::new(ModelManager::new().await?);
    let ctx = lib_core::ctx::Ctx::root_ctx();

    // Create Project
    let project_slug = format!("res-proj-{}", Uuid::new_v4());
    let project_id = ProjectBmc::create(&ctx, &mm, &project_slug, "Resource Test Project").await?;

    // Create Agent
    let agent_name = "worker";
    let agent_id = AgentBmc::create(
        &ctx,
        &mm,
        AgentForCreate {
            project_id,
            name: agent_name.to_string(),
            program: "test".to_string(),
            model: "gpt-4".to_string(),
            task_description: "test".to_string(),
        },
    )
    .await?;

    // Create a message
    MessageBmc::create(
        &ctx,
        &mm,
        MessageForCreate {
            project_id,
            sender_id: agent_id,
            recipient_ids: vec![agent_id],
            cc_ids: None,
            bcc_ids: None,
            subject: "Lazy Load Subject".to_string(),
            body_md: "Secret Body Content".to_string(),
            thread_id: Some("T1".to_string()),
            importance: Some("normal".to_string()),
            ack_required: false,
        },
    )
    .await?;

    // Create a product
    let product_uid = format!("PROD-{}", Uuid::new_v4());
    let product_name = format!("Test Product {}", Uuid::new_v4());
    ProductBmc::ensure(&ctx, &mm, &product_uid, &product_name).await?;

    let service = AgentMailService::new_with_mm(mm.clone(), false);

    // --- TEST 1: resource://inbox (Lazy Load - No bodies) ---
    let uri = format!("resource://inbox/{}?project={}", agent_name, project_slug);
    let res = service
        .read_resource_impl(ReadResourceRequestParam { uri })
        .await?;
    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &res.contents[0] {
        assert!(text.contains("Lazy Load Subject"));
        assert!(
            !text.contains("Secret Body Content"),
            "Should not contain body by default"
        );
    }

    // --- TEST 2: resource://inbox (With bodies) ---
    let uri = format!(
        "resource://inbox/{}?project={}&include_bodies=true",
        agent_name, project_slug
    );
    let res = service
        .read_resource_impl(ReadResourceRequestParam { uri })
        .await?;
    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &res.contents[0] {
        assert!(text.contains("Lazy Load Subject"));
        assert!(
            text.contains("Secret Body Content"),
            "Should contain body when include_bodies=true"
        );
    }

    // --- TEST 3: resource://thread (With bodies) ---
    let uri = format!(
        "resource://thread/T1?project={}&include_bodies=true",
        project_slug
    );
    let res = service
        .read_resource_impl(ReadResourceRequestParam { uri })
        .await?;
    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &res.contents[0] {
        assert!(text.contains("Lazy Load Subject"));
        assert!(text.contains("Secret Body Content"));
    }

    // --- TEST 4: resource://agent ---
    let uri = format!("resource://agent/{}?project={}", agent_name, project_slug);
    let res = service
        .read_resource_impl(ReadResourceRequestParam { uri })
        .await?;
    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &res.contents[0] {
        assert!(text.contains(agent_name));
        assert!(text.contains("gpt-4"));
    }

    // --- TEST 5: resource://product ---
    let uri = format!("resource://product/{}", product_uid);
    let res = service
        .read_resource_impl(ReadResourceRequestParam { uri })
        .await?;
    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &res.contents[0] {
        assert!(text.contains("Test Product"));
        assert!(text.contains(&product_uid));
    }

    // --- TEST 6: resource://identity ---
    let uri = "resource://identity//abs/path/to/repo".to_string();
    let res = service
        .read_resource_impl(ReadResourceRequestParam { uri })
        .await?;
    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &res.contents[0] {
        assert!(text.contains("/abs/path/to/repo"));
        assert!(text.contains("repo-abs-path-to-repo"));
    }

    // --- TEST 7: Legacy agent-mail:// scheme ---
    let uri = format!("agent-mail://{}/inbox/{}", project_slug, agent_name);
    let res = service
        .read_resource_impl(ReadResourceRequestParam { uri })
        .await?;
    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &res.contents[0] {
        assert!(text.contains("Lazy Load Subject"));
    }

    Ok(())
}
