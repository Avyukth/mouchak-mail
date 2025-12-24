//! Observability tool implementations
//!
//! Handles tool metrics, activity tracking, and pending reviews listing.

use lib_core::{
    ctx::Ctx,
    model::{
        ModelManager, activity::ActivityBmc, agent::AgentBmc, message::MessageBmc,
        project::ProjectBmc, tool_metric::ToolMetricBmc,
    },
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::{ListActivityParams, ListPendingReviewsParams, ListToolMetricsParams};

/// List recent tool usage metrics for observability.
pub async fn list_tool_metrics_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListToolMetricsParams,
) -> Result<CallToolResult, McpError> {
    let limit = params.limit.unwrap_or(50);
    let metrics = ToolMetricBmc::list_recent(ctx, mm, params.project_id, limit)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let json_str = serde_json::to_string_pretty(&metrics)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(CallToolResult::success(vec![Content::text(json_str)]))
}

/// Get aggregated tool usage statistics.
pub async fn get_tool_stats_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListToolMetricsParams,
) -> Result<CallToolResult, McpError> {
    let stats = ToolMetricBmc::get_stats(ctx, mm, params.project_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let json_str = serde_json::to_string_pretty(&stats)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(CallToolResult::success(vec![Content::text(json_str)]))
}

/// List recent activity for a project.
pub async fn list_activity_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListActivityParams,
) -> Result<CallToolResult, McpError> {
    let limit = params.limit.unwrap_or(50);
    let items = ActivityBmc::list_recent(ctx, mm, params.project_id, limit)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let json_str = serde_json::to_string_pretty(&items)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(CallToolResult::success(vec![Content::text(json_str)]))
}

/// List messages requiring acknowledgment that haven't been fully acknowledged.
pub async fn list_pending_reviews_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListPendingReviewsParams,
) -> Result<CallToolResult, McpError> {
    // Resolve project_id from slug if provided
    let project_id = if let Some(ref slug) = params.project_slug {
        let project = ProjectBmc::get_by_identifier(ctx, mm, slug)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Some(project.id)
    } else {
        None
    };

    // Resolve sender_id from name if provided (requires project context)
    let sender_id = if let Some(ref sender_name) = params.sender_name {
        if let Some(pid) = project_id {
            let agent = AgentBmc::get_by_name(ctx, mm, pid, sender_name)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Some(agent.id)
        } else {
            None
        }
    } else {
        None
    };

    let limit = params.limit.unwrap_or(5);
    let rows = MessageBmc::list_pending_reviews(
        ctx,
        mm,
        project_id.map(|p| p.get()),
        sender_id.map(|s| s.get()),
        limit,
    )
    .await
    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let json_str = serde_json::to_string_pretty(&rows)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(CallToolResult::success(vec![Content::text(json_str)]))
}
