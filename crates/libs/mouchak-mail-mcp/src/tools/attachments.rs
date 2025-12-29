//! Attachment tool implementations
//!
//! Handles adding and retrieving message attachments via Git storage.

use mouchak_mail_core::{ctx::Ctx, model::ModelManager, store::git_store};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{AddAttachmentParams, GetAttachmentParams};

/// Add attachment to a message (base64 encoded, stored in Git).
pub async fn add_attachment_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: AddAttachmentParams,
) -> Result<CallToolResult, McpError> {
    use mouchak_mail_core::model::message::MessageBmc;

    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    // Verify message exists
    let _message = MessageBmc::get(ctx, mm, params.message_id)
        .await
        .map_err(|e| McpError::invalid_params(format!("Message not found: {}", e), None))?;

    // Generate attachment ID
    let attachment_id = format!(
        "att_{}_{}",
        params.message_id,
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("0")
    );

    // Store attachment in Git
    let repo = git_store::open_repo(&mm.repo_root)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let attachment_path = std::path::PathBuf::from("projects")
        .join(&project.slug)
        .join("attachments")
        .join(&attachment_id)
        .join(&params.filename);

    // Decode base64 content
    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD
        .decode(&params.content_base64)
        .map_err(|e| McpError::invalid_params(format!("Invalid base64: {}", e), None))?;

    git_store::commit_file(
        &repo,
        &attachment_path,
        &String::from_utf8_lossy(&content),
        &format!(
            "attachment: {} for message {}",
            params.filename, params.message_id
        ),
        "mcp-bot",
        "mcp-bot@localhost",
    )
    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Attachment '{}' added with ID {}",
        params.filename, attachment_id
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Get attachment from a message (returns base64 encoded content).
pub async fn get_attachment_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: GetAttachmentParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let repo = git_store::open_repo(&mm.repo_root)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let attachment_path = std::path::PathBuf::from("projects")
        .join(&project.slug)
        .join("attachments")
        .join(&params.attachment_id)
        .join(&params.filename);

    match git_store::read_file_content(&repo, &attachment_path) {
        Ok(content) => {
            use base64::Engine;
            let content_base64 =
                base64::engine::general_purpose::STANDARD.encode(content.as_bytes());

            let mime_type = match params.filename.rsplit('.').next() {
                Some("txt") => "text/plain",
                Some("json") => "application/json",
                Some("md") => "text/markdown",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("pdf") => "application/pdf",
                _ => "application/octet-stream",
            };

            let output = format!(
                "Attachment: {}\nMIME Type: {}\n\nContent (base64):\n{}",
                params.filename, mime_type, content_base64
            );
            Ok(CallToolResult::success(vec![Content::text(output)]))
        }
        Err(_) => Err(McpError::invalid_params(
            format!("Attachment not found: {}", params.filename),
            None,
        )),
    }
}
