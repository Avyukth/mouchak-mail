//! Build slot tool implementations
//!
//! Handles CI/CD build slot acquisition, renewal, and release.

use lib_core::{
    ctx::Ctx,
    model::{
        ModelManager,
        agent::AgentBmc,
        build_slot::{BuildSlotBmc, BuildSlotForCreate},
    },
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{AcquireBuildSlotParams, ReleaseBuildSlotParams, RenewBuildSlotParams};

/// Acquire an exclusive build slot for CI/CD isolation.
pub async fn acquire_build_slot_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: AcquireBuildSlotParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let ttl = params.ttl_seconds.unwrap_or(1800);
    let slot_c = BuildSlotForCreate {
        project_id: project.id.get(),
        agent_id: agent.id.get(),
        slot_name: params.slot_name.clone(),
        ttl_seconds: ttl,
    };

    let slot_id = BuildSlotBmc::acquire(ctx, mm, slot_c)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let expires = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl);
    let msg = format!(
        "Acquired build slot '{}' (id: {}, expires: {})",
        params.slot_name, slot_id, expires
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Extend TTL on an active build slot.
pub async fn renew_build_slot_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: RenewBuildSlotParams,
) -> Result<CallToolResult, McpError> {
    let ttl = params.ttl_seconds.unwrap_or(1800);
    let new_expires = BuildSlotBmc::renew(ctx, mm, params.slot_id, ttl)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Renewed build slot {} (new expires: {})",
        params.slot_id, new_expires
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Release a held build slot.
pub async fn release_build_slot_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ReleaseBuildSlotParams,
) -> Result<CallToolResult, McpError> {
    BuildSlotBmc::release(ctx, mm, params.slot_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!("Released build slot {}", params.slot_id);
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}
