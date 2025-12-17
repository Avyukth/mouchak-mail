use crate::{Result, ctx::Ctx, model::ModelManager};
use std::path::Path;

/// Pre-commit guard for file reservation checks
pub struct PrecommitGuardBmc;

impl PrecommitGuardBmc {
    /// Install pre-commit hook in git repository
    pub async fn install(_ctx: &Ctx, _mm: &ModelManager, git_repo_path: &Path) -> Result<String> {
        let hooks_dir = git_repo_path.join(".git").join("hooks");
        let hook_path = hooks_dir.join("pre-commit");

        // Create hooks directory if it doesn't exist
        tokio::fs::create_dir_all(&hooks_dir).await?;

        // Hook script content
        let hook_script = r#"#!/bin/sh
# MCP Agent Mail - Pre-commit Guard
# Checks file reservations before allowing commit

# Check if AGENT_NAME is set
if [ -z "$AGENT_NAME" ]; then
    echo "Warning: AGENT_NAME not set, skipping reservation check"
    exit 0
fi

# TODO: Call agent mail API to verify file reservations
# For now, just pass through
exit 0
"#;

        // Write hook file
        tokio::fs::write(&hook_path, hook_script).await?;

        // Make executable (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&hook_path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&hook_path, perms).await?;
        }

        Ok(format!("Pre-commit hook installed at {:?}", hook_path))
    }

    /// Uninstall pre-commit hook from git repository
    pub async fn uninstall(_ctx: &Ctx, _mm: &ModelManager, git_repo_path: &Path) -> Result<String> {
        let hook_path = git_repo_path.join(".git").join("hooks").join("pre-commit");

        if hook_path.exists() {
            tokio::fs::remove_file(&hook_path).await?;
            Ok(format!("Pre-commit hook removed from {:?}", hook_path))
        } else {
            Ok("No pre-commit hook found".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_install_uninstall_precommit_guard() {
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join("test_repo");

        // Initialize a git repo structure
        std::fs::create_dir_all(git_dir.join(".git/hooks")).unwrap();

        let ctx = Ctx::root_ctx();
        let dummy_mm = {
            use libsql::Builder;
            let db_path = temp_dir.path().join("test.db");
            let db = Builder::new_local(&db_path).build().await.unwrap();
            let conn = db.connect().unwrap();
            ModelManager::new_for_test(conn, temp_dir.path().to_path_buf())
        };

        // Test install
        let result = PrecommitGuardBmc::install(&ctx, &dummy_mm, &git_dir).await;
        assert!(result.is_ok());
        assert!(git_dir.join(".git/hooks/pre-commit").exists());

        // Test uninstall
        let result = PrecommitGuardBmc::uninstall(&ctx, &dummy_mm, &git_dir).await;
        assert!(result.is_ok());
        assert!(!git_dir.join(".git/hooks/pre-commit").exists());
    }
}
