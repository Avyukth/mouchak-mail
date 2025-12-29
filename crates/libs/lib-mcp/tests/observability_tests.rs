#![allow(clippy::unwrap_used, clippy::expect_used)]

use lib_common::config::AppConfig;
use lib_core::ctx::Ctx;
use lib_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    agent_capabilities::{AgentCapabilityBmc, AgentCapabilityForCreate},
    message::{MessageBmc, MessageForCreate},
    project::ProjectBmc,
    tool_metric::{ToolMetricBmc, ToolMetricForCreate},
};
use lib_mcp::tools::observability;
use lib_mcp::tools::{ListActivityParams, ListPendingReviewsParams, ListToolMetricsParams};
use libsql::Builder;
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
    let db_path = temp_dir.path().join("test_observability.db");
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

#[tokio::test]
async fn test_list_tool_metrics_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListToolMetricsParams {
        project_id: None,
        limit: Some(10),
    };

    let result = observability::list_tool_metrics_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(
        output.contains("[]"),
        "Empty metrics should return empty array"
    );
}

#[tokio::test]
async fn test_list_tool_metrics_impl_with_data() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "metrics-project", "Metrics Test Project")
        .await
        .unwrap();

    let metric = ToolMetricForCreate {
        project_id: Some(project_id.into()),
        agent_id: None,
        tool_name: "send_message".to_string(),
        args_json: None,
        status: "success".to_string(),
        error_code: None,
        duration_ms: 42,
    };
    ToolMetricBmc::create(&ctx, &mm, metric).await.unwrap();

    let params = ListToolMetricsParams {
        project_id: Some(project_id.into()),
        limit: Some(10),
    };

    let result = observability::list_tool_metrics_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("send_message"));
}

#[tokio::test]
async fn test_get_tool_stats_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListToolMetricsParams {
        project_id: None,
        limit: None,
    };

    let result = observability::get_tool_stats_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_tool_stats_impl_with_data() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "stats-project", "Stats Test Project")
        .await
        .unwrap();

    for i in 0..5 {
        let metric = ToolMetricForCreate {
            project_id: Some(project_id.into()),
            agent_id: None,
            tool_name: "fetch_inbox".to_string(),
            args_json: None,
            status: if i % 2 == 0 {
                "success".to_string()
            } else {
                "error".to_string()
            },
            error_code: if i % 2 != 0 {
                Some("TEST_ERROR".to_string())
            } else {
                None
            },
            duration_ms: 10 + i,
        };
        ToolMetricBmc::create(&ctx, &mm, metric).await.unwrap();
    }

    let params = ListToolMetricsParams {
        project_id: Some(project_id.into()),
        limit: None,
    };

    let result = observability::get_tool_stats_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_list_activity_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "activity-empty", "Activity Empty Project")
        .await
        .unwrap();

    let params = ListActivityParams {
        project_id: project_id.into(),
        limit: Some(10),
    };

    let result = observability::list_activity_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(
        output.contains("[]"),
        "Empty activity should return empty array"
    );
}

#[tokio::test]
async fn test_list_activity_impl_with_data() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_id = ProjectBmc::create(&ctx, &mm, "activity-project", "Activity Test Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "activity_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent for activity test".to_string(),
    };
    let agent_id = AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let metric = ToolMetricForCreate {
        project_id: Some(project_id.into()),
        agent_id: Some(agent_id.into()),
        tool_name: "activity_test_tool".to_string(),
        args_json: Some(r#"{"key": "value"}"#.to_string()),
        status: "success".to_string(),
        error_code: None,
        duration_ms: 100,
    };
    ToolMetricBmc::create(&ctx, &mm, metric).await.unwrap();

    let params = ListActivityParams {
        project_id: project_id.into(),
        limit: Some(10),
    };

    let result = observability::list_activity_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_list_pending_reviews_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ListPendingReviewsParams {
        project_slug: None,
        sender_name: None,
        limit: Some(10),
    };

    let result = observability::list_pending_reviews_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(
        output.contains("[]"),
        "No pending reviews should return empty array"
    );
}

#[tokio::test]
async fn test_list_pending_reviews_impl_with_ack_required_message() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "reviews-project";
    let project_id = ProjectBmc::create(&ctx, &mm, project_slug, "Reviews Test Project")
        .await
        .unwrap();

    let sender_c = AgentForCreate {
        project_id,
        name: "sender".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Sender".to_string(),
    };
    let sender_id = AgentBmc::create(&ctx, &mm, sender_c).await.unwrap();

    let cap = AgentCapabilityForCreate {
        agent_id: sender_id.into(),
        capability: "send_message".to_string(),
        granted_by: None,
        expires_at: None,
    };
    AgentCapabilityBmc::create(&ctx, &mm, cap).await.unwrap();

    let receiver_c = AgentForCreate {
        project_id,
        name: "reviewer".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Reviewer".to_string(),
    };
    let receiver_id = AgentBmc::create(&ctx, &mm, receiver_c).await.unwrap();

    let msg = MessageForCreate {
        project_id: project_id.into(),
        sender_id: sender_id.into(),
        recipient_ids: vec![receiver_id.into()],
        cc_ids: None,
        bcc_ids: None,
        subject: "Review needed".to_string(),
        body_md: "Please review this".to_string(),
        thread_id: None,
        importance: Some("high".to_string()),
        ack_required: true,
    };
    MessageBmc::create(&ctx, &mm, msg).await.unwrap();

    let params = ListPendingReviewsParams {
        project_slug: Some(project_slug.to_string()),
        sender_name: None,
        limit: Some(10),
    };

    let result = observability::list_pending_reviews_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(
        output.contains("Review needed") || output.contains("sender"),
        "Should list pending review message: {}",
        output
    );
}

#[tokio::test]
async fn test_list_pending_reviews_impl_with_sender_filter() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "sender-filter-project";
    let project_id = ProjectBmc::create(&ctx, &mm, project_slug, "Sender Filter Project")
        .await
        .unwrap();

    let sender_c = AgentForCreate {
        project_id,
        name: "specific_sender".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Specific sender".to_string(),
    };
    let sender_id = AgentBmc::create(&ctx, &mm, sender_c).await.unwrap();

    let cap = AgentCapabilityForCreate {
        agent_id: sender_id.into(),
        capability: "send_message".to_string(),
        granted_by: None,
        expires_at: None,
    };
    AgentCapabilityBmc::create(&ctx, &mm, cap).await.unwrap();

    let receiver_c = AgentForCreate {
        project_id,
        name: "receiver".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Receiver".to_string(),
    };
    let receiver_id = AgentBmc::create(&ctx, &mm, receiver_c).await.unwrap();

    let msg = MessageForCreate {
        project_id: project_id.into(),
        sender_id: sender_id.into(),
        recipient_ids: vec![receiver_id.into()],
        cc_ids: None,
        bcc_ids: None,
        subject: "Filtered review".to_string(),
        body_md: "Filter test".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    MessageBmc::create(&ctx, &mm, msg).await.unwrap();

    let params = ListPendingReviewsParams {
        project_slug: Some(project_slug.to_string()),
        sender_name: Some("specific_sender".to_string()),
        limit: Some(10),
    };

    let result = observability::list_pending_reviews_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}
