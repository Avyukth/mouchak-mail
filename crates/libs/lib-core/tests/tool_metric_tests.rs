//! Tool metric model tests
//!
//! Tests for tool usage tracking and statistics.

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
use lib_core::model::tool_metric::{ToolMetricBmc, ToolMetricForCreate};
use lib_core::types::ProjectId;
use lib_core::utils::slugify;

/// Helper to set up a project for tool metric tests
async fn setup_project(tc: &TestContext) -> i64 {
    let human_key = "/test/metrics-repo";
    let slug = slugify(human_key);

    ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project")
}

/// Test creating a tool metric
#[tokio::test]
async fn test_create_tool_metric() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    let metric_c = ToolMetricForCreate {
        project_id: Some(project_id),
        agent_id: None,
        tool_name: "send_message".to_string(),
        args_json: Some(r#"{"recipient": "agent-2"}"#.to_string()),
        status: "success".to_string(),
        error_code: None,
        duration_ms: 42,
    };

    let metric_id = ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
        .await
        .expect("Failed to create tool metric");

    assert!(metric_id > 0, "Metric ID should be positive");
}

/// Test creating a tool metric with error
#[tokio::test]
async fn test_create_tool_metric_with_error() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    let metric_c = ToolMetricForCreate {
        project_id: Some(project_id),
        agent_id: None,
        tool_name: "check_inbox".to_string(),
        args_json: None,
        status: "error".to_string(),
        error_code: Some("AGENT_NOT_FOUND".to_string()),
        duration_ms: 15,
    };

    let metric_id = ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
        .await
        .expect("Failed to create tool metric");

    assert!(metric_id > 0);
}

/// Test listing recent tool metrics for a project
#[tokio::test]
async fn test_list_recent_for_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    // Create multiple metrics
    for tool_name in &["ensure_project", "register_agent", "send_message"] {
        let metric_c = ToolMetricForCreate {
            project_id: Some(project_id),
            agent_id: None,
            tool_name: tool_name.to_string(),
            args_json: None,
            status: "success".to_string(),
            error_code: None,
            duration_ms: 10,
        };
        ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
            .await
            .expect("Failed to create metric");
    }

    let metrics = ToolMetricBmc::list_recent(&tc.ctx, &tc.mm, Some(project_id), 10)
        .await
        .expect("Failed to list metrics");

    assert_eq!(metrics.len(), 3, "Should have 3 metrics");
}

/// Test listing recent tool metrics with limit
#[tokio::test]
async fn test_list_recent_with_limit() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    // Create 5 metrics
    for i in 0..5 {
        let metric_c = ToolMetricForCreate {
            project_id: Some(project_id),
            agent_id: None,
            tool_name: format!("tool_{}", i),
            args_json: None,
            status: "success".to_string(),
            error_code: None,
            duration_ms: i as i64 * 10,
        };
        ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
            .await
            .expect("Failed to create metric");
    }

    // Limit to 3
    let metrics = ToolMetricBmc::list_recent(&tc.ctx, &tc.mm, Some(project_id), 3)
        .await
        .expect("Failed to list metrics");

    assert_eq!(
        metrics.len(),
        3,
        "Should return only 3 metrics due to limit"
    );
}

/// Test listing recent tool metrics globally (no project filter)
#[tokio::test]
async fn test_list_recent_global() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create metrics without project
    for tool_name in &["health", "ready", "version"] {
        let metric_c = ToolMetricForCreate {
            project_id: None,
            agent_id: None,
            tool_name: tool_name.to_string(),
            args_json: None,
            status: "success".to_string(),
            error_code: None,
            duration_ms: 5,
        };
        ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
            .await
            .expect("Failed to create metric");
    }

    // List globally
    let metrics = ToolMetricBmc::list_recent(&tc.ctx, &tc.mm, None, 10)
        .await
        .expect("Failed to list metrics globally");

    assert_eq!(metrics.len(), 3, "Should have 3 global metrics");
}

