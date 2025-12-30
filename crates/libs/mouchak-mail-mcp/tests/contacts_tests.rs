#![allow(clippy::unwrap_used, clippy::expect_used)]

use libsql::Builder;
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    project::ProjectBmc,
};
use mouchak_mail_mcp::tools::contacts;
use mouchak_mail_mcp::tools::{
    ListContactsParams, RequestContactParams, RespondContactByNameParams, RespondContactParams,
    SetContactPolicyParams,
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
    let db_path = temp_dir.path().join("test_contacts.db");
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

async fn setup_two_projects_with_agents(
    mm: &Arc<ModelManager>,
    suffix: &str,
) -> (String, String, String, String) {
    let ctx = Ctx::root_ctx();

    let project1_slug = format!("contacts-project-1-{}", suffix);
    let project1_id = ProjectBmc::create(&ctx, mm, &project1_slug, "Contacts Project 1")
        .await
        .unwrap();

    let agent1_c = AgentForCreate {
        project_id: project1_id,
        name: "alice".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Alice agent".to_string(),
    };
    AgentBmc::create(&ctx, mm, agent1_c).await.unwrap();

    let project2_slug = format!("contacts-project-2-{}", suffix);
    let project2_id = ProjectBmc::create(&ctx, mm, &project2_slug, "Contacts Project 2")
        .await
        .unwrap();

    let agent2_c = AgentForCreate {
        project_id: project2_id,
        name: "bob".to_string(),
        program: "claude".to_string(),
        model: "sonnet".to_string(),
        task_description: "Bob agent".to_string(),
    };
    AgentBmc::create(&ctx, mm, agent2_c).await.unwrap();

    (
        project1_slug,
        "alice".to_string(),
        project2_slug,
        "bob".to_string(),
    )
}

#[tokio::test]
async fn test_request_contact_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (proj1_slug, agent1_name, proj2_slug, agent2_name) =
        setup_two_projects_with_agents(&mm, "request").await;

    let params = RequestContactParams {
        from_project_slug: proj1_slug,
        from_agent_name: agent1_name,
        to_project_slug: proj2_slug,
        to_agent_name: agent2_name,
        reason: "Collaboration needed".to_string(),
    };

    let result = contacts::request_contact_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Contact request sent"));
    assert!(output.contains("pending"));
}

#[tokio::test]
async fn test_request_contact_impl_project_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = RequestContactParams {
        from_project_slug: "nonexistent-project".to_string(),
        from_agent_name: "agent".to_string(),
        to_project_slug: "another-nonexistent".to_string(),
        to_agent_name: "agent2".to_string(),
        reason: "Test".to_string(),
    };

    let result = contacts::request_contact_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("not found"));
}

#[tokio::test]
async fn test_respond_contact_impl_accept() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (proj1_slug, agent1_name, proj2_slug, agent2_name) =
        setup_two_projects_with_agents(&mm, "respond-accept").await;

    let request_params = RequestContactParams {
        from_project_slug: proj1_slug.clone(),
        from_agent_name: agent1_name.clone(),
        to_project_slug: proj2_slug.clone(),
        to_agent_name: agent2_name.clone(),
        reason: "Collaboration".to_string(),
    };

    let request_result = contacts::request_contact_impl(&ctx, &mm, request_params).await;
    assert!(request_result.is_ok());

    let output = extract_text(&request_result.unwrap());
    let link_id: i64 = output
        .split("link_id:")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
        .expect("Should extract link_id");

    let respond_params = RespondContactParams {
        link_id,
        accept: true,
    };

    let result = contacts::respond_contact_impl(&ctx, &mm, respond_params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("accepted"));
}

