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
use lib_core::{AgentId, ProjectId};

/// Helper to create a test project
async fn create_test_project(tc: &TestContext, name: &str) -> ProjectId {
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

    assert!(agent_id.get() > 0, "Agent should have valid ID");
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

    let result = AgentBmc::get(&tc.ctx, &tc.mm, AgentId(99999)).await;

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
        project_id: project_id.get(),
        sender_id: agent_id.get(),
        recipient_ids: vec![agent_id.get()],
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

    let result = AgentBmc::delete(&tc.ctx, &tc.mm, AgentId(99999)).await;
    assert!(result.is_err(), "Deleting nonexistent agent should fail");
}

#[tokio::test]
async fn test_check_reviewer_exists_found() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "reviewer-exists").await;

    let agent_c = AgentForCreate {
        project_id,
        name: "reviewer".to_string(),
        program: "claude-code".to_string(),
        model: "claude-opus-4".to_string(),
        task_description: "Reviewer agent".to_string(),
    };
    AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();

    let result = AgentBmc::check_reviewer_exists(&tc.ctx, &tc.mm, project_id, None)
        .await
        .expect("check_reviewer_exists should not error");

    assert!(result.is_some(), "Reviewer should be found");
    assert_eq!(result.unwrap().name.to_lowercase(), "reviewer");
}

#[tokio::test]
async fn test_check_reviewer_exists_not_found() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "no-reviewer").await;

    let result = AgentBmc::check_reviewer_exists(&tc.ctx, &tc.mm, project_id, None)
        .await
        .expect("check_reviewer_exists should not error");

    assert!(result.is_none(), "Reviewer should not be found");
}

#[tokio::test]
async fn test_check_reviewer_exists_stale() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "stale-reviewer").await;

    let agent_c = AgentForCreate {
        project_id,
        name: "reviewer".to_string(),
        program: "claude-code".to_string(),
        model: "claude-opus-4".to_string(),
        task_description: "Stale reviewer".to_string(),
    };
    AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();

    let result = AgentBmc::check_reviewer_exists(
        &tc.ctx,
        &tc.mm,
        project_id,
        Some(std::time::Duration::from_secs(0)),
    )
    .await
    .expect("check_reviewer_exists should not error");

    assert!(
        result.is_none(),
        "Stale reviewer should be treated as non-existent"
    );
}

#[tokio::test]
async fn test_count_messages_sent() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "count-sent").await;

    let agent = AgentForCreate {
        project_id,
        name: "SenderAgent".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender".to_string(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent).await.unwrap();

    let count_before = AgentBmc::count_messages_sent(&tc.ctx, &tc.mm, agent_id)
        .await
        .unwrap();
    assert_eq!(count_before, 0);

    let msg = lib_core::model::message::MessageForCreate {
        project_id: project_id.get(),
        sender_id: agent_id.get(),
        recipient_ids: vec![agent_id.get()],
        cc_ids: None,
        bcc_ids: None,
        subject: "Test message".into(),
        body_md: "Body".into(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    lib_core::model::message::MessageBmc::create(&tc.ctx, &tc.mm, msg)
        .await
        .unwrap();

    let count_after = AgentBmc::count_messages_sent(&tc.ctx, &tc.mm, agent_id)
        .await
        .unwrap();
    assert_eq!(count_after, 1);
}

#[tokio::test]
async fn test_count_messages_received() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "count-received").await;

    let sender = AgentForCreate {
        project_id,
        name: "MsgSender".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Sender".to_string(),
    };
    let sender_id = AgentBmc::create(&tc.ctx, &tc.mm, sender).await.unwrap();

    let receiver = AgentForCreate {
        project_id,
        name: "MsgReceiver".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Receiver".to_string(),
    };
    let receiver_id = AgentBmc::create(&tc.ctx, &tc.mm, receiver).await.unwrap();

    let count_before = AgentBmc::count_messages_received(&tc.ctx, &tc.mm, receiver_id)
        .await
        .unwrap();
    assert_eq!(count_before, 0);

    let msg = lib_core::model::message::MessageForCreate {
        project_id: project_id.get(),
        sender_id: sender_id.get(),
        recipient_ids: vec![receiver_id.get()],
        cc_ids: None,
        bcc_ids: None,
        subject: "For receiver".into(),
        body_md: "Body".into(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    lib_core::model::message::MessageBmc::create(&tc.ctx, &tc.mm, msg)
        .await
        .unwrap();

    let count_after = AgentBmc::count_messages_received(&tc.ctx, &tc.mm, receiver_id)
        .await
        .unwrap();
    assert_eq!(count_after, 1);
}

#[tokio::test]
async fn test_update_profile() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "update-profile").await;

    let agent = AgentForCreate {
        project_id,
        name: "UpdateMe".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Original task".to_string(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent).await.unwrap();

    let update = lib_core::model::agent::AgentProfileUpdate {
        task_description: Some("Updated task description".to_string()),
        attachments_policy: Some("reject".to_string()),
        contact_policy: Some("manual".to_string()),
    };
    AgentBmc::update_profile(&tc.ctx, &tc.mm, agent_id, update)
        .await
        .expect("update_profile should succeed");

    let updated = AgentBmc::get(&tc.ctx, &tc.mm, agent_id).await.unwrap();
    assert_eq!(updated.task_description, "Updated task description");
    assert_eq!(updated.attachments_policy, "reject");
    assert_eq!(updated.contact_policy, "manual");
}

#[tokio::test]
async fn test_update_profile_partial() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "partial-update").await;

    let agent = AgentForCreate {
        project_id,
        name: "PartialUpdate".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Original".to_string(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent).await.unwrap();

    let original = AgentBmc::get(&tc.ctx, &tc.mm, agent_id).await.unwrap();
    let original_policy = original.attachments_policy.clone();

    let update = lib_core::model::agent::AgentProfileUpdate {
        task_description: Some("Only task updated".to_string()),
        attachments_policy: None,
        contact_policy: None,
    };
    AgentBmc::update_profile(&tc.ctx, &tc.mm, agent_id, update)
        .await
        .unwrap();

    let updated = AgentBmc::get(&tc.ctx, &tc.mm, agent_id).await.unwrap();
    assert_eq!(updated.task_description, "Only task updated");
    assert_eq!(
        updated.attachments_policy, original_policy,
        "Attachments policy should be unchanged"
    );
}

#[tokio::test]
async fn test_get_by_name_with_suggestions() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = create_test_project(&tc, "suggestions").await;

    for name in &["BlueMountain", "BlueOcean", "GreenForest"] {
        let agent = AgentForCreate {
            project_id,
            name: (*name).to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "Test".to_string(),
        };
        AgentBmc::create(&tc.ctx, &tc.mm, agent).await.unwrap();
    }

    let result = AgentBmc::get_by_name(&tc.ctx, &tc.mm, project_id, "BluMountain").await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("BlueMountain") || err_str.contains("suggestions"),
        "Error should contain suggestions"
    );
}
