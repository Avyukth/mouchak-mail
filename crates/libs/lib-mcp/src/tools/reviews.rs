//! Review tool implementations
//!
//! Handles task review state tracking and claiming.

use lib_core::{
    ctx::Ctx,
    model::{
        ModelManager,
        agent::AgentBmc,
        message::{MessageBmc, MessageForCreate},
        orchestration::{OrchestrationState, parse_thread_state},
    },
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{ClaimResult, ClaimReviewParams, GetReviewStateParams, ReviewStateResponse};

/// Get the current review state of a task thread based on message prefixes.
pub async fn get_review_state_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: GetReviewStateParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let messages = MessageBmc::list_by_thread(ctx, mm, project.id.get(), &params.thread_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let state = parse_thread_state(&messages);
    let is_reviewed = matches!(
        state,
        OrchestrationState::Approved | OrchestrationState::Acknowledged
    );

    // Try to find reviewer from [REVIEWING] or [APPROVED] messages
    let reviewer = messages
        .iter()
        .rev()
        .find(|m| {
            let subj = m.subject.to_uppercase();
            subj.starts_with("[REVIEWING]") || subj.starts_with("[APPROVED]")
        })
        .map(|m| m.sender_name.clone());

    let last_update = messages
        .last()
        .map(|m| m.created_ts.to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let response = ReviewStateResponse {
        thread_id: params.thread_id.clone(),
        state: state.prefix().to_string(),
        is_reviewed,
        reviewer,
        last_update,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Claim a pending review by sending a [REVIEWING] message.
pub async fn claim_review_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ClaimReviewParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    // Get the completion message
    let message = MessageBmc::get(ctx, mm, params.message_id)
        .await
        .map_err(|e| McpError::invalid_params(format!("Message not found: {}", e), None))?;

    let thread_id = message
        .thread_id
        .clone()
        .unwrap_or_else(|| format!("TASK-{}", params.message_id));

    // Get thread messages to check state
    let messages = MessageBmc::list_by_thread(ctx, mm, project.id.get(), &thread_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let state = parse_thread_state(&messages);

    // Check if already claimed (REVIEWING state)
    if matches!(state, OrchestrationState::Reviewing) {
        // Find who claimed it
        let claimer = messages
            .iter()
            .rev()
            .find(|m| m.subject.to_uppercase().starts_with("[REVIEWING]"))
            .map(|m| m.sender_name.clone());

        let result = ClaimResult {
            success: false,
            thread_id,
            claimed_by: claimer,
        };
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        return Ok(CallToolResult::success(vec![Content::text(json)]));
    }

    // Get reviewer agent ID
    let reviewer = AgentBmc::get_by_name(ctx, mm, project.id, &params.reviewer_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Reviewer not found: {}", e), None))?;

    // Send [REVIEWING] message
    let msg = MessageForCreate {
        project_id: project.id.get(),
        sender_id: reviewer.id.get(),
        recipient_ids: vec![message.sender_id],
        cc_ids: None,
        bcc_ids: None,
        subject: format!(
            "[REVIEWING] {}",
            message.subject.trim_start_matches("[COMPLETION]").trim()
        ),
        body_md: format!(
            "Claiming review for task. Original message ID: {}",
            params.message_id
        ),
        thread_id: Some(thread_id.clone()),
        importance: None,
        ack_required: false,
    };

    MessageBmc::create(ctx, mm, msg)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let result = ClaimResult {
        success: true,
        thread_id,
        claimed_by: Some(params.reviewer_name),
    };
    let json = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(CallToolResult::success(vec![Content::text(json)]))
}
