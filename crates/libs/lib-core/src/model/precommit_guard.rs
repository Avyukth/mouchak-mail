use crate::{Result, ctx::Ctx, model::ModelManager};
use std::path::Path;
use tracing::{debug, info};

/// Parse boolean environment variable with truthy value detection.
///
/// Recognizes: "1", "true", "yes", "t", "y" (case-insensitive)
/// Returns false for unset or any other value.
fn parse_bool_env(key: &str) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "t" | "y"))
        .unwrap_or(false)
}

/// Check if worktree features should be active.
///
/// Returns true if either `WORKTREES_ENABLED` or `GIT_IDENTITY_ENABLED`
/// environment variable is set to a truthy value.
pub fn worktrees_active() -> bool {
    parse_bool_env("WORKTREES_ENABLED") || parse_bool_env("GIT_IDENTITY_ENABLED")
}

/// Pre-commit guard for file reservation checks
pub struct PrecommitGuardBmc;

impl PrecommitGuardBmc {
    /// Check if pre-commit guard should run.
    ///
    /// Returns true if `WORKTREES_ENABLED` or `GIT_IDENTITY_ENABLED` is set
    /// to a truthy value. This gates all pre-commit guard functionality.
    ///
    /// # Example
    ///
    /// ```
    /// # use lib_core::model::precommit_guard::PrecommitGuardBmc;
    /// std::env::set_var("WORKTREES_ENABLED", "1");
    /// assert!(PrecommitGuardBmc::should_check());
    /// std::env::remove_var("WORKTREES_ENABLED");
    /// ```
    pub fn should_check() -> bool {
        worktrees_active()
    }

    /// Check file reservations before commit.
    ///
    /// Early returns with `Ok(None)` if worktrees are not enabled.
    /// Returns `Ok(Some(violations))` if there are reservation violations,
    /// or `Ok(None)` if all checks pass.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - Request context
    /// * `_mm` - Model manager for database access
    /// * `_agent_name` - Agent attempting the commit
    /// * `_files` - List of files being committed
    ///
    /// # Returns
    ///
    /// * `Ok(None)` - No violations or worktrees not enabled
    /// * `Ok(Some(Vec<String>))` - List of violation messages
    pub async fn check_reservations(
        _ctx: &Ctx,
        _mm: &ModelManager,
        _agent_name: &str,
        _files: &[String],
    ) -> Result<Option<Vec<String>>> {
        // Gate: early return if worktrees not enabled
        if !Self::should_check() {
            info!("Pre-commit guard skipped: WORKTREES_ENABLED and GIT_IDENTITY_ENABLED not set");
            return Ok(None);
        }

        debug!(
            agent = _agent_name,
            files_count = _files.len(),
            "Checking file reservations"
        );

        // TODO: Implement actual reservation checking against database
        // For now, pass through with no violations
        Ok(None)
    }

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
    use serial_test::serial;
    use tempfile::TempDir;

    // ============================================================================
    // GATE BEHAVIOR TESTS (PORT-3.1)
    // Using temp-env for safe environment variable manipulation
    // Using serial_test to prevent race conditions between env var tests
    // ============================================================================

    #[test]
    fn test_parse_bool_env_truthy_values() {
        // All these should return true
        let truthy_cases = [
            ("TEST_BOOL_1", "1"),
            ("TEST_BOOL_TRUE", "true"),
            ("TEST_BOOL_TRUE_UPPER", "TRUE"),
            ("TEST_BOOL_TRUE_MIXED", "TrUe"),
            ("TEST_BOOL_YES", "yes"),
            ("TEST_BOOL_YES_UPPER", "YES"),
            ("TEST_BOOL_T", "t"),
            ("TEST_BOOL_T_UPPER", "T"),
            ("TEST_BOOL_Y", "y"),
            ("TEST_BOOL_Y_UPPER", "Y"),
        ];

        for (key, val) in truthy_cases {
            temp_env::with_var(key, Some(val), || {
                assert!(parse_bool_env(key), "Expected true for {}={}", key, val);
            });
        }
    }

    #[test]
    fn test_parse_bool_env_falsy_values() {
        // All these should return false
        let falsy_cases = [
            ("TEST_BOOL_0", "0"),
            ("TEST_BOOL_FALSE", "false"),
            ("TEST_BOOL_NO", "no"),
            ("TEST_BOOL_EMPTY", ""),
            ("TEST_BOOL_RANDOM", "random"),
        ];

        for (key, val) in falsy_cases {
            temp_env::with_var(key, Some(val), || {
                assert!(!parse_bool_env(key), "Expected false for {}={}", key, val);
            });
        }

        // Unset should return false
        temp_env::with_var_unset("UNSET_VAR_XYZ", || {
            assert!(!parse_bool_env("UNSET_VAR_XYZ"));
        });
    }

