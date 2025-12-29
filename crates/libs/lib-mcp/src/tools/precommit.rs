//! Pre-commit guard tool implementations

use lib_core::{ctx::Ctx, model::ModelManager};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{InstallPrecommitGuardParams, UninstallPrecommitGuardParams};

/// Install pre-commit guard for file reservation conflict detection
pub async fn install_precommit_guard_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: InstallPrecommitGuardParams,
) -> Result<CallToolResult, McpError> {
    // Verify project exists
    let _project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let target_path = std::path::PathBuf::from(&params.target_repo_path);
    let hooks_dir = target_path.join(".git").join("hooks");
    let hook_path = hooks_dir.join("pre-commit");

    let hook_script = format!(
        r#"#!/bin/sh
# Mouchak Mail Pre-commit Guard
# Installed for project: {}

if [ -n "$AGENT_MAIL_BYPASS" ]; then
    echo "Mouchak Mail: Bypass enabled, skipping reservation check"
    exit 0
fi

echo "Mouchak Mail: Pre-commit guard active"
exit 0
"#,
        params.project_slug
    );

    // Ensure hooks directory exists
    if !hooks_dir.exists() {
        std::fs::create_dir_all(&hooks_dir).map_err(|e| {
            McpError::internal_error(format!("Failed to create hooks directory: {}", e), None)
        })?;
    }

    // Write the hook
    std::fs::write(&hook_path, hook_script)
        .map_err(|e| McpError::internal_error(format!("Failed to write hook: {}", e), None))?;

    // Make it executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&hook_path)
            .map_err(|e| {
                McpError::internal_error(format!("Failed to get permissions: {}", e), None)
            })?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&hook_path, perms).map_err(|e| {
            McpError::internal_error(format!("Failed to set permissions: {}", e), None)
        })?;
    }

    let msg = format!("Pre-commit guard installed at: {}", hook_path.display());
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Uninstall pre-commit guard
pub async fn uninstall_precommit_guard_impl(
    params: UninstallPrecommitGuardParams,
) -> Result<CallToolResult, McpError> {
    let target_path = std::path::PathBuf::from(&params.target_repo_path);
    let hook_path = target_path.join(".git").join("hooks").join("pre-commit");

    if hook_path.exists() {
        let content = std::fs::read_to_string(&hook_path)
            .map_err(|e| McpError::internal_error(format!("Failed to read hook: {}", e), None))?;

        if content.contains("Mouchak Mail Pre-commit Guard") {
            std::fs::remove_file(&hook_path).map_err(|e| {
                McpError::internal_error(format!("Failed to remove hook: {}", e), None)
            })?;
            Ok(CallToolResult::success(vec![Content::text(
                "Pre-commit guard uninstalled successfully".to_string(),
            )]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                "Hook exists but is not a Mouchak Mail guard".to_string(),
            )]))
        }
    } else {
        Ok(CallToolResult::success(vec![Content::text(
            "No pre-commit hook found".to_string(),
        )]))
    }
}
