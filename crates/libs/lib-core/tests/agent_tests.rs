//! Agent model tests
//!
//! Tests for agent registration, lookup, and management.

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::utils::slugify;

/// Helper to create a test project
async fn create_test_project(tc: &TestContext, name: &str) -> i64 {
    let human_key = format!("/test/agents/{}", name);
    let slug = slugify(&human_key);

    // Check if project exists first
    if let Ok(project) = ProjectBmc::get_by_slug(&tc.ctx, &tc.mm, &slug).await {
        return project.id;
    }

    ProjectBmc::create(&tc.ctx, &tc.mm, &slug, &human_key)
        .await
        .expect("Failed to create project")
}

/// Test registering a new agent
#[tokio::test]
async fn test_register_agent() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "register").await;

    let agent_c = AgentForCreate {
        project_id,
        name: "TestAgent".to_string(),
        program: "antigravity".to_string(),
        model: "gemini-2.0-pro".to_string(),
        task_description: "Testing agent".to_string(),
    };

    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_c)
        .await
        .expect("Failed to create agent");

    assert!(agent_id > 0, "Agent should have valid ID");
}

/// Test agent lookup by ID
#[tokio::test]
async fn test_get_agent_by_id() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "get-by-id").await;

    let agent_c = AgentForCreate {
        project_id,
        name: "GetTestAgent".to_string(),
        program: "test-program".to_string(),
        model: "test-model".to_string(),
        task_description: "Get test".to_string(),
    };

    let created_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();

    let agent = AgentBmc::get(&tc.ctx, &tc.mm, created_id)
        .await
        .expect("Should find agent by ID");

    assert_eq!(agent.id, created_id);
    assert_eq!(agent.name, "GetTestAgent");
}

/// Test agent lookup by name
#[tokio::test]
async fn test_get_agent_by_name() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "get-by-name").await;

    let agent_c = AgentForCreate {
        project_id,
        name: "BlueOcean".to_string(),
        program: "gemini".to_string(),
        model: "gemini-2.0-flash".to_string(),
        task_description: "Lookup test".to_string(),
    };

    let created_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();

    let found = AgentBmc::get_by_name(&tc.ctx, &tc.mm, project_id, "BlueOcean")
        .await
        .expect("Should find agent by name");

    assert_eq!(created_id, found.id);
    assert_eq!(found.name, "BlueOcean");
}

/// Test listing agents in a project
#[tokio::test]
async fn test_list_agents_for_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "list-agents").await;

    // Create multiple agents
    for name in &["AgentAlpha", "AgentBeta", "AgentGamma"] {
        let agent_c = AgentForCreate {
            project_id,
            name: (*name).to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: format!("Agent {}", name),
        };
        AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();
    }

    let agents = AgentBmc::list_all_for_project(&tc.ctx, &tc.mm, project_id)
        .await
        .expect("Failed to list agents");

    assert_eq!(agents.len(), 3, "Should have 3 agents");
}

/// Test agent not found error
#[tokio::test]
async fn test_agent_not_found() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let result = AgentBmc::get(&tc.ctx, &tc.mm, 99999).await;

    assert!(result.is_err(), "Should return error for nonexistent agent");
}

#[tokio::test]
async fn test_delete_agent_cascade() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "delete-agent").await;

    let agent = AgentForCreate {
        project_id,
        name: "ToDelete".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Will be deleted".to_string(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent)
        .await
        .expect("Failed to create agent");

    let msg = lib_core::model::message::MessageForCreate {
        project_id,
        sender_id: agent_id,
        recipient_ids: vec![agent_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "From doomed agent".into(),
        body_md: "Will be deleted with agent".into(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    lib_core::model::message::MessageBmc::create(&tc.ctx, &tc.mm, msg)
        .await
        .expect("Failed to create message");

    AgentBmc::delete(&tc.ctx, &tc.mm, agent_id)
        .await
        .expect("Failed to delete agent");

    let result = AgentBmc::get(&tc.ctx, &tc.mm, agent_id).await;
    assert!(result.is_err(), "Agent should not exist after deletion");

    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await;
    assert!(
        project.is_ok(),
        "Project should still exist after agent deletion"
    );
}

#[tokio::test]
async fn test_delete_nonexistent_agent() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let result = AgentBmc::delete(&tc.ctx, &tc.mm, 99999).await;
    assert!(result.is_err(), "Deleting nonexistent agent should fail");
}
