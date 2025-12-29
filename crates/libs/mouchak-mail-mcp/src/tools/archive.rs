//! Archive tool implementations
//!
//! Handles committing project state to git archive.

use mouchak_mail_core::{
    ctx::Ctx,
    model::{ModelManager, project::ProjectBmc},
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::CommitArchiveParams;
use super::helpers;

/// Commit project state (mailbox, agents) to the git archive.
pub async fn commit_archive_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: CommitArchiveParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let oid = ProjectBmc::sync_to_archive(ctx, mm, project.id, &params.message)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Archived project '{}' to git. Commit ID: {}",
        project.slug, oid
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}
