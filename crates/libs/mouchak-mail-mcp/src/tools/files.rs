//! File reservation tool implementations
//!
//! Handles file path reservations to prevent conflicts between agents.

use mouchak_mail_core::{
    ctx::Ctx,
    model::{
        ModelManager,
        agent::AgentBmc,
        agent_capabilities::AgentCapabilityBmc,
        file_reservation::{FileReservationBmc, FileReservationForCreate},
    },
    utils::validation::{
        validate_agent_name, validate_project_key, validate_reservation_path, validate_ttl,
    },
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{
    FileReservationParams, FileReservationPathsParams, ForceReleaseReservationParams,
    ListReservationsParams, ReleaseFileReservationsByAgentParams, ReleaseReservationParams,
    RenewFileReservationParams, RenewFileReservationsByAgentParams,
};

/// Reserve a file path pattern to prevent conflicts between agents.
pub async fn reserve_file_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: FileReservationParams,
) -> Result<CallToolResult, McpError> {
    // Validate inputs
    validate_project_key(&params.project_slug).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    validate_agent_name(&params.agent_name).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    validate_reservation_path(&params.path_pattern).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    if let Some(ttl) = params.ttl_seconds {
        validate_ttl(ttl as u64).map_err(|e| {
            McpError::invalid_params(
                format!("{}", e),
                Some(serde_json::json!({ "details": e.context() })),
            )
        })?;
    }

    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    if !AgentCapabilityBmc::check(ctx, mm, agent.id.get(), "file_reservation_paths")
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
    {
        return Err(McpError::invalid_params(
            format!(
                "Agent '{}' does not have 'file_reservation_paths' capability",
                params.agent_name
            ),
            None,
        ));
    }

    let ttl = params.ttl_seconds.unwrap_or(3600);
    let expires_ts = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl);

    let res_c = FileReservationForCreate {
        project_id: project.id,
        agent_id: agent.id,
        path_pattern: params.path_pattern.clone(),
        exclusive: params.exclusive.unwrap_or(true),
        reason: params
            .reason
            .unwrap_or_else(|| "Reserved via MCP".to_string()),
        expires_ts,
    };

    let id = FileReservationBmc::create(ctx, mm, res_c)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Reserved '{}' for agent '{}' (reservation id: {}, expires: {})",
        params.path_pattern, params.agent_name, id, expires_ts
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// List all active file reservations in a project.
pub async fn list_reservations_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListReservationsParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let reservations = FileReservationBmc::list_active_for_project(ctx, mm, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!(
        "Active reservations in '{}' ({}):\n\n",
        params.project_slug,
        reservations.len()
    );
    for r in &reservations {
        output.push_str(&format!(
            "- [{}] {} (agent_id: {}, exclusive: {}, expires: {})\n",
            r.id, r.path_pattern, r.agent_id, r.exclusive, r.expires_ts
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Release a file reservation by ID.
pub async fn release_reservation_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ReleaseReservationParams,
) -> Result<CallToolResult, McpError> {
    FileReservationBmc::release(ctx, mm, params.reservation_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!("Released reservation {}", params.reservation_id);
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Force release a file reservation (emergency override).
pub async fn force_release_reservation_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ForceReleaseReservationParams,
) -> Result<CallToolResult, McpError> {
    FileReservationBmc::force_release(ctx, mm, params.reservation_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!("Force released reservation {}", params.reservation_id);
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Extend the TTL of a file reservation.
pub async fn renew_file_reservation_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: RenewFileReservationParams,
) -> Result<CallToolResult, McpError> {
    let ttl = params.ttl_seconds.unwrap_or(3600);
    let new_expires = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl);

    FileReservationBmc::renew(ctx, mm, params.reservation_id, new_expires)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Renewed reservation {} until {}",
        params.reservation_id, new_expires
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Reserve multiple file paths with conflict detection.
pub async fn file_reservation_paths_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: FileReservationPathsParams,
) -> Result<CallToolResult, McpError> {
    // Validate inputs
    validate_project_key(&params.project_slug).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    validate_agent_name(&params.agent_name).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    // Validate all paths
    for path in &params.paths {
        validate_reservation_path(path).map_err(|e| {
            McpError::invalid_params(
                format!("{}", e),
                Some(serde_json::json!({ "details": e.context() })),
            )
        })?;
    }

    if let Some(ttl) = params.ttl_seconds {
        validate_ttl(ttl as u64).map_err(|e| {
            McpError::invalid_params(
                format!("{}", e),
                Some(serde_json::json!({ "details": e.context() })),
            )
        })?;
    }

    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let active_reservations = FileReservationBmc::list_active_for_project(ctx, mm, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let ttl = params.ttl_seconds.unwrap_or(3600);
    let now = chrono::Utc::now().naive_utc();
    let expires_ts = now + chrono::Duration::seconds(ttl);

    let mut granted = Vec::new();
    let mut conflicts = Vec::new();

    for path in params.paths {
        // Check conflicts using glob pattern matching
        for res in &active_reservations {
            if res.agent_id != agent.id
                && (res.exclusive || params.exclusive)
                && mouchak_mail_core::utils::pathspec::paths_conflict(&res.path_pattern, &path)
            {
                conflicts.push(format!(
                    "Conflict: {} overlaps with {} (held by agent ID {}, expires: {})",
                    path, res.path_pattern, res.agent_id, res.expires_ts
                ));
            }
        }

        // Always grant (advisory model)
        let fr_c = FileReservationForCreate {
            project_id: project.id,
            agent_id: agent.id,
            path_pattern: path.clone(),
            exclusive: params.exclusive,
            reason: params.reason.clone().unwrap_or_default(),
            expires_ts,
        };

        let id = FileReservationBmc::create(ctx, mm, fr_c)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        granted.push(format!(
            "Granted: {} (id: {}, expires: {})",
            path, id, expires_ts
        ));
    }

    let mut output = format!("Granted {} reservations\n\n", granted.len());
    for g in granted {
        output.push_str(&format!("  {}\n", g));
    }

    if !conflicts.is_empty() {
        output.push_str(&format!("\n⚠️ {} conflicts detected:\n", conflicts.len()));
        for c in conflicts {
            output.push_str(&format!("  {}\n", c));
        }
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn release_file_reservations_by_path_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ReleaseFileReservationsByAgentParams,
) -> Result<CallToolResult, McpError> {
    validate_project_key(&params.project_slug).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    validate_agent_name(&params.agent_name).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let mut released_ids = Vec::new();

    if let Some(paths) = &params.paths {
        for path in paths {
            if let Some(id) =
                FileReservationBmc::release_by_path(ctx, mm, project.id.get(), agent.id.get(), path)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            {
                released_ids.push(id);
            }
        }
    }

    if let Some(ids) = &params.file_reservation_ids {
        for id in ids {
            FileReservationBmc::release(ctx, mm, *id)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            released_ids.push(*id);
        }
    }

    let output = serde_json::json!({
        "released_count": released_ids.len(),
        "released_ids": released_ids,
        "agent_name": params.agent_name,
        "project_slug": params.project_slug
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&output).unwrap_or_default(),
    )]))
}

pub async fn renew_file_reservations_by_agent_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: RenewFileReservationsByAgentParams,
) -> Result<CallToolResult, McpError> {
    validate_project_key(&params.project_slug).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    validate_agent_name(&params.agent_name).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let all_reservations = FileReservationBmc::list_active_for_project(ctx, mm, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let agent_reservations: Vec<_> = all_reservations
        .into_iter()
        .filter(|r| r.agent_id == agent.id)
        .collect();

    let reservations_to_renew: Vec<_> = if let Some(paths) = &params.paths {
        agent_reservations
            .into_iter()
            .filter(|r| paths.contains(&r.path_pattern))
            .collect()
    } else {
        agent_reservations
    };

    let ttl = params.extend_seconds.unwrap_or(3600);
    let new_expires = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl);

    let mut renewed_ids = Vec::new();
    for res in &reservations_to_renew {
        FileReservationBmc::renew(ctx, mm, res.id, new_expires)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        renewed_ids.push(res.id);
    }

    let output = serde_json::json!({
        "renewed_count": renewed_ids.len(),
        "renewed_ids": renewed_ids,
        "new_expires_ts": new_expires.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "agent_name": params.agent_name,
        "project_slug": params.project_slug
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&output).unwrap_or_default(),
    )]))
}
