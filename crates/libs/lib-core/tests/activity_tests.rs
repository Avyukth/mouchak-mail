//! Activity feed model tests
//!
//! Tests for aggregated activity feed across messages, agents, and tool usage.

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::activity::ActivityBmc;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::model::tool_metric::{ToolMetricBmc, ToolMetricForCreate};
use lib_core::utils::slugify;

/// Helper to set up a project with agent
async fn setup_project_and_agent(tc: &TestContext) -> (i64, i64) {
    let human_key = "/test/activity-repo";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let agent = AgentForCreate {
        project_id,
        name: "activity-agent".to_string(),
        program: "claude-code".to_string(),
        model: "claude-3".to_string(),
        task_description: "Testing activity feed".to_string(),
    };

    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent)
        .await
        .expect("Failed to create agent");

    (project_id, agent_id)
}

/// Test listing recent activity with messages
#[tokio::test]
async fn test_list_activity_with_messages() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    // Create a second agent as recipient
    let agent2 = AgentForCreate {
        project_id,
        name: "recipient-agent".to_string(),
        program: "cursor".to_string(),
        model: "gpt-4".to_string(),
        task_description: "Receiving messages".to_string(),
    };
    let agent2_id = AgentBmc::create(&tc.ctx, &tc.mm, agent2)
        .await
        .expect("Failed to create agent 2");

    // Send a message
    let msg = MessageForCreate {
        project_id,
        sender_id: agent_id,
        recipient_ids: vec![agent2_id],
        cc_ids: None,
        bcc_ids: None,
        subject: "Test Activity Message".to_string(),
        body_md: "This is a test message for activity feed.".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    MessageBmc::create(&tc.ctx, &tc.mm, msg)
        .await
        .expect("Failed to create message");

    // List activity
    let activity = ActivityBmc::list_recent(&tc.ctx, &tc.mm, project_id, 10)
        .await
        .expect("Failed to list activity");

    // Should have: 2 agents (created) + 1 message
    assert!(activity.len() >= 1, "Should have at least 1 activity item");

    // Check message is in activity
    let has_message = activity.iter().any(|a| a.kind == "message");
    assert!(has_message, "Activity should include messages");
}

/// Test listing recent activity with tool metrics
#[tokio::test]
async fn test_list_activity_with_tool_metrics() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    // Create tool metrics
    for tool_name in &["send_message", "check_inbox", "reserve_file"] {
        let metric_c = ToolMetricForCreate {
            project_id: Some(project_id),
            agent_id: Some(agent_id),
            tool_name: tool_name.to_string(),
            args_json: None,
            status: "success".to_string(),
            error_code: None,
            duration_ms: 50,
        };
        ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
            .await
            .expect("Failed to create tool metric");
    }

    // List activity
    let activity = ActivityBmc::list_recent(&tc.ctx, &tc.mm, project_id, 10)
        .await
        .expect("Failed to list activity");

    // Check tool metrics are in activity
    let tool_count = activity.iter().filter(|a| a.kind == "tool").count();
    assert_eq!(tool_count, 3, "Should have 3 tool activities");
}

/// Test listing recent activity with agents
#[tokio::test]
async fn test_list_activity_with_agents() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/test/multi-agent-activity";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    // Create multiple agents
    for name in &["agent-alpha", "agent-beta", "agent-gamma"] {
        let agent = AgentForCreate {
            project_id,
            name: name.to_string(),
            program: "test-program".to_string(),
            model: "test-model".to_string(),
            task_description: format!("Testing agent {}", name),
        };
        AgentBmc::create(&tc.ctx, &tc.mm, agent)
            .await
            .expect("Failed to create agent");
    }

    // List activity
    let activity = ActivityBmc::list_recent(&tc.ctx, &tc.mm, project_id, 10)
        .await
        .expect("Failed to list activity");

    // Check agents are in activity
    let agent_count = activity.iter().filter(|a| a.kind == "agent").count();
    assert_eq!(agent_count, 3, "Should have 3 agent activities");
}

/// Test activity limit
#[tokio::test]
async fn test_list_activity_with_limit() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    // Create many tool metrics
    for i in 0..10 {
        let metric_c = ToolMetricForCreate {
            project_id: Some(project_id),
            agent_id: Some(agent_id),
            tool_name: format!("tool_{}", i),
            args_json: None,
            status: "success".to_string(),
            error_code: None,
            duration_ms: i as i64 * 10,
        };
        ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
            .await
            .expect("Failed to create tool metric");
    }

    // List with limit of 5
    let activity = ActivityBmc::list_recent(&tc.ctx, &tc.mm, project_id, 5)
        .await
        .expect("Failed to list activity");

    assert!(activity.len() <= 5, "Should respect limit");
}

/// Test activity item structure
#[tokio::test]
async fn test_activity_item_structure() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    // Create a tool metric with specific values
    let metric_c = ToolMetricForCreate {
        project_id: Some(project_id),
        agent_id: Some(agent_id),
        tool_name: "ensure_project".to_string(),
        args_json: Some(r#"{"project_key": "/test"}"#.to_string()),
        status: "success".to_string(),
        error_code: None,
        duration_ms: 123,
    };
    ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
        .await
        .expect("Failed to create tool metric");

    let activity = ActivityBmc::list_recent(&tc.ctx, &tc.mm, project_id, 10)
        .await
        .expect("Failed to list activity");

    // Find the tool activity
    let tool_activity = activity.iter().find(|a| a.kind == "tool");
    assert!(tool_activity.is_some(), "Should have tool activity");

    let tool = tool_activity.unwrap();
    assert!(
        tool.id.starts_with("tool:"),
        "Tool activity ID should have tool: prefix"
    );
    assert_eq!(tool.project_id, project_id);
    assert!(
        tool.title.contains("ensure_project"),
        "Title should contain tool name"
    );
    assert!(tool.description.is_some(), "Description should be present");
}

/// Test empty activity for new project
#[tokio::test]
async fn test_empty_activity() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/test/empty-activity-project";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    // No agents, messages, or metrics created

    let activity = ActivityBmc::list_recent(&tc.ctx, &tc.mm, project_id, 10)
        .await
        .expect("Failed to list activity");

    // Should be empty (no agents, no messages, no tools)
    assert!(activity.is_empty(), "New project should have no activity");
}

/// Test activity sorting by created_at
#[tokio::test]
async fn test_activity_sorted_by_time() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    // Create metrics with slight delays to ensure different timestamps
    for i in 0..3 {
        let metric_c = ToolMetricForCreate {
            project_id: Some(project_id),
            agent_id: Some(agent_id),
            tool_name: format!("tool_{}", i),
            args_json: None,
            status: "success".to_string(),
            error_code: None,
            duration_ms: 10,
        };
        ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
            .await
            .expect("Failed to create tool metric");
    }

    let activity = ActivityBmc::list_recent(&tc.ctx, &tc.mm, project_id, 10)
        .await
        .expect("Failed to list activity");

    // Verify activity is sorted by created_at descending
    for i in 0..activity.len().saturating_sub(1) {
        assert!(
            activity[i].created_at >= activity[i + 1].created_at,
            "Activity should be sorted by created_at descending"
        );
    }
}