#[tokio::test]
async fn test_respond_contact_impl_reject() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (proj1_slug, agent1_name, proj2_slug, agent2_name) =
        setup_two_projects_with_agents(&mm, "respond-reject").await;

    let request_params = RequestContactParams {
        from_project_slug: proj1_slug,
        from_agent_name: agent1_name,
        to_project_slug: proj2_slug,
        to_agent_name: agent2_name,
        reason: "Test rejection".to_string(),
    };

    let request_result = contacts::request_contact_impl(&ctx, &mm, request_params).await;
    let output = extract_text(&request_result.unwrap());
    let link_id: i64 = output
        .split("link_id:")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
        .expect("Should extract link_id");

    let respond_params = RespondContactParams {
        link_id,
        accept: false,
    };

    let result = contacts::respond_contact_impl(&ctx, &mm, respond_params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("rejected"));
}

#[tokio::test]
async fn test_list_contacts_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "empty-contacts-project";
    let project_id = ProjectBmc::create(&ctx, &mm, project_slug, "Empty Contacts Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "lonely_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent with no contacts".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = ListContactsParams {
        project_slug: project_slug.to_string(),
        agent_name: "lonely_agent".to_string(),
    };

    let result = contacts::list_contacts_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Contacts for"));
    assert!(output.contains("(0)"));
}

#[tokio::test]
async fn test_list_contacts_impl_with_contacts() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (proj1_slug, agent1_name, proj2_slug, agent2_name) =
        setup_two_projects_with_agents(&mm, "with-contacts").await;

    let request_params = RequestContactParams {
        from_project_slug: proj1_slug.clone(),
        from_agent_name: agent1_name.clone(),
        to_project_slug: proj2_slug,
        to_agent_name: agent2_name,
        reason: "Contact for listing".to_string(),
    };
    contacts::request_contact_impl(&ctx, &mm, request_params)
        .await
        .unwrap();

    let params = ListContactsParams {
        project_slug: proj1_slug,
        agent_name: agent1_name,
    };

    let result = contacts::list_contacts_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Contacts for"));
}

#[tokio::test]
async fn test_list_contacts_impl_agent_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "list-contacts-notfound";
    ProjectBmc::create(&ctx, &mm, project_slug, "List Contacts NotFound Project")
        .await
        .unwrap();

    let params = ListContactsParams {
        project_slug: project_slug.to_string(),
        agent_name: "nonexistent_agent".to_string(),
    };

    let result = contacts::list_contacts_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("not found"));
}

#[tokio::test]
async fn test_set_contact_policy_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "policy-project";
    let project_id = ProjectBmc::create(&ctx, &mm, project_slug, "Policy Test Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "policy_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent for policy test".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = SetContactPolicyParams {
        project_slug: project_slug.to_string(),
        agent_name: "policy_agent".to_string(),
        contact_policy: "auto".to_string(),
    };

    let result = contacts::set_contact_policy_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Contact policy"));
    assert!(output.contains("auto"));
}

#[tokio::test]
async fn test_set_contact_policy_impl_manual() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "manual-policy-project";
    let project_id = ProjectBmc::create(&ctx, &mm, project_slug, "Manual Policy Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "manual_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent for manual policy".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = SetContactPolicyParams {
        project_slug: project_slug.to_string(),
        agent_name: "manual_agent".to_string(),
        contact_policy: "manual".to_string(),
    };

    let result = contacts::set_contact_policy_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("manual"));
}

#[tokio::test]
async fn test_set_contact_policy_impl_deny() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "deny-policy-project";
    let project_id = ProjectBmc::create(&ctx, &mm, project_slug, "Deny Policy Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "deny_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent for deny policy".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = SetContactPolicyParams {
        project_slug: project_slug.to_string(),
        agent_name: "deny_agent".to_string(),
        contact_policy: "deny".to_string(),
    };

    let result = contacts::set_contact_policy_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("deny"));
}

#[tokio::test]
async fn test_set_contact_policy_impl_agent_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "policy-notfound";
    ProjectBmc::create(&ctx, &mm, project_slug, "Policy NotFound Project")
        .await
        .unwrap();

    let params = SetContactPolicyParams {
        project_slug: project_slug.to_string(),
        agent_name: "nonexistent".to_string(),
        contact_policy: "auto".to_string(),
    };

    let result = contacts::set_contact_policy_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("not found"));
}

