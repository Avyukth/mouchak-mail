//! Agent link model tests
//!
//! Tests for contact request/response flow between agents.

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::agent_link::{AgentLinkBmc, AgentLinkForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::utils::slugify;

/// Helper to set up a project with two agents
async fn setup_project_with_agents(tc: &TestContext, suffix: &str) -> (i64, i64, i64) {
    let human_key = format!("/test/link-repo-{}", suffix);
    let slug = slugify(&human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, &human_key)
        .await
        .expect("Failed to create project");

    let agent_a = AgentForCreate {
        project_id,
        name: format!("agent-a-{}", suffix),
        program: "claude-code".to_string(),
        model: "claude-3".to_string(),
        task_description: "Agent A for link tests".to_string(),
    };
    let agent_a_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_a)
        .await
        .expect("Failed to create agent A");

    let agent_b = AgentForCreate {
        project_id,
        name: format!("agent-b-{}", suffix),
        program: "cursor".to_string(),
        model: "gpt-4".to_string(),
        task_description: "Agent B for link tests".to_string(),
    };
    let agent_b_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_b)
        .await
        .expect("Failed to create agent B");

    (project_id, agent_a_id, agent_b_id)
}

/// Test requesting contact between agents
#[tokio::test]
async fn test_request_contact() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_a_id, agent_b_id) = setup_project_with_agents(&tc, "request").await;

    let link_c = AgentLinkForCreate {
        a_project_id: project_id,
        a_agent_id: agent_a_id,
        b_project_id: project_id,
        b_agent_id: agent_b_id,
        reason: "Collaboration on feature X".to_string(),
    };

    let link_id = AgentLinkBmc::request_contact(&tc.ctx, &tc.mm, link_c)
        .await
        .expect("Failed to request contact");

    assert!(link_id > 0, "Link ID should be positive");
}

/// Test accepting a contact request
#[tokio::test]
async fn test_accept_contact() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_a_id, agent_b_id) = setup_project_with_agents(&tc, "accept").await;

    // Agent A requests contact with Agent B
    let link_c = AgentLinkForCreate {
        a_project_id: project_id,
        a_agent_id: agent_a_id,
        b_project_id: project_id,
        b_agent_id: agent_b_id,
        reason: "Working together".to_string(),
    };

    let link_id = AgentLinkBmc::request_contact(&tc.ctx, &tc.mm, link_c)
        .await
        .expect("Failed to request contact");

    // Agent B accepts
    AgentLinkBmc::respond_contact(&tc.ctx, &tc.mm, link_id, true)
        .await
        .expect("Failed to accept contact");

    // Verify A can see B in contacts
    let contacts = AgentLinkBmc::list_contacts(&tc.ctx, &tc.mm, project_id, agent_a_id)
        .await
        .expect("Failed to list contacts");

    assert_eq!(contacts.len(), 1, "Agent A should have 1 contact");
    assert_eq!(contacts[0].status, "accepted");
}

/// Test rejecting a contact request
#[tokio::test]
async fn test_reject_contact() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_a_id, agent_b_id) = setup_project_with_agents(&tc, "reject").await;

    // Agent A requests contact
    let link_c = AgentLinkForCreate {
        a_project_id: project_id,
        a_agent_id: agent_a_id,
        b_project_id: project_id,
        b_agent_id: agent_b_id,
        reason: "Collaboration request".to_string(),
    };

    let link_id = AgentLinkBmc::request_contact(&tc.ctx, &tc.mm, link_c)
        .await
        .expect("Failed to request contact");

    // Agent B rejects
    AgentLinkBmc::respond_contact(&tc.ctx, &tc.mm, link_id, false)
        .await
        .expect("Failed to reject contact");

    // Verify A has no contacts (rejected links don't show in contacts)
    let contacts = AgentLinkBmc::list_contacts(&tc.ctx, &tc.mm, project_id, agent_a_id)
        .await
        .expect("Failed to list contacts");

    assert!(
        contacts.is_empty(),
        "Rejected requests should not appear in contacts"
    );
}

