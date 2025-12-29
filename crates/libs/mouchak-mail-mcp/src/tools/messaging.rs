//! Messaging tool implementations
//!
//! Handles sending, receiving, threading, and searching messages.

use mouchak_mail_core::{
    ctx::Ctx,
    model::{
        ModelManager,
        agent_capabilities::AgentCapabilityBmc,
        message::{MessageBmc, MessageForCreate},
    },
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{
    AcknowledgeMessageParams, GetMessageParams, GetThreadParams, ListInboxParams,
    ListThreadsParams, MarkMessageReadParams, ReplyMessageParams, SearchMessagesParams,
    SendMessageParams,
};

/// Send a message from one agent to others.
pub async fn send_message_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: SendMessageParams,
) -> Result<CallToolResult, McpError> {
    let (project, sender) =
        helpers::resolve_project_and_agent(ctx, mm, &params.project_slug, &params.sender_name)
            .await?;

    if !AgentCapabilityBmc::check(ctx, mm, sender.id.get(), "send_message")
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
    {
        return Err(McpError::invalid_params(
            format!(
                "Agent '{}' does not have 'send_message' capability",
                params.sender_name
            ),
            None,
        ));
    }

    let recipient_ids = helpers::resolve_agent_names(ctx, mm, project.id.get(), &params.to).await?;

    let cc_ids =
        helpers::resolve_optional_agent_names(ctx, mm, project.id.get(), params.cc.as_deref())
            .await?;

    let bcc_ids =
        helpers::resolve_optional_agent_names(ctx, mm, project.id.get(), params.bcc.as_deref())
            .await?;

    let msg_c = MessageForCreate {
        project_id: project.id.get(),
        sender_id: sender.id.get(),
        recipient_ids,
        cc_ids,
        bcc_ids,
        subject: params.subject.clone(),
        body_md: params.body_md,
        thread_id: params.thread_id,
        importance: params.importance,
        ack_required: params.ack_required.unwrap_or(false),
    };

    let msg_id = MessageBmc::create(ctx, mm, msg_c)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Message sent (id: {}) from '{}' to '{}' with subject '{}'",
        msg_id, params.sender_name, params.to, params.subject
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// List messages in an agent's inbox.
pub async fn list_inbox_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListInboxParams,
) -> Result<CallToolResult, McpError> {
    let (project, agent) =
        helpers::resolve_project_and_agent(ctx, mm, &params.project_slug, &params.agent_name)
            .await?;

    if !AgentCapabilityBmc::check(ctx, mm, agent.id.get(), "fetch_inbox")
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
    {
        return Err(McpError::invalid_params(
            format!(
                "Agent '{}' does not have 'fetch_inbox' capability",
                params.agent_name
            ),
            None,
        ));
    }

    let messages = MessageBmc::list_inbox_for_agent(
        ctx,
        mm,
        project.id.get(),
        agent.id.get(),
        params.limit.unwrap_or(50),
    )
    .await
    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!(
        "Inbox for '{}' ({} messages):\n\n",
        params.agent_name,
        messages.len()
    );
    for m in &messages {
        output.push_str(&format!(
            "- [{}] {} (from: {}, thread: {:?}, {})\n",
            m.id, m.subject, m.sender_name, m.thread_id, m.importance
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Get a specific message by ID.
pub async fn get_message_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: GetMessageParams,
) -> Result<CallToolResult, McpError> {
    let message = MessageBmc::get(ctx, mm, params.message_id)
        .await
        .map_err(|e| McpError::invalid_params(format!("Message not found: {}", e), None))?;

    let output = format!(
        "Message ID: {}\nFrom: {}\nSubject: {}\nThread: {:?}\nImportance: {}\nCreated: {}\n\n---\n{}",
        message.id,
        message.sender_name,
        message.subject,
        message.thread_id,
        message.importance,
        message.created_ts,
        message.body_md
    );

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Search messages using full-text search.
pub async fn search_messages_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: SearchMessagesParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let messages = MessageBmc::search(
        ctx,
        mm,
        project.id.get(),
        &params.query,
        params.limit.unwrap_or(20),
    )
    .await
    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!(
        "Search results for '{}' ({} matches):\n\n",
        params.query,
        messages.len()
    );
    for m in &messages {
        output.push_str(&format!(
            "- [{}] {} (from: {}, thread: {:?})\n",
            m.id, m.subject, m.sender_name, m.thread_id
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Get all messages in a thread.
pub async fn get_thread_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: GetThreadParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let messages = MessageBmc::list_by_thread(ctx, mm, project.id.get(), &params.thread_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!(
        "Thread '{}' ({} messages):\n\n",
        params.thread_id,
        messages.len()
    );
    for m in &messages {
        output.push_str(&format!(
            "---\n[{}] From: {} | {}\nSubject: {}\n\n{}\n\n",
            m.id, m.sender_name, m.created_ts, m.subject, m.body_md
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Reply to an existing message.
pub async fn reply_message_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ReplyMessageParams,
) -> Result<CallToolResult, McpError> {
    let (project, sender) =
        helpers::resolve_project_and_agent(ctx, mm, &params.project_slug, &params.sender_name)
            .await?;

    if !AgentCapabilityBmc::check(ctx, mm, sender.id.get(), "send_message")
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
    {
        return Err(McpError::invalid_params(
            format!(
                "Agent '{}' does not have 'send_message' capability",
                params.sender_name
            ),
            None,
        ));
    }

    let original_msg = MessageBmc::get(ctx, mm, params.message_id)
        .await
        .map_err(|e| McpError::invalid_params(format!("Message not found: {}", e), None))?;

    let subject = if original_msg.subject.starts_with("Re: ") {
        original_msg.subject.clone()
    } else {
        format!("Re: {}", original_msg.subject)
    };

    let msg_c = MessageForCreate {
        project_id: project.id.get(),
        sender_id: sender.id.get(),
        recipient_ids: vec![original_msg.sender_id],
        cc_ids: None,
        bcc_ids: None,
        subject: subject.clone(),
        body_md: params.body_md,
        thread_id: original_msg.thread_id.clone(),
        importance: params.importance,
        ack_required: false,
    };

    let msg_id = MessageBmc::create(ctx, mm, msg_c)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!("Reply sent (id: {}) with subject '{}'", msg_id, subject);
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Mark a message as read.
pub async fn mark_message_read_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: MarkMessageReadParams,
) -> Result<CallToolResult, McpError> {
    let (_project, agent) =
        helpers::resolve_project_and_agent(ctx, mm, &params.project_slug, &params.agent_name)
            .await?;

    MessageBmc::mark_read(ctx, mm, params.message_id, agent.id.get())
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Message {} marked as read by '{}'",
        params.message_id, params.agent_name
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Acknowledge a message requiring acknowledgment.
pub async fn acknowledge_message_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: AcknowledgeMessageParams,
) -> Result<CallToolResult, McpError> {
    let (_project, agent) =
        helpers::resolve_project_and_agent(ctx, mm, &params.project_slug, &params.agent_name)
            .await?;

    if !AgentCapabilityBmc::check(ctx, mm, agent.id.get(), "acknowledge_message")
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
    {
        return Err(McpError::invalid_params(
            format!(
                "Agent '{}' does not have 'acknowledge_message' capability",
                params.agent_name
            ),
            None,
        ));
    }

    MessageBmc::acknowledge(ctx, mm, params.message_id, agent.id.get())
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Message {} acknowledged by '{}'",
        params.message_id, params.agent_name
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// List all conversation threads in a project.
pub async fn list_threads_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListThreadsParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let threads = MessageBmc::list_threads(ctx, mm, project.id.get(), params.limit.unwrap_or(50))
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!(
        "Threads in '{}' ({}):\n\n",
        params.project_slug,
        threads.len()
    );
    for t in &threads {
        output.push_str(&format!(
            "- {} | {} ({} msgs, last: {})\n",
            t.thread_id, t.subject, t.message_count, t.last_message_ts
        ));
    }
    Ok(CallToolResult::success(vec![Content::text(output)]))
}
