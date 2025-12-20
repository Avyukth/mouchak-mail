use lib_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    message::{MessageBmc, MessageForCreate},
    project::ProjectBmc,
};
use lib_mcp::tools::AgentMailService;
use rmcp::model::ReadResourceRequestParam;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_resources_gap_features() -> anyhow::Result<()> {
    // 1. Setup Data
    let mm = Arc::new(ModelManager::new().await?);
    let ctx = lib_core::ctx::Ctx::root_ctx();

    // Create Project
    let project_slug = format!("test-proj-{}", Uuid::new_v4());
    let project_id = ProjectBmc::create(&ctx, &mm, &project_slug, "Test Project").await?;

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

    // Create Thread & Message
    let thread_id = "THREAD-123";
    MessageBmc::create(
        &ctx,
        &mm,
        MessageForCreate {
            project_id,
            sender_id: agent_id,
            recipient_ids: vec![agent_id], // Self message
            cc_ids: None,
            bcc_ids: None,
            subject: "Test Subject".to_string(),
            body_md: "Test Body".to_string(),
            thread_id: Some(thread_id.to_string()),
            importance: Some("normal".to_string()),
            ack_required: false,
        },
    )
    .await?;

    // 2. Initialize Service with shared MM
    let service = AgentMailService::new_with_mm(mm.clone(), false);

    // 3. Test list_resources
    let res = service.list_resources_impl(None).await?;

    // Validate Inboxes
    let inbox_uri = format!("agent-mail://{}/inbox/{}", project_slug, agent_name);
    let outbox_uri = format!("agent-mail://{}/outbox/{}", project_slug, agent_name);
    let threads_uri = format!("agent-mail://{}/threads", project_slug);

    let has_inbox = res.resources.iter().any(|r| r.raw.uri == inbox_uri);
    let has_outbox = res.resources.iter().any(|r| r.raw.uri == outbox_uri);
    let has_threads = res.resources.iter().any(|r| r.raw.uri == threads_uri);

    assert!(has_inbox, "List resources should contain inbox");
    assert!(has_outbox, "List resources should contain outbox");
    assert!(has_threads, "List resources should contain threads");

    // 4. Test read_resource (Inbox) - with include_bodies=true to get message content
    let inbox_with_bodies_uri = format!("{}?include_bodies=true", inbox_uri);
    let inbox_req = ReadResourceRequestParam {
        uri: inbox_with_bodies_uri,
    };
    let read_res = service.read_resource_impl(inbox_req).await?;
    let content = &read_res.contents[0];
    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = content {
        assert!(
            text.contains("Test Body"),
            "Inbox should contain message body when include_bodies=true"
        );
    } else {
        panic!("Expected text content for inbox");
    }

    // 5. Test read_resource (Threads List)
    let threads_req = ReadResourceRequestParam {
        uri: threads_uri.clone(),
    };
    let threads_res = service.read_resource_impl(threads_req).await?;
    let content = &threads_res.contents[0];
    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = content {
        assert!(
            text.contains(thread_id),
            "Threads list should contain thread ID"
        );
        assert!(
            text.contains("Test Subject"),
            "Threads list should contain subject"
        );
    } else {
        panic!("Expected text content for threads list");
    }

    Ok(())
}
