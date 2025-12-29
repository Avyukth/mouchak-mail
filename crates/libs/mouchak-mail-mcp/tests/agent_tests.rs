#![allow(clippy::unwrap_used, clippy::expect_used)]

use libsql::Builder;
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::{ModelManager, agent::AgentBmc, project::ProjectBmc};
use mouchak_mail_mcp::tools::agent;
use mouchak_mail_mcp::tools::{
    CreateAgentIdentityParams, GetAgentProfileParams, ListAgentsParams, RegisterAgentParams,
    UpdateAgentProfileParams, WhoisParams,
};
use std::sync::Arc;
use tempfile::TempDir;

fn extract_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .first()
        .map(|c| format!("{:?}", c))
        .unwrap_or_default()
}

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_agent.db");
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

async fn setup_project(mm: &Arc<ModelManager>, suffix: &str) -> String {
    let ctx = Ctx::root_ctx();
    let project_slug = format!("agent_project_{}", suffix);
    ProjectBmc::create(
        &ctx,
        mm,
        &project_slug,
        &format!("Agent Project {}", suffix),
    )
    .await
    .unwrap();
    project_slug
}

#[tokio::test]
async fn test_register_agent_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "register").await;

    let params = RegisterAgentParams {
        project_slug: project_slug.clone(),
        name: "TestAgent".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "Testing agent registration".to_string(),
    };

    let result = agent::register_agent_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Registered agent"));
    assert!(output.contains("TestAgent"));
    assert!(output.contains("default capabilities"));
}

#[tokio::test]
async fn test_register_agent_impl_already_exists() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "exists").await;

    let params1 = RegisterAgentParams {
        project_slug: project_slug.clone(),
        name: "ExistingAgent".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "First registration".to_string(),
    };

    agent::register_agent_impl(&ctx, &mm, params1)
        .await
        .unwrap();

    let params2 = RegisterAgentParams {
        project_slug: project_slug.clone(),
        name: "ExistingAgent".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "First registration".to_string(),
    };

    let result = agent::register_agent_impl(&ctx, &mm, params2).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("already exists"));
}

#[tokio::test]
async fn test_register_agent_impl_invalid_name() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "invalid_name").await;

    let params = RegisterAgentParams {
        project_slug,
        name: "invalid-name-with-hyphens".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "Should fail".to_string(),
    };

    let result = agent::register_agent_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_register_agent_impl_project_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = RegisterAgentParams {
        project_slug: "nonexistent_project".to_string(),
        name: "SomeAgent".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "Should fail".to_string(),
    };

    let result = agent::register_agent_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_whois_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "whois").await;

    let register_params = RegisterAgentParams {
        project_slug: project_slug.clone(),
        name: "WhoisAgent".to_string(),
        program: "claude_code".to_string(),
        model: "sonnet".to_string(),
        task_description: "Agent for whois test".to_string(),
    };
    agent::register_agent_impl(&ctx, &mm, register_params)
        .await
        .unwrap();

    let params = WhoisParams {
        project_slug,
        agent_name: "WhoisAgent".to_string(),
    };

    let result = agent::whois_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("WhoisAgent"));
    assert!(output.contains("claude_code"));
    assert!(output.contains("sonnet"));
}

#[tokio::test]
async fn test_whois_impl_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "whois_notfound").await;

    let params = WhoisParams {
        project_slug,
        agent_name: "NonexistentAgent".to_string(),
    };

    let result = agent::whois_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_agent_profile_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "update_profile").await;

    let register_params = RegisterAgentParams {
        project_slug: project_slug.clone(),
        name: "ProfileAgent".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "Original task".to_string(),
    };
    agent::register_agent_impl(&ctx, &mm, register_params)
        .await
        .unwrap();

    let params = UpdateAgentProfileParams {
        project_slug: project_slug.clone(),
        agent_name: "ProfileAgent".to_string(),
        task_description: Some("Updated task description".to_string()),
        attachments_policy: Some("reject".to_string()),
        contact_policy: Some("manual".to_string()),
    };

    let result = agent::update_agent_profile_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Updated profile"));

    let project = ProjectBmc::get_by_identifier(&ctx, &mm, &project_slug)
        .await
        .unwrap();
    let agent = AgentBmc::get_by_name(&ctx, &mm, project.id, "ProfileAgent")
        .await
        .unwrap();
    assert_eq!(agent.task_description, "Updated task description");
}

#[tokio::test]
async fn test_update_agent_profile_impl_agent_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "update_notfound").await;

    let params = UpdateAgentProfileParams {
        project_slug,
        agent_name: "NonexistentAgent".to_string(),
        task_description: Some("New task".to_string()),
        attachments_policy: None,
        contact_policy: None,
    };

    let result = agent::update_agent_profile_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_agent_profile_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "get_profile").await;

    let register_params = RegisterAgentParams {
        project_slug: project_slug.clone(),
        name: "DetailedAgent".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "Detailed profile test".to_string(),
    };
    agent::register_agent_impl(&ctx, &mm, register_params)
        .await
        .unwrap();

    let params = GetAgentProfileParams {
        project_slug,
        agent_name: "DetailedAgent".to_string(),
    };

    let result = agent::get_agent_profile_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("DetailedAgent"));
    assert!(output.contains("Messages Sent"));
    assert!(output.contains("Messages Received"));
    assert!(output.contains("Active Reservations"));
}