/// Test getting tool statistics for a project
#[tokio::test]
async fn test_get_stats_for_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    // Create metrics for different tools
    for _ in 0..3 {
        let metric_c = ToolMetricForCreate {
            project_id: Some(project_id),
            agent_id: None,
            tool_name: "send_message".to_string(),
            args_json: None,
            status: "success".to_string(),
            error_code: None,
            duration_ms: 100,
        };
        ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
            .await
            .expect("Failed to create metric");
    }

    for _ in 0..2 {
        let metric_c = ToolMetricForCreate {
            project_id: Some(project_id),
            agent_id: None,
            tool_name: "check_inbox".to_string(),
            args_json: None,
            status: "success".to_string(),
            error_code: None,
            duration_ms: 50,
        };
        ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
            .await
            .expect("Failed to create metric");
    }

    // One error
    let metric_c = ToolMetricForCreate {
        project_id: Some(project_id),
        agent_id: None,
        tool_name: "check_inbox".to_string(),
        args_json: None,
        status: "error".to_string(),
        error_code: Some("TIMEOUT".to_string()),
        duration_ms: 30000,
    };
    ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
        .await
        .expect("Failed to create metric");

    let stats = ToolMetricBmc::get_stats(&tc.ctx, &tc.mm, Some(project_id))
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.len(), 2, "Should have stats for 2 tools");

    // Find send_message stats
    let send_stats = stats
        .iter()
        .find(|s| s.tool_name == "send_message")
        .unwrap();
    assert_eq!(send_stats.count, 3);
    assert_eq!(send_stats.error_count, 0);

    // Find check_inbox stats
    let inbox_stats = stats.iter().find(|s| s.tool_name == "check_inbox").unwrap();
    assert_eq!(inbox_stats.count, 3); // 2 success + 1 error
    assert_eq!(inbox_stats.error_count, 1);
}

/// Test getting tool statistics globally
#[tokio::test]
async fn test_get_stats_global() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create metrics without project
    for _ in 0..5 {
        let metric_c = ToolMetricForCreate {
            project_id: None,
            agent_id: None,
            tool_name: "health".to_string(),
            args_json: None,
            status: "success".to_string(),
            error_code: None,
            duration_ms: 1,
        };
        ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
            .await
            .expect("Failed to create metric");
    }

    let stats = ToolMetricBmc::get_stats(&tc.ctx, &tc.mm, None)
        .await
        .expect("Failed to get global stats");

    assert!(!stats.is_empty(), "Should have stats");

    let health_stats = stats.iter().find(|s| s.tool_name == "health").unwrap();
    assert_eq!(health_stats.count, 5);
}

/// Test tool metric with agent association
#[tokio::test]
async fn test_tool_metric_with_agent() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    let agent = AgentForCreate {
        project_id: ProjectId(project_id),
        name: "metrics-agent".to_string(),
        program: "claude-code".to_string(),
        model: "claude-3".to_string(),
        task_description: "Testing tool metrics".to_string(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent)
        .await
        .expect("Failed to create agent");

    let metric_c = ToolMetricForCreate {
        project_id: Some(project_id),
        agent_id: Some(agent_id.into()),
        tool_name: "reserve_file".to_string(),
        args_json: Some(r#"{"path": "src/**"}"#.to_string()),
        status: "success".to_string(),
        error_code: None,
        duration_ms: 25,
    };

    let metric_id = ToolMetricBmc::create(&tc.ctx, &tc.mm, metric_c)
        .await
        .expect("Failed to create metric");

    let metrics = ToolMetricBmc::list_recent(&tc.ctx, &tc.mm, Some(project_id), 1)
        .await
        .expect("Failed to list metrics");

    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].id, metric_id);
    assert_eq!(metrics[0].agent_id, Some(agent_id.into()));
    assert_eq!(metrics[0].tool_name, "reserve_file");
}
