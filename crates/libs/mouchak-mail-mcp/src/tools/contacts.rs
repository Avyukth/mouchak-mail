//! Contact management tool implementations
//!
//! Handles agent-to-agent contact requests and policies.

use mouchak_mail_core::{
    ctx::Ctx,
    model::{
        ModelManager,
        agent::{AgentBmc, AgentProfileUpdate},
        agent_link::{AgentLinkBmc, AgentLinkForCreate},
        project::ProjectBmc,
    },
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{
    ListContactsParams, RequestContactParams, RespondContactParams, SetContactPolicyParams,
};

/// Request to add another agent as a contact.
pub async fn request_contact_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: RequestContactParams,
) -> Result<CallToolResult, McpError> {
    let from_project = ProjectBmc::get_by_identifier(ctx, mm, &params.from_project_slug)
        .await
        .map_err(|e| McpError::invalid_params(format!("From project not found: {}", e), None))?;
    let from_agent = AgentBmc::get_by_name(ctx, mm, from_project.id, &params.from_agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("From agent not found: {}", e), None))?;

    let to_project = ProjectBmc::get_by_identifier(ctx, mm, &params.to_project_slug)
        .await
        .map_err(|e| McpError::invalid_params(format!("To project not found: {}", e), None))?;
    let to_agent = AgentBmc::get_by_name(ctx, mm, to_project.id, &params.to_agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("To agent not found: {}", e), None))?;

    let link_c = AgentLinkForCreate {
        a_project_id: from_project.id.get(),
        a_agent_id: from_agent.id.get(),
        b_project_id: to_project.id.get(),
        b_agent_id: to_agent.id.get(),
        reason: params.reason,
    };

    let link_id = AgentLinkBmc::request_contact(ctx, mm, link_c)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Contact request sent (link_id: {}, status: pending)",
        link_id
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Accept or reject a contact request.
pub async fn respond_contact_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: RespondContactParams,
) -> Result<CallToolResult, McpError> {
    AgentLinkBmc::respond_contact(ctx, mm, params.link_id, params.accept)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let status = if params.accept {
        "accepted"
    } else {
        "rejected"
    };
    let msg = format!("Contact request {} {}", params.link_id, status);
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// List all contacts for an agent.
pub async fn list_contacts_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListContactsParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let links = AgentLinkBmc::list_contacts(ctx, mm, project.id.get(), agent.id.get())
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!(
        "Contacts for '{}' ({}):\n\n",
        params.agent_name,
        links.len()
    );
    for link in &links {
        let (other_project_id, other_agent_id) = if link.a_agent_id == agent.id.get() {
            (link.b_project_id, link.b_agent_id)
        } else {
            (link.a_project_id, link.a_agent_id)
        };
        output.push_str(&format!(
            "- [{}] project:{} agent:{} (status: {}, reason: {})\n",
            link.id, other_project_id, other_agent_id, link.status, link.reason
        ));
    }
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Set an agent's contact acceptance policy.
pub async fn set_contact_policy_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: SetContactPolicyParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let update = AgentProfileUpdate {
        task_description: None,
        attachments_policy: None,
        contact_policy: Some(params.contact_policy.clone()),
    };

    AgentBmc::update_profile(ctx, mm, agent.id, update)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Contact policy for '{}' set to '{}'",
        params.agent_name, params.contact_policy
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}
