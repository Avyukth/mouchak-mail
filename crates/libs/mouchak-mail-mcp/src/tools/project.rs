//! Project management tool implementations
//!
//! Handles project creation, listing, and info retrieval.

use mouchak_mail_core::{
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
    let slug = params.effective_slug();

    validate_project_key(&slug).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    match ProjectBmc::get_by_identifier(ctx, mm, &slug).await {
        Ok(project) => {
            let msg = format!(
                "Project exists: {} (id: {}, created: {})",
                project.slug, project.id, project.created_at
            );
            Ok(CallToolResult::success(vec![Content::text(msg)]))
        }
        Err(_) => {
            let human_key = params.effective_human_key();
            let id = ProjectBmc::create(ctx, mm, &slug, &human_key)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let msg = format!("Created project '{}' with id {}", slug, id);
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

    let message_count = ProjectBmc::count_messages(ctx, mm, project.id)
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
