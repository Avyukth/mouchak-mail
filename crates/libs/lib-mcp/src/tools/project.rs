//! Project management tool implementations
//!
//! Handles project creation, listing, and info retrieval.

use lib_core::{
    ctx::Ctx,
    model::{ModelManager, agent::AgentBmc, project::ProjectBmc},
    utils::validation::validate_project_key,
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{EnsureProjectParams, GetProjectInfoParams};

/// Ensure a project exists (create if not).
pub async fn ensure_project_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: EnsureProjectParams,
) -> Result<CallToolResult, McpError> {
    // Validate project key
    validate_project_key(&params.slug).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    // Get or create project
    match ProjectBmc::get_by_identifier(ctx, mm, &params.slug).await {
        Ok(project) => {
            let msg = format!(
                "Project exists: {} (id: {}, created: {})",
                project.slug, project.id, project.created_at
            );
            Ok(CallToolResult::success(vec![Content::text(msg)]))
        }
        Err(_) => {
            let id = ProjectBmc::create(ctx, mm, &params.slug, &params.human_key)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let msg = format!("Created project '{}' with id {}", params.slug, id);
            Ok(CallToolResult::success(vec![Content::text(msg)]))
        }
    }
}

/// List all available projects.
pub async fn list_projects_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
) -> Result<CallToolResult, McpError> {
    let projects = ProjectBmc::list_all(ctx, mm)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!("Projects ({}):\n\n", projects.len());
    for p in &projects {
        output.push_str(&format!(
            "- {} (slug: {}, created: {})\n",
            p.human_key, p.slug, p.created_at
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Get detailed information about a project.
pub async fn get_project_info_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: GetProjectInfoParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agents = AgentBmc::list_all_for_project(ctx, mm, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let message_count = ProjectBmc::count_messages(ctx, mm, project.id.get())
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let output = format!(
        "Project: {} ({})\nID: {}\nAgents: {}\nMessages: {}\nCreated: {}",
        project.human_key,
        project.slug,
        project.id,
        agents.len(),
        message_count,
        project.created_at
    );
    Ok(CallToolResult::success(vec![Content::text(output)]))
}