async fn setup_single_project_with_two_agents(
    mm: &Arc<ModelManager>,
    suffix: &str,
) -> (String, String, String) {
    let ctx = Ctx::root_ctx();

    let project_slug = format!("contacts-same-project-{}", suffix);
    let project_id = ProjectBmc::create(&ctx, mm, &project_slug, "Contacts Same Project")
        .await
        .unwrap();

    let agent1_c = AgentForCreate {
        project_id,
        name: format!("alice_{}", suffix),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Alice agent".to_string(),
    };
    AgentBmc::create(&ctx, mm, agent1_c).await.unwrap();

    let agent2_c = AgentForCreate {
        project_id,
        name: format!("bob_{}", suffix),
        program: "claude".to_string(),
        model: "sonnet".to_string(),
        task_description: "Bob agent".to_string(),
    };
    AgentBmc::create(&ctx, mm, agent2_c).await.unwrap();

    (
        project_slug,
        format!("alice_{}", suffix),
        format!("bob_{}", suffix),
    )
}

#[tokio::test]
async fn test_respond_contact_by_name_impl_accept() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, alice, bob) =
        setup_single_project_with_two_agents(&mm, "byname-accept").await;

    let request_params = RequestContactParams {
        from_project_slug: project_slug.clone(),
        from_agent_name: alice.clone(),
        to_project_slug: project_slug.clone(),
        to_agent_name: bob.clone(),
        reason: "Testing name-based response".to_string(),
    };
    contacts::request_contact_impl(&ctx, &mm, request_params)
        .await
        .unwrap();

    let respond_params = RespondContactByNameParams {
        project_slug: project_slug.clone(),
        to_agent: bob.clone(),
        from_agent: alice.clone(),
        accept: true,
    };

    let result = contacts::respond_contact_by_name_impl(&ctx, &mm, respond_params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("accepted"));
    assert!(output.contains("link_id"));
}

#[tokio::test]
async fn test_respond_contact_by_name_impl_reject() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, alice, bob) =
        setup_single_project_with_two_agents(&mm, "byname-reject").await;

    let request_params = RequestContactParams {
        from_project_slug: project_slug.clone(),
        from_agent_name: alice.clone(),
        to_project_slug: project_slug.clone(),
        to_agent_name: bob.clone(),
        reason: "Testing rejection".to_string(),
    };
    contacts::request_contact_impl(&ctx, &mm, request_params)
        .await
        .unwrap();

    let respond_params = RespondContactByNameParams {
        project_slug: project_slug.clone(),
        to_agent: bob.clone(),
        from_agent: alice.clone(),
        accept: false,
    };

    let result = contacts::respond_contact_by_name_impl(&ctx, &mm, respond_params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("rejected"));
}

#[tokio::test]
async fn test_respond_contact_by_name_impl_no_pending_request() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, alice, bob) =
        setup_single_project_with_two_agents(&mm, "byname-nopending").await;

    let respond_params = RespondContactByNameParams {
        project_slug,
        to_agent: bob,
        from_agent: alice,
        accept: true,
    };

    let result = contacts::respond_contact_by_name_impl(&ctx, &mm, respond_params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("No pending contact request"));
}

#[tokio::test]
async fn test_respond_contact_by_name_impl_agent_not_found() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "byname-notfound";
    ProjectBmc::create(&ctx, &mm, project_slug, "ByName NotFound Project")
        .await
        .unwrap();

    let respond_params = RespondContactByNameParams {
        project_slug: project_slug.to_string(),
        to_agent: "nonexistent_to".to_string(),
        from_agent: "nonexistent_from".to_string(),
        accept: true,
    };

    let result = contacts::respond_contact_by_name_impl(&ctx, &mm, respond_params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("not found"));
}