#[tokio::test]
async fn test_get_agent_profile_impl_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "profile_notfound").await;

    let params = GetAgentProfileParams {
        project_slug,
        agent_name: "GhostAgent".to_string(),
    };

    let result = agent::get_agent_profile_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_agents_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "list_empty").await;

    let params = ListAgentsParams {
        project_slug: project_slug.clone(),
    };

    let result = agent::list_agents_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains(&project_slug));
    assert!(output.contains("(0)"));
}

#[tokio::test]
async fn test_list_agents_impl_with_agents() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "list_with").await;

    for name in ["AgentAlpha", "AgentBeta", "AgentGamma"] {
        let params = RegisterAgentParams {
            project_slug: project_slug.clone(),
            name: name.to_string(),
            program: "claude_code".to_string(),
            model: "opus".to_string(),
            task_description: format!("Task for {}", name),
        };
        agent::register_agent_impl(&ctx, &mm, params).await.unwrap();
    }

    let params = ListAgentsParams {
        project_slug: project_slug.clone(),
    };

    let result = agent::list_agents_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("(3)"));
    assert!(output.contains("AgentAlpha"));
    assert!(output.contains("AgentBeta"));
    assert!(output.contains("AgentGamma"));
}

#[tokio::test]
async fn test_create_agent_identity_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "identity").await;

    let params = CreateAgentIdentityParams {
        project_slug,
        hint: None,
    };

    let result = agent::create_agent_identity_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Suggested name"));
    assert!(output.contains("Alternatives"));
}

#[tokio::test]
async fn test_create_agent_identity_impl_with_hint() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "identity_hint").await;

    let params = CreateAgentIdentityParams {
        project_slug,
        hint: Some("Blue".to_string()),
    };

    let result = agent::create_agent_identity_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Suggested name"));
    assert!(output.contains("Blue"));
}

#[tokio::test]
async fn test_register_agent_impl_unix_username_hint() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let project_slug = setup_project(&mm, "unix_hint").await;

    let params = RegisterAgentParams {
        project_slug: project_slug.clone(),
        name: "ubuntu".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "Testing unix username hint".to_string(),
    };

    let result = agent::register_agent_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Registered agent"));
    assert!(output.contains("Hint:"));
}

#[tokio::test]
async fn test_register_agent_impl_no_unix_hint_for_proper_name() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let project_slug = setup_project(&mm, "no_unix_hint").await;

    let params = RegisterAgentParams {
        project_slug: project_slug.clone(),
        name: "BlueMountain".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "Testing no unix hint".to_string(),
    };

    let result = agent::register_agent_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Registered agent"));
    assert!(!output.contains("Hint:"));
}

#[tokio::test]
async fn test_create_agent_identity_impl_with_non_matching_hint() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let project_slug = setup_project(&mm, "identity_nonmatch").await;

    let params = CreateAgentIdentityParams {
        project_slug,
        hint: Some("xyz123".to_string()),
    };

    let result = agent::create_agent_identity_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Suggested name"));
    assert!(output.contains("Alternatives"));
    assert!(!output.contains("xyz123"));
}

#[tokio::test]
async fn test_create_agent_identity_impl_avoids_existing() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = setup_project(&mm, "identity_avoid").await;

    let register_params = RegisterAgentParams {
        project_slug: project_slug.clone(),
        name: "BlueMountain".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "Existing agent".to_string(),
    };
    agent::register_agent_impl(&ctx, &mm, register_params)
        .await
        .unwrap();

    let params = CreateAgentIdentityParams {
        project_slug,
        hint: Some("Blue".to_string()),
    };

    let result = agent::create_agent_identity_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(!output.contains("BlueMountain") || output.contains("Alternatives"));
}

#[tokio::test]
async fn test_list_agents_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListAgentsParams {
        project_slug: "nonexistent_project".to_string(),
    };

    let result = agent::list_agents_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_agent_identity_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = CreateAgentIdentityParams {
        project_slug: "nonexistent_project".to_string(),
        hint: None,
    };

    let result = agent::create_agent_identity_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_whois_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = WhoisParams {
        project_slug: "nonexistent_project".to_string(),
        agent_name: "SomeAgent".to_string(),
    };

    let result = agent::whois_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_agent_profile_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = GetAgentProfileParams {
        project_slug: "nonexistent_project".to_string(),
        agent_name: "SomeAgent".to_string(),
    };

    let result = agent::get_agent_profile_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_agent_profile_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = UpdateAgentProfileParams {
        project_slug: "nonexistent_project".to_string(),
        agent_name: "SomeAgent".to_string(),
        task_description: Some("New task".to_string()),
        attachments_policy: None,
        contact_policy: None,
    };

    let result = agent::update_agent_profile_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_register_agent_impl_invalid_project_slug() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = RegisterAgentParams {
        project_slug: "has spaces invalid".to_string(),
        name: "ValidAgent".to_string(),
        program: "claude_code".to_string(),
        model: "opus".to_string(),
        task_description: "Should fail".to_string(),
    };

    let result = agent::register_agent_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}