    #[test]
    #[serial]
    fn test_worktrees_active_neither_set() {
        temp_env::with_vars_unset(["WORKTREES_ENABLED", "GIT_IDENTITY_ENABLED"], || {
            assert!(
                !worktrees_active(),
                "worktrees_active should be false when neither env var set"
            );
        });
    }

    #[test]
    #[serial]
    fn test_worktrees_active_worktrees_enabled_only() {
        temp_env::with_vars(
            [
                ("WORKTREES_ENABLED", Some("1")),
                ("GIT_IDENTITY_ENABLED", None::<&str>),
            ],
            || {
                assert!(
                    worktrees_active(),
                    "worktrees_active should be true when WORKTREES_ENABLED=1"
                );
            },
        );
    }

    #[test]
    #[serial]
    fn test_worktrees_active_git_identity_only() {
        temp_env::with_vars(
            [
                ("WORKTREES_ENABLED", None::<&str>),
                ("GIT_IDENTITY_ENABLED", Some("true")),
            ],
            || {
                assert!(
                    worktrees_active(),
                    "worktrees_active should be true when GIT_IDENTITY_ENABLED=true"
                );
            },
        );
    }

    #[test]
    #[serial]
    fn test_worktrees_active_both_set() {
        temp_env::with_vars(
            [
                ("WORKTREES_ENABLED", Some("yes")),
                ("GIT_IDENTITY_ENABLED", Some("y")),
            ],
            || {
                assert!(
                    worktrees_active(),
                    "worktrees_active should be true when both env vars set"
                );
            },
        );
    }

    #[test]
    #[serial]
    fn test_worktrees_active_explicitly_disabled() {
        temp_env::with_vars(
            [
                ("WORKTREES_ENABLED", Some("0")),
                ("GIT_IDENTITY_ENABLED", Some("false")),
            ],
            || {
                assert!(
                    !worktrees_active(),
                    "worktrees_active should be false when explicitly set to falsy values"
                );
            },
        );
    }

    #[test]
    #[serial]
    fn test_should_check_delegates_to_worktrees_active() {
        // Test disabled state
        temp_env::with_vars_unset(["WORKTREES_ENABLED", "GIT_IDENTITY_ENABLED"], || {
            assert!(!PrecommitGuardBmc::should_check());
        });

        // Test enabled state
        temp_env::with_vars(
            [
                ("WORKTREES_ENABLED", Some("1")),
                ("GIT_IDENTITY_ENABLED", None::<&str>),
            ],
            || {
                assert!(PrecommitGuardBmc::should_check());
            },
        );
    }

    #[test]
    #[serial]
    fn test_check_reservations_skips_when_disabled() {
        temp_env::with_vars_unset(["WORKTREES_ENABLED", "GIT_IDENTITY_ENABLED"], || {
            // Create a new runtime inside the closure (not nested in existing runtime)
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let ctx = Ctx::root_ctx();
                let dummy_mm = {
                    use libsql::Builder;
                    let db_path = temp_dir.path().join("test.db");
                    let db = Builder::new_local(&db_path).build().await.unwrap();
                    let conn = db.connect().unwrap();
                    ModelManager::new_for_test(conn, temp_dir.path().to_path_buf())
                };

                // With env vars unset, should skip and return Ok(None)
                let result = PrecommitGuardBmc::check_reservations(
                    &ctx,
                    &dummy_mm,
                    "test_agent",
                    &["src/main.rs".to_string()],
                )
                .await;

                assert!(result.is_ok());
                assert!(
                    result.unwrap().is_none(),
                    "Should skip and return None when worktrees disabled"
                );
            });
        });
    }

    #[test]
    #[serial]
    fn test_check_reservations_runs_when_enabled() {
        temp_env::with_vars(
            [
                ("WORKTREES_ENABLED", Some("1")),
                ("GIT_IDENTITY_ENABLED", None::<&str>),
            ],
            || {
                // Create a new runtime inside the closure (not nested in existing runtime)
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let ctx = Ctx::root_ctx();
                    let dummy_mm = {
                        use libsql::Builder;
                        let db_path = temp_dir.path().join("test.db");
                        let db = Builder::new_local(&db_path).build().await.unwrap();
                        let conn = db.connect().unwrap();
                        ModelManager::new_for_test(conn, temp_dir.path().to_path_buf())
                    };

                    // With WORKTREES_ENABLED=1, should run check (currently passes through)
                    let result = PrecommitGuardBmc::check_reservations(
                        &ctx,
                        &dummy_mm,
                        "test_agent",
                        &["src/main.rs".to_string()],
                    )
                    .await;

                    assert!(result.is_ok());
                    // Currently returns None (no violations) as check is TODO
                    assert!(result.unwrap().is_none());
                });
            },
        );
    }

    // ============================================================================
    // EXISTING TESTS
    // ============================================================================

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
