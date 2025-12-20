//! Common helper functions for MCP tools
//!
//! This module contains reusable helper functions that are used across multiple tools.

use lib_core::{
    ctx::Ctx,
    model::{
        agent::{Agent, AgentBmc},
        project::{Project, ProjectBmc},
        ModelManager,
    },
};
use rmcp::ErrorData as McpError;
use std::sync::Arc;

/// Resolve a project by slug or human_key.
///
/// Returns the project or an McpError with a user-friendly message.
pub async fn resolve_project(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    slug: &str,
) -> Result<Project, McpError> {
    ProjectBmc::get_by_identifier(ctx, mm, slug)
        .await
        .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))
}

/// Resolve an agent by name within a project.
///
/// Returns the agent or an McpError with a user-friendly message.
pub async fn resolve_agent(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    project_id: i64,
    agent_name: &str,
) -> Result<Agent, McpError> {
    AgentBmc::get_by_name(ctx, mm, project_id, agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))
}

/// Resolve project and agent in one call.
///
/// Common pattern: look up project by slug, then agent by name.
pub async fn resolve_project_and_agent(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    project_slug: &str,
    agent_name: &str,
) -> Result<(Project, Agent), McpError> {
    let project = resolve_project(ctx, mm, project_slug).await?;
    let agent = resolve_agent(ctx, mm, project.id, agent_name).await?;
    Ok((project, agent))
}

/// Parse comma-separated agent names and resolve them to IDs.
///
/// Returns Vec of agent IDs or error if any agent not found.
pub async fn resolve_agent_names(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    project_id: i64,
    names_csv: &str,
) -> Result<Vec<i64>, McpError> {
    let mut ids = Vec::new();
    for name in names_csv.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let agent = resolve_agent(ctx, mm, project_id, name).await?;
        ids.push(agent.id);
    }
    Ok(ids)
}

/// Parse optional comma-separated agent names and resolve them to IDs.
///
/// Returns None if input is None or empty, otherwise Vec of agent IDs.
pub async fn resolve_optional_agent_names(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    project_id: i64,
    names_csv: Option<&str>,
) -> Result<Option<Vec<i64>>, McpError> {
    match names_csv {
        Some(names) if !names.trim().is_empty() => {
            Ok(Some(resolve_agent_names(ctx, mm, project_id, names).await?))
        }
        _ => Ok(None),
    }
}
