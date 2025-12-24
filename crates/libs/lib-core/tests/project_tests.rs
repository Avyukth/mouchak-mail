//! Project model tests
//!
//! Tests for project creation, retrieval, and management.

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
use lib_core::types::ProjectId;
use lib_core::utils::slugify;

/// Test creating a new project
#[tokio::test]
async fn test_create_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/test/project/path";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    assert!(project_id > 0, "Project ID should be positive");
}

/// Test getting project by slug
#[tokio::test]
async fn test_get_project_by_slug() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/my/cool/project";
    let slug = slugify(human_key);

    let _id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let retrieved = ProjectBmc::get_by_slug(&tc.ctx, &tc.mm, &slug)
        .await
        .expect("Failed to get project by slug");

    assert_eq!(retrieved.human_key, human_key);
    assert_eq!(retrieved.slug, slug);
}

/// Test getting project by human key
#[tokio::test]
async fn test_get_project_by_human_key() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/project/for/lookup";
    let slug = slugify(human_key);

    let _id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let retrieved = ProjectBmc::get_by_human_key(&tc.ctx, &tc.mm, human_key)
        .await
        .expect("Failed to get project by human key");

    assert_eq!(retrieved.human_key, human_key);
}

/// Test listing all projects
#[tokio::test]
async fn test_list_all_projects() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create multiple projects
    for name in &["alpha", "beta", "gamma"] {
        let human_key = format!("/project/{}", name);
        let slug = slugify(&human_key);
        ProjectBmc::create(&tc.ctx, &tc.mm, &slug, &human_key)
            .await
            .unwrap();
    }

    let projects = ProjectBmc::list_all(&tc.ctx, &tc.mm)
        .await
        .expect("Failed to list projects");

    assert_eq!(projects.len(), 3, "Should have 3 projects");
}

/// Test project not found error
#[tokio::test]
async fn test_project_not_found() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let result = ProjectBmc::get_by_slug(&tc.ctx, &tc.mm, "nonexistent-slug").await;

    assert!(
        result.is_err(),
        "Should return error for nonexistent project"
    );
}

/// Test adopting (consolidating) projects
#[tokio::test]
async fn test_adopt_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // 1. Create Source Project
    let src_key = "/src/project";
    let src_slug = slugify(src_key);
    let src_id = ProjectBmc::create(&tc.ctx, &tc.mm, &src_slug, src_key)
        .await
        .unwrap();

    // 2. Add an Agent to Source
    let agent_c = AgentForCreate {
        project_id: ProjectId(src_id),
        name: "test-agent".into(),
        program: "test".into(),
        model: "test".into(),
        task_description: "test".into(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();

    // 3. Add a Message to Source
    let msg_c = lib_core::model::message::MessageForCreate {
        project_id: src_id,
        sender_id: agent_id.into(),
        recipient_ids: vec![agent_id.into()],
        cc_ids: None,
        bcc_ids: None,
        subject: "Test adopt".into(),
        body_md: "Content".into(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    let msg_id = lib_core::model::message::MessageBmc::create(&tc.ctx, &tc.mm, msg_c)
        .await
        .unwrap();

    // 4. Create Dest Project
    let dest_key = "/dest/project";
    let dest_slug = slugify(dest_key);
    let dest_id = ProjectBmc::create(&tc.ctx, &tc.mm, &dest_slug, dest_key)
        .await
        .unwrap();

    // 5. Perform Adopt (src -> dest)
    ProjectBmc::adopt(&tc.ctx, &tc.mm, src_id, dest_id)
        .await
        .expect("Adopt failed");

    // 6. Verify Artifacts Moved
    // Check Agent
    let agent = AgentBmc::get(&tc.ctx, &tc.mm, agent_id).await.unwrap();
    assert_eq!(
        agent.project_id,
        ProjectId(dest_id),
        "Agent should be moved to dest project"
    );

    // Check Message
    let msg = lib_core::model::message::MessageBmc::get(&tc.ctx, &tc.mm, msg_id)
        .await
        .unwrap();
    assert_eq!(
        msg.project_id, dest_id,
        "Message should be moved to dest project"
    );
}

#[tokio::test]
async fn test_delete_project_cascade() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/project/to/delete";
    let slug = slugify(human_key);
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let agent = AgentForCreate {
        project_id: ProjectId(project_id),
        name: "doomed-agent".into(),
        program: "test".into(),
        model: "test".into(),
        task_description: "Will be deleted".into(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent)
        .await
        .expect("Failed to create agent");

    let msg = lib_core::model::message::MessageForCreate {
        project_id,
        sender_id: agent_id.into(),
        recipient_ids: vec![agent_id.into()],
        cc_ids: None,
        bcc_ids: None,
        subject: "Doomed message".into(),
        body_md: "Will be deleted".into(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    lib_core::model::message::MessageBmc::create(&tc.ctx, &tc.mm, msg)
        .await
        .expect("Failed to create message");

    ProjectBmc::delete(&tc.ctx, &tc.mm, project_id)
        .await
        .expect("Failed to delete project");

    let result = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await;
    assert!(result.is_err(), "Project should not exist after deletion");

    let result = AgentBmc::get(&tc.ctx, &tc.mm, agent_id).await;
    assert!(
        result.is_err(),
        "Agent should not exist after project deletion"
    );
}

#[tokio::test]
async fn test_delete_nonexistent_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let result = ProjectBmc::delete(&tc.ctx, &tc.mm, 99999).await;
    assert!(result.is_err(), "Deleting nonexistent project should fail");
}