/// Test listing pending contact requests
#[tokio::test]
async fn test_list_pending_requests() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_a_id, agent_b_id) = setup_project_with_agents(&tc, "pending").await;

    // Agent A requests contact with Agent B
    let link_c = AgentLinkForCreate {
        a_project_id: project_id,
        a_agent_id: agent_a_id,
        b_project_id: project_id,
        b_agent_id: agent_b_id,
        reason: "Want to collaborate".to_string(),
    };

    AgentLinkBmc::request_contact(&tc.ctx, &tc.mm, link_c)
        .await
        .expect("Failed to request contact");

    // Agent B checks pending requests
    let pending = AgentLinkBmc::list_pending_requests(&tc.ctx, &tc.mm, project_id, agent_b_id)
        .await
        .expect("Failed to list pending requests");

    assert_eq!(pending.len(), 1, "Agent B should have 1 pending request");
    assert_eq!(pending[0].status, "pending");
    assert_eq!(pending[0].a_agent_id, agent_a_id);
}

/// Test contacts are visible from both sides
#[tokio::test]
async fn test_bidirectional_contacts() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_a_id, agent_b_id) =
        setup_project_with_agents(&tc, "bidirectional").await;

    // Create and accept contact
    let link_c = AgentLinkForCreate {
        a_project_id: project_id,
        a_agent_id: agent_a_id,
        b_project_id: project_id,
        b_agent_id: agent_b_id,
        reason: "Mutual collaboration".to_string(),
    };

    let link_id = AgentLinkBmc::request_contact(&tc.ctx, &tc.mm, link_c)
        .await
        .expect("Failed to request contact");

    AgentLinkBmc::respond_contact(&tc.ctx, &tc.mm, link_id, true)
        .await
        .expect("Failed to accept contact");

    // Both agents should see the contact
    let a_contacts = AgentLinkBmc::list_contacts(&tc.ctx, &tc.mm, project_id, agent_a_id)
        .await
        .expect("Failed to list A's contacts");

    let b_contacts = AgentLinkBmc::list_contacts(&tc.ctx, &tc.mm, project_id, agent_b_id)
        .await
        .expect("Failed to list B's contacts");

    assert_eq!(a_contacts.len(), 1, "Agent A should see the contact");
    assert_eq!(b_contacts.len(), 1, "Agent B should see the contact");
}

/// Test multiple pending requests
#[tokio::test]
async fn test_multiple_pending_requests() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/test/multi-request-repo";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    // Create target agent
    let target = AgentForCreate {
        project_id,
        name: "target-agent".to_string(),
        program: "claude-code".to_string(),
        model: "claude-3".to_string(),
        task_description: "Target agent".to_string(),
    };
    let target_id = AgentBmc::create(&tc.ctx, &tc.mm, target)
        .await
        .expect("Failed to create target agent");

    // Create multiple requesters
    for i in 1..=3 {
        let requester = AgentForCreate {
            project_id,
            name: format!("requester-{}", i),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: format!("Requester {}", i),
        };
        let requester_id = AgentBmc::create(&tc.ctx, &tc.mm, requester)
            .await
            .expect("Failed to create requester");

        let link_c = AgentLinkForCreate {
            a_project_id: project_id,
            a_agent_id: requester_id,
            b_project_id: project_id,
            b_agent_id: target_id,
            reason: format!("Request from agent {}", i),
        };
        AgentLinkBmc::request_contact(&tc.ctx, &tc.mm, link_c)
            .await
            .expect("Failed to request contact");
    }

    // Target should have 3 pending requests
    let pending = AgentLinkBmc::list_pending_requests(&tc.ctx, &tc.mm, project_id, target_id)
        .await
        .expect("Failed to list pending requests");

    assert_eq!(pending.len(), 3, "Target should have 3 pending requests");
}

/// Test empty contacts for new agent
#[tokio::test]
async fn test_empty_contacts() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_a_id, _) = setup_project_with_agents(&tc, "empty").await;

    let contacts = AgentLinkBmc::list_contacts(&tc.ctx, &tc.mm, project_id, agent_a_id)
        .await
        .expect("Failed to list contacts");

    assert!(contacts.is_empty(), "New agent should have no contacts");
}
