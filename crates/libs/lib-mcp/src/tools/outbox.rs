//! Outbox tool implementations
//!
//! Handles listing sent messages for agents.

use lib_core::{
    ctx::Ctx,
    model::{ModelManager, agent::AgentBmc, message::MessageBmc},
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::ListOutboxParams;
use super::helpers;

/// List messages in an agent's outbox (sent messages).
pub async fn list_outbox_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListOutboxParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let messages = MessageBmc::list_outbox_for_agent(
        ctx,
        mm,
        project.id.get(),
        agent.id.get(),
        params.limit.unwrap_or(50),
    )
    .await
    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!(
        "Outbox for '{}' ({} messages):\n\n",
        params.agent_name,
        messages.len()
    );
    for m in &messages {
        output.push_str(&format!(
            "- [{}] {} (to: {:?}, thread: {:?}, {})\n",
            m.id, m.subject, m.sender_name, m.thread_id, m.importance
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
