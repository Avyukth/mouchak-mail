//! Common helper functions for MCP tools
//!
//! This module contains reusable helper functions that are used across multiple tools.

use mouchak_mail_core::{
    ctx::Ctx,
    model::{
        ModelManager,
        agent::{Agent, AgentBmc},
        project::{Project, ProjectBmc},
    },
    utils::validation::{validate_agent_name, validate_project_key},
};
use rmcp::ErrorData as McpError;
use std::sync::Arc;

use crate::tools::errors::{ErrorCode, mcp_err};

/// Resolve a project by slug or human_key.
///
/// Validates input format before querying database.
/// Returns the project or an McpError with structured error code and suggestion.
pub async fn resolve_project(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    slug: &str,
) -> Result<Project, McpError> {
    // Validate input format first
    if let Err(e) = validate_project_key(slug) {
        return Err(mcp_err!(
            ErrorCode::InvalidProjectKey,
            &e.to_string(),
            { "project_slug": slug, "validation_error": e.to_string() }
        ));
    }

    ProjectBmc::get_by_identifier(ctx, mm, slug).await.map_err(|_| {
        mcp_err!(
            ErrorCode::ProjectNotFound,
            &format!("Project '{}' not found", slug),
            {
                "project_slug": slug,
                "suggestion": "Check project exists with list_projects or create with ensure_project"
            }
        )
    })
}

/// Resolve an agent by name within a project.
///
/// Validates input format before querying database.
/// Returns the agent or an McpError with structured error code and suggestion.
pub async fn resolve_agent(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    project_id: i64,
    agent_name: &str,
) -> Result<Agent, McpError> {
    if let Err(e) = validate_agent_name(agent_name) {
        let sanitized_name: String = agent_name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .take(64)
            .collect();
        let suggestion = if sanitized_name.is_empty() {
            "Use alphanumeric characters and underscores only (1-64 chars)".to_string()
        } else {
            sanitized_name
        };
        return Err(mcp_err!(
            ErrorCode::InvalidAgentName,
            &e.to_string(),
            { "agent_name": agent_name, "validation_error": e.to_string(), "suggestion": suggestion }
        ));
    }

    AgentBmc::get_by_name(
        ctx,
        mm,
        mouchak_mail_core::types::ProjectId::new(project_id),
        agent_name,
    )
    .await
    .map_err(|_| {
        mcp_err!(
            ErrorCode::AgentNotFound,
            &format!("Agent '{}' not found", agent_name),
            {
                "agent_name": agent_name,
                "project_id": project_id,
                "suggestion": "Check agent name with list_agents or register with register_agent"
            }
        )
    })
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
    let agent = resolve_agent(ctx, mm, project.id.get(), agent_name).await?;
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
    for name in names_csv
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let agent = resolve_agent(ctx, mm, project_id, name).await?;
        ids.push(agent.id.get());
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
