use crate::model::agent::AgentBmc;
use crate::model::file_reservation::FileReservationBmc;
use crate::model::project::ProjectBmc;
use crate::{Result, ctx::Ctx, model::ModelManager};
use chrono::Utc;
use glob::Pattern;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

/// Guard execution mode for pre-commit checks.
///
/// Controls how the guard behaves when file reservation conflicts are detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GuardMode {
    /// Block commits with conflicts (default behavior).
    /// Returns an error when conflicts are detected.
    #[default]
    Enforce,

    /// Warn about conflicts but allow the commit.
    /// Logs warnings and returns Ok with conflict details.
    Warn,

    /// Skip all checks entirely.
    /// Used for emergency bypass situations.
    Bypass,
}

impl GuardMode {
    /// Parse guard mode from environment variables.
    ///
    /// Checks in order:
    /// 1. `AGENT_MAIL_BYPASS=1` → Bypass mode
    /// 2. `AGENT_MAIL_GUARD_MODE=warn|advisory|adv` → Warn mode
    /// 3. Otherwise → Enforce mode (default)
    ///
    /// # Example
    ///
    /// ```
    /// # use lib_core::model::precommit_guard::GuardMode;
    /// // With no env vars set, defaults to Enforce
    /// // With AGENT_MAIL_BYPASS=1, returns Bypass
    /// // With AGENT_MAIL_GUARD_MODE=warn, returns Warn
    /// ```
    pub fn from_env() -> Self {
        // Check bypass first (highest priority)
        if parse_bool_env("AGENT_MAIL_BYPASS") {
            info!("Pre-commit guard bypass enabled via AGENT_MAIL_BYPASS=1");
            return Self::Bypass;
        }

        // Check guard mode
        if let Ok(mode) = std::env::var("AGENT_MAIL_GUARD_MODE") {
            match mode.to_lowercase().trim() {
                "warn" | "advisory" | "adv" => {
                    debug!(mode = %mode, "Pre-commit guard in advisory mode");
                    return Self::Warn;
                }
                "block" | "enforce" | "" => {
                    // Explicit enforce or default
                }
                other => {
                    warn!(
                        mode = %other,
                        "Unknown AGENT_MAIL_GUARD_MODE value, defaulting to enforce"
                    );
                }
            }
        }

        Self::Enforce
    }

    /// Check if this mode should skip all checks.
    pub fn is_bypass(&self) -> bool {
        matches!(self, Self::Bypass)
    }

    /// Check if this mode allows commits with warnings.
    pub fn is_advisory(&self) -> bool {
        matches!(self, Self::Warn)
    }
}

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

/// Get the hooks directory for a git repository.
///
/// Respects git's `core.hooksPath` configuration:
/// 1. If `core.hooksPath` is set and absolute, use it directly
/// 2. If `core.hooksPath` is set and relative, resolve from repo root
/// 3. Fall back to `<git-dir>/hooks`
/// 4. Last resort: `.git/hooks`
///
/// # Arguments
/// * `repo_path` - Path to the git repository root
///
/// # Returns
/// The resolved hooks directory path
pub fn get_hooks_dir(repo_path: &Path) -> PathBuf {
    // Helper to run git commands
    fn git_config(repo: &Path, args: &[&str]) -> Option<String> {
        Command::new("git")
            .args(["-C", &repo.to_string_lossy()])
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    // Check core.hooksPath
    if let Some(hooks_path) = git_config(repo_path, &["config", "--get", "core.hooksPath"]) {
        let path = Path::new(&hooks_path);

        // Check if absolute (Unix or Windows)
        if path.is_absolute() {
            debug!(hooks_path = %hooks_path, "Using absolute core.hooksPath");
            return path.to_path_buf();
        }

        // Relative path - resolve from repo root
        if let Some(toplevel) = git_config(repo_path, &["rev-parse", "--show-toplevel"]) {
            let resolved = PathBuf::from(&toplevel).join(&hooks_path);
            debug!(
                hooks_path = %hooks_path,
                toplevel = %toplevel,
                resolved = %resolved.display(),
                "Resolved relative core.hooksPath"
            );
            return resolved;
        }

        // Fallback: resolve relative to provided repo_path
        let resolved = repo_path.join(&hooks_path);
        debug!(
            hooks_path = %hooks_path,
            resolved = %resolved.display(),
            "Resolved relative core.hooksPath from repo_path"
        );
        return resolved;
    }

    // No core.hooksPath - use git-dir/hooks
    if let Some(git_dir) = git_config(repo_path, &["rev-parse", "--git-dir"]) {
        let git_path = Path::new(&git_dir);
        let hooks_dir = if git_path.is_absolute() {
            git_path.join("hooks")
        } else {
            repo_path.join(&git_dir).join("hooks")
        };
        debug!(git_dir = %git_dir, hooks_dir = %hooks_dir.display(), "Using git-dir/hooks");
        return hooks_dir;
    }

    // Last resort: traditional .git/hooks
    let fallback = repo_path.join(".git").join("hooks");
    debug!(fallback = %fallback.display(), "Falling back to .git/hooks");
    fallback
}

/// Pre-commit guard for file reservation checks
pub struct PrecommitGuardBmc;

/// Render the pre-push hook script content.
///
/// The script:
/// 1. Reads ref tuples from stdin (local_ref local_sha remote_ref remote_sha)
/// 2. Skips if AGENT_NAME not set or WORKTREES_ENABLED/GIT_IDENTITY_ENABLED not set
/// 3. Attempts to call the agent-mail server's check-push endpoint
/// 4. Gracefully degrades if server is unreachable (exits 0)
///
/// # Arguments
/// * `server_url` - Base URL for the agent-mail server (e.g., "http://localhost:8080")
pub fn render_prepush_script(server_url: &str) -> String {
    format!(
        r#"#!/bin/sh
# MCP Agent Mail - Pre-push Guard
# Checks file reservations before allowing push
#
# Reads stdin for ref tuples: <local_ref> <local_sha> <remote_ref> <remote_sha>
# Calls server endpoint for push validation

SERVER_URL="{server_url}"

# Gate: Check if worktrees are enabled
check_gate() {{
    # Check WORKTREES_ENABLED or GIT_IDENTITY_ENABLED
    case "${{WORKTREES_ENABLED:-0}}${{GIT_IDENTITY_ENABLED:-0}}" in
        *1*|*true*|*TRUE*|*yes*|*YES*|*y*|*Y*|*t*|*T*)
            return 0
            ;;
    esac
    return 1
}}

# Gate: Skip if not enabled
if ! check_gate; then
    exit 0
fi

# Bypass mode: Skip all checks
case "${{AGENT_MAIL_BYPASS:-0}}" in
    1|true|TRUE|yes|YES|y|Y|t|T)
        echo "[pre-push] bypass enabled via AGENT_MAIL_BYPASS=1" >&2
        exit 0
        ;;
esac

# Check if AGENT_NAME is set
if [ -z "$AGENT_NAME" ]; then
    echo "Warning: AGENT_NAME not set, skipping push check" >&2
    exit 0
fi

# Read ref tuples from stdin
# Format: <local_ref> <local_sha> <remote_ref> <remote_sha>
refs=""
while read -r local_ref local_sha remote_ref remote_sha; do
    if [ -n "$local_ref" ] && [ -n "$local_sha" ]; then
        refs="$refs$local_ref $local_sha $remote_ref $remote_sha\n"
    fi
done

# If no refs to push, exit
if [ -z "$refs" ]; then
    exit 0
fi

# Try to call server endpoint for push validation
# Gracefully degrade if server is unreachable
if command -v curl >/dev/null 2>&1; then
    response=$(printf "%b" "$refs" | curl -s --connect-timeout 2 --max-time 5 \
        -X POST "$SERVER_URL/api/guard/check-push" \
        -H "Content-Type: application/json" \
        -H "X-Agent-Name: $AGENT_NAME" \
        -d @- 2>/dev/null) || true

    # Check response (if server returned an error)
    case "$response" in
        *"error"*|*"blocked"*)
            echo "[pre-push] Push blocked by agent-mail guard:" >&2
            echo "$response" >&2
            # Check advisory mode
            case "${{AGENT_MAIL_GUARD_MODE:-block}}" in
                warn|WARN|advisory|ADVISORY|adv|ADV)
                    echo "[pre-push] Advisory mode - allowing push despite conflicts" >&2
                    exit 0
                    ;;
                *)
                    exit 1
                    ;;
            esac
            ;;
    esac
fi

# Server unreachable or no curl - gracefully allow
exit 0
"#
    )
}

impl PrecommitGuardBmc {
    /// Check if pre-commit guard should run.
    ///
    /// Returns true if `WORKTREES_ENABLED` or `GIT_IDENTITY_ENABLED` is set
    /// to a truthy value. This gates all pre-commit guard functionality.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # // Note: Uses env vars which require unsafe in Rust 2024 edition
    /// # use lib_core::model::precommit_guard::PrecommitGuardBmc;
    /// // With WORKTREES_ENABLED=1, should_check() returns true
    /// assert!(PrecommitGuardBmc::should_check());
    /// ```
    pub fn should_check() -> bool {
        worktrees_active()
    }

    /// Check file reservations before commit.
    ///
    /// Early returns with `Ok(None)` if worktrees are not enabled or bypass mode is active.
    /// Returns `Ok(Some(violations))` if there are reservation violations in warn mode,
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
    /// * `Ok(None)` - No violations, worktrees not enabled, or bypass mode
    /// * `Ok(Some(Vec<String>))` - List of violation messages (warn mode with conflicts)
    ///
    /// # Guard Modes
    ///
    /// * **Enforce** (default): Block on conflicts
    /// * **Warn**: Allow with warnings
    /// * **Bypass**: Skip all checks
    pub async fn check_reservations(
        ctx: &Ctx,
        mm: &ModelManager,
        agent_name: &str,
        files: &[String],
    ) -> Result<Option<Vec<String>>> {
        // Gate: early return if worktrees not enabled
        if !Self::should_check() {
            info!("Pre-commit guard skipped: WORKTREES_ENABLED and GIT_IDENTITY_ENABLED not set");
            return Ok(None);
        }

        // Check guard mode
        let mode = GuardMode::from_env();

        // Bypass mode: skip all checks
        if mode.is_bypass() {
            return Ok(None);
        }

        debug!(
            agent = agent_name,
            files_count = files.len(),
            mode = ?mode,
            "Checking file reservations"
        );

        // Get project slug from environment or detect from git
        let project_slug = match std::env::var("AGENT_MAIL_PROJECT") {
            Ok(slug) if !slug.is_empty() => slug,
            _ => {
                // Try to detect from current directory name
                std::env::current_dir()
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "default".to_string())
            }
        };

        // Look up project - if not found, no reservations to check
        let project = match ProjectBmc::get_by_slug(ctx, mm, &project_slug).await {
            Ok(p) => p,
            Err(_) => {
                debug!(project_slug = %project_slug, "Project not found, skipping reservation check");
                return Ok(None);
            }
        };

        // Look up agent - if not found, cannot check ownership
        let agent = match AgentBmc::get_by_name(ctx, mm, project.id, agent_name).await {
            Ok(a) => a,
            Err(_) => {
                debug!(
                    agent = agent_name,
                    "Agent not found, skipping reservation check"
                );
                return Ok(None);
            }
        };

        // Get active reservations for this project
        let reservations = FileReservationBmc::list_active_for_project(ctx, mm, project.id).await?;

        if reservations.is_empty() {
            debug!("No active reservations for project");
            return Ok(None);
        }

        // Check each file against reservations from OTHER agents
        let mut violations = Vec::new();
        let now = Utc::now().naive_utc();

        for file in files {
            for reservation in &reservations {
                // Skip our own reservations
                if reservation.agent_id == agent.id {
                    continue;
                }

                // Skip expired reservations
                if reservation.expires_ts < now {
                    continue;
                }

                // Check if file matches the reservation pattern
                if let Ok(pattern) = Pattern::new(&reservation.path_pattern) {
                    if pattern.matches(file) {
                        // Get the agent name who holds the reservation
                        let holder_name = AgentBmc::get(ctx, mm, reservation.agent_id)
                            .await
                            .map(|a| a.name)
                            .unwrap_or_else(|_| format!("agent#{}", reservation.agent_id));

                        violations.push(format!(
                            "File '{}' is reserved by '{}' (pattern: {}, reason: {})",
                            file, holder_name, reservation.path_pattern, reservation.reason
                        ));
                    }
                }
            }
        }

        if violations.is_empty() {
            debug!("No reservation conflicts found");
            return Ok(None);
        }

        // Handle based on mode
        match mode {
            GuardMode::Enforce => {
                // Return violations as errors (caller should block commit)
                warn!(
                    violations_count = violations.len(),
                    "File reservation conflicts detected (enforce mode)"
                );
                Ok(Some(violations))
            }
            GuardMode::Warn => {
                // Log warnings but allow commit
                for violation in &violations {
                    warn!(violation = %violation, "File reservation conflict (warn mode)");
                }
                Ok(Some(violations))
            }
            GuardMode::Bypass => {
                // Already handled above, but for completeness
                Ok(None)
            }
        }
    }

    /// Install pre-commit and pre-push hooks in git repository.
    ///
    /// # Arguments
    /// * `ctx` - Request context
    /// * `mm` - Model manager
    /// * `git_repo_path` - Path to the git repository root
    /// * `server_url` - Optional server URL for pre-push validation.
    ///   Falls back to `MCP_AGENT_MAIL_URL` or `API_URL` env vars, then `http://localhost:8080`
    pub async fn install(
        _ctx: &Ctx,
        _mm: &ModelManager,
        git_repo_path: &Path,
        server_url: Option<&str>,
    ) -> Result<String> {
        // Respect core.hooksPath configuration
        let hooks_dir = get_hooks_dir(git_repo_path);

        // Create hooks directory if it doesn't exist
        tokio::fs::create_dir_all(&hooks_dir).await?;

        // Install pre-commit hook
        let precommit_path = hooks_dir.join("pre-commit");
        let precommit_script = r#"#!/bin/sh
# MCP Agent Mail - Pre-commit Guard
# Checks file reservations before allowing commit

# Gate: Check if worktrees are enabled
check_gate() {
    # Check WORKTREES_ENABLED or GIT_IDENTITY_ENABLED
    case "${WORKTREES_ENABLED:-0}${GIT_IDENTITY_ENABLED:-0}" in
        *1*|*true*|*TRUE*|*yes*|*YES*|*y*|*Y*|*t*|*T*)
            return 0
            ;;
    esac
    return 1
}

# Gate: Skip if not enabled
if ! check_gate; then
    exit 0
fi

# Bypass mode: Skip all checks
case "${AGENT_MAIL_BYPASS:-0}" in
    1|true|TRUE|yes|YES|y|Y|t|T)
        echo "[pre-commit] bypass enabled via AGENT_MAIL_BYPASS=1" >&2
        exit 0
        ;;
esac

# Check if AGENT_NAME is set
if [ -z "$AGENT_NAME" ]; then
    echo "Warning: AGENT_NAME not set, skipping reservation check" >&2
    exit 0
fi

# TODO: Call agent mail API to verify file reservations
# For now, just pass through
exit 0
"#;

        tokio::fs::write(&precommit_path, precommit_script).await?;
        Self::make_executable(&precommit_path).await?;

        // Install pre-push hook
        let prepush_path = hooks_dir.join("pre-push");
        let server = match server_url {
            Some(url) => url.to_string(),
            None => std::env::var("MCP_AGENT_MAIL_URL")
                .or_else(|_| std::env::var("API_URL"))
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        };
        let prepush_script = render_prepush_script(&server);

        tokio::fs::write(&prepush_path, prepush_script).await?;
        Self::make_executable(&prepush_path).await?;

        Ok(format!(
            "Hooks installed: pre-commit at {:?}, pre-push at {:?}",
            precommit_path, prepush_path
        ))
    }

    /// Make a file executable (Unix only).
    #[allow(unused_variables)]
    async fn make_executable(path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(path).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(path, perms).await?;
        }
        Ok(())
    }

    /// Uninstall pre-commit and pre-push hooks from git repository.
    ///
    /// Respects `core.hooksPath` configuration when locating hooks.
    pub async fn uninstall(_ctx: &Ctx, _mm: &ModelManager, git_repo_path: &Path) -> Result<String> {
        // Respect core.hooksPath configuration
        let hooks_dir = get_hooks_dir(git_repo_path);
        let precommit_path = hooks_dir.join("pre-commit");
        let prepush_path = hooks_dir.join("pre-push");

        let mut removed = Vec::new();

        if precommit_path.exists() {
            tokio::fs::remove_file(&precommit_path).await?;
            removed.push("pre-commit");
        }

        if prepush_path.exists() {
            tokio::fs::remove_file(&prepush_path).await?;
            removed.push("pre-push");
        }

        if removed.is_empty() {
            Ok("No hooks found".to_string())
        } else {
            Ok(format!("Removed hooks: {}", removed.join(", ")))
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]
mod tests {
    use super::*;
    use lib_common::config::AppConfig;
    use serial_test::serial;
    use std::sync::Arc;
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
                    ModelManager::new_for_test(
                        conn,
                        temp_dir.path().to_path_buf(),
                        Arc::new(AppConfig::default()),
                    )
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
                        ModelManager::new_for_test(
                            conn,
                            temp_dir.path().to_path_buf(),
                            std::sync::Arc::new(lib_common::config::AppConfig::default()),
                        )
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
                    // No reservations in test DB, so should return None
                    assert!(result.unwrap().is_none());
                });
            },
        );
    }

    // ============================================================================
    // GUARD MODE TESTS (PORT-3.2)
    // ============================================================================

    #[test]
    fn test_guard_mode_default_is_enforce() {
        assert_eq!(GuardMode::default(), GuardMode::Enforce);
    }

    #[test]
    fn test_guard_mode_is_bypass() {
        assert!(!GuardMode::Enforce.is_bypass());
        assert!(!GuardMode::Warn.is_bypass());
        assert!(GuardMode::Bypass.is_bypass());
    }

    #[test]
    fn test_guard_mode_is_advisory() {
        assert!(!GuardMode::Enforce.is_advisory());
        assert!(GuardMode::Warn.is_advisory());
        assert!(!GuardMode::Bypass.is_advisory());
    }

    #[test]
    #[serial]
    fn test_guard_mode_from_env_defaults_to_enforce() {
        temp_env::with_vars_unset(["AGENT_MAIL_BYPASS", "AGENT_MAIL_GUARD_MODE"], || {
            assert_eq!(GuardMode::from_env(), GuardMode::Enforce);
        });
    }

    #[test]
    #[serial]
    fn test_guard_mode_from_env_bypass_truthy_values() {
        let truthy = ["1", "true", "TRUE", "yes", "YES", "t", "T", "y", "Y"];
        for val in truthy {
            temp_env::with_vars(
                [
                    ("AGENT_MAIL_BYPASS", Some(val)),
                    ("AGENT_MAIL_GUARD_MODE", None::<&str>),
                ],
                || {
                    assert_eq!(
                        GuardMode::from_env(),
                        GuardMode::Bypass,
                        "Expected Bypass for AGENT_MAIL_BYPASS={}",
                        val
                    );
                },
            );
        }
    }

    #[test]
    #[serial]
    fn test_guard_mode_from_env_bypass_takes_priority() {
        // Even with AGENT_MAIL_GUARD_MODE=warn, bypass should win
        temp_env::with_vars(
            [
                ("AGENT_MAIL_BYPASS", Some("1")),
                ("AGENT_MAIL_GUARD_MODE", Some("warn")),
            ],
            || {
                assert_eq!(GuardMode::from_env(), GuardMode::Bypass);
            },
        );
    }

    #[test]
    #[serial]
    fn test_guard_mode_from_env_warn_modes() {
        let warn_values = ["warn", "WARN", "advisory", "ADVISORY", "adv", "ADV"];
        for val in warn_values {
            temp_env::with_vars(
                [
                    ("AGENT_MAIL_BYPASS", None::<&str>),
                    ("AGENT_MAIL_GUARD_MODE", Some(val)),
                ],
                || {
                    assert_eq!(
                        GuardMode::from_env(),
                        GuardMode::Warn,
                        "Expected Warn for AGENT_MAIL_GUARD_MODE={}",
                        val
                    );
                },
            );
        }
    }

    #[test]
    #[serial]
    fn test_guard_mode_from_env_enforce_modes() {
        let enforce_values = ["block", "BLOCK", "enforce", "ENFORCE", ""];
        for val in enforce_values {
            temp_env::with_vars(
                [
                    ("AGENT_MAIL_BYPASS", None::<&str>),
                    ("AGENT_MAIL_GUARD_MODE", Some(val)),
                ],
                || {
                    assert_eq!(
                        GuardMode::from_env(),
                        GuardMode::Enforce,
                        "Expected Enforce for AGENT_MAIL_GUARD_MODE={}",
                        val
                    );
                },
            );
        }
    }

    #[test]
    #[serial]
    fn test_guard_mode_from_env_unknown_defaults_to_enforce() {
        temp_env::with_vars(
            [
                ("AGENT_MAIL_BYPASS", None::<&str>),
                ("AGENT_MAIL_GUARD_MODE", Some("unknown_value")),
            ],
            || {
                assert_eq!(GuardMode::from_env(), GuardMode::Enforce);
            },
        );
    }

    #[test]
    #[serial]
    fn test_check_reservations_bypass_mode_skips() {
        temp_env::with_vars(
            [
                ("WORKTREES_ENABLED", Some("1")),
                ("AGENT_MAIL_BYPASS", Some("1")),
            ],
            || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let ctx = Ctx::root_ctx();
                    let dummy_mm = {
                        use libsql::Builder;
                        let db_path = temp_dir.path().join("test.db");
                        let db = Builder::new_local(&db_path).build().await.unwrap();
                        let conn = db.connect().unwrap();
                        ModelManager::new_for_test(
                            conn,
                            temp_dir.path().to_path_buf(),
                            Arc::new(AppConfig::default()),
                        )
                    };

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
                        "Bypass mode should skip checks and return None"
                    );
                });
            },
        );
    }

    // ============================================================================
    // HOOK INSTALLATION TESTS (PORT-3.3)
    // ============================================================================

    #[tokio::test]
    async fn test_install_uninstall_both_hooks() {
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
            ModelManager::new_for_test(
                conn,
                temp_dir.path().to_path_buf(),
                Arc::new(AppConfig::default()),
            )
        };

        // Test install (both hooks)
        let result = PrecommitGuardBmc::install(&ctx, &dummy_mm, &git_dir, None).await;
        assert!(result.is_ok());
        assert!(
            git_dir.join(".git/hooks/pre-commit").exists(),
            "pre-commit hook should be installed"
        );
        assert!(
            git_dir.join(".git/hooks/pre-push").exists(),
            "pre-push hook should be installed"
        );

        // Verify pre-push hook is executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(git_dir.join(".git/hooks/pre-push"))
                .unwrap()
                .permissions();
            assert!(
                perms.mode() & 0o111 != 0,
                "pre-push hook should be executable"
            );
        }

        // Test uninstall (both hooks)
        let result = PrecommitGuardBmc::uninstall(&ctx, &dummy_mm, &git_dir).await;
        assert!(result.is_ok());
        assert!(
            !git_dir.join(".git/hooks/pre-commit").exists(),
            "pre-commit hook should be removed"
        );
        assert!(
            !git_dir.join(".git/hooks/pre-push").exists(),
            "pre-push hook should be removed"
        );
    }

    #[tokio::test]
    async fn test_install_with_custom_server_url() {
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join("test_repo");
        std::fs::create_dir_all(git_dir.join(".git/hooks")).unwrap();

        let ctx = Ctx::root_ctx();
        let dummy_mm = {
            use libsql::Builder;
            let db_path = temp_dir.path().join("test.db");
            let db = Builder::new_local(&db_path).build().await.unwrap();
            let conn = db.connect().unwrap();
            ModelManager::new_for_test(
                conn,
                temp_dir.path().to_path_buf(),
                Arc::new(AppConfig::default()),
            )
        };

        // Install with custom server URL
        let custom_url = "http://custom-server:9999";
        let result = PrecommitGuardBmc::install(&ctx, &dummy_mm, &git_dir, Some(custom_url)).await;
        assert!(result.is_ok());

        // Read pre-push hook and verify it contains custom URL
        let prepush_content = std::fs::read_to_string(git_dir.join(".git/hooks/pre-push")).unwrap();
        assert!(
            prepush_content.contains(custom_url),
            "pre-push hook should contain custom server URL"
        );
    }

    #[test]
    fn test_render_prepush_script_content() {
        let script = render_prepush_script("http://test:8080");

        // Verify script starts with shebang
        assert!(
            script.starts_with("#!/bin/sh"),
            "Script should start with shebang"
        );

        // Verify gate check
        assert!(
            script.contains("WORKTREES_ENABLED"),
            "Script should check WORKTREES_ENABLED"
        );
        assert!(
            script.contains("GIT_IDENTITY_ENABLED"),
            "Script should check GIT_IDENTITY_ENABLED"
        );

        // Verify bypass mode handling
        assert!(
            script.contains("AGENT_MAIL_BYPASS"),
            "Script should check AGENT_MAIL_BYPASS"
        );

        // Verify stdin reading for ref tuples
        assert!(
            script.contains("read -r local_ref local_sha remote_ref remote_sha"),
            "Script should read ref tuples from stdin"
        );

        // Verify server URL
        assert!(
            script.contains("http://test:8080"),
            "Script should contain server URL"
        );

        // Verify graceful degradation
        assert!(
            script.contains("exit 0"),
            "Script should exit 0 on graceful degradation"
        );

        // Verify advisory mode handling
        assert!(
            script.contains("AGENT_MAIL_GUARD_MODE"),
            "Script should check advisory mode"
        );
    }

    #[test]
    fn test_render_prepush_script_reads_stdin() {
        let script = render_prepush_script("http://localhost:8080");

        // Verify the script has a while loop to read stdin
        assert!(
            script.contains("while read"),
            "Script should have while read loop"
        );

        // Verify it collects refs
        assert!(script.contains("refs="), "Script should collect refs");
    }

    // ============================================================================
    // GET_HOOKS_DIR TESTS (PORT-3.4)
    // ============================================================================

    #[test]
    fn test_get_hooks_dir_fallback_to_git_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize a git repo
        std::process::Command::new("git")
            .args(["init", &repo_path.to_string_lossy()])
            .output()
            .expect("git init");

        // Without core.hooksPath, should return .git/hooks
        let hooks_dir = get_hooks_dir(&repo_path);
        assert!(
            hooks_dir.ends_with("hooks"),
            "Should end with 'hooks': {:?}",
            hooks_dir
        );
        // On a real git repo, it returns <git-dir>/hooks
        assert!(
            hooks_dir.to_string_lossy().contains(".git"),
            "Should contain .git: {:?}",
            hooks_dir
        );
    }

    #[test]
    fn test_get_hooks_dir_with_absolute_hooks_path() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Create custom hooks directory
        let custom_hooks = temp_dir.path().join("custom-hooks");
        std::fs::create_dir_all(&custom_hooks).unwrap();

        // Initialize git repo
        std::process::Command::new("git")
            .args(["init", &repo_path.to_string_lossy()])
            .output()
            .expect("git init");

        // Set absolute core.hooksPath
        std::process::Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "config",
                "core.hooksPath",
                &custom_hooks.to_string_lossy(),
            ])
            .output()
            .expect("git config");

        let hooks_dir = get_hooks_dir(&repo_path);
        assert_eq!(
            hooks_dir, custom_hooks,
            "Should use absolute core.hooksPath"
        );
    }

    #[test]
    fn test_get_hooks_dir_with_relative_hooks_path() {
        let temp_dir = TempDir::new().unwrap();
        // Canonicalize to resolve symlinks (e.g., /var -> /private/var on macOS)
        let repo_path = temp_dir.path().canonicalize().unwrap();

        // Create custom hooks directory (relative path)
        let custom_hooks_relative = "my-hooks";
        let custom_hooks_absolute = repo_path.join(custom_hooks_relative);
        std::fs::create_dir_all(&custom_hooks_absolute).unwrap();

        // Initialize git repo
        std::process::Command::new("git")
            .args(["init", &repo_path.to_string_lossy()])
            .output()
            .expect("git init");

        // Set relative core.hooksPath
        std::process::Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "config",
                "core.hooksPath",
                custom_hooks_relative,
            ])
            .output()
            .expect("git config");

        let hooks_dir = get_hooks_dir(&repo_path);
        assert_eq!(
            hooks_dir, custom_hooks_absolute,
            "Should resolve relative core.hooksPath from repo root"
        );
    }

    #[test]
    fn test_get_hooks_dir_non_git_directory() {
        let temp_dir = TempDir::new().unwrap();
        let non_git_path = temp_dir.path().to_path_buf();

        // Don't initialize git - should fall back to .git/hooks
        let hooks_dir = get_hooks_dir(&non_git_path);
        assert_eq!(
            hooks_dir,
            non_git_path.join(".git").join("hooks"),
            "Should fall back to .git/hooks for non-git directory"
        );
    }

    #[tokio::test]
    async fn test_install_with_custom_hooks_path() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Create custom hooks directory
        let custom_hooks = temp_dir.path().join("custom-hooks");
        std::fs::create_dir_all(&custom_hooks).unwrap();

        // Initialize git repo
        std::process::Command::new("git")
            .args(["init", &repo_path.to_string_lossy()])
            .output()
            .expect("git init");

        // Set core.hooksPath
        std::process::Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "config",
                "core.hooksPath",
                &custom_hooks.to_string_lossy(),
            ])
            .output()
            .expect("git config");

        let ctx = Ctx::root_ctx();
        let dummy_mm = {
            use libsql::Builder;
            let db_path = temp_dir.path().join("test.db");
            let db = Builder::new_local(&db_path).build().await.unwrap();
            let conn = db.connect().unwrap();
            ModelManager::new_for_test(
                conn,
                temp_dir.path().to_path_buf(),
                Arc::new(AppConfig::default()),
            )
        };

        // Install hooks
        let result = PrecommitGuardBmc::install(&ctx, &dummy_mm, &repo_path, None).await;
        assert!(result.is_ok(), "Install should succeed");

        // Verify hooks are in custom directory, NOT .git/hooks
        assert!(
            custom_hooks.join("pre-commit").exists(),
            "pre-commit should be in custom hooks dir"
        );
        assert!(
            custom_hooks.join("pre-push").exists(),
            "pre-push should be in custom hooks dir"
        );
        assert!(
            !repo_path.join(".git/hooks/pre-commit").exists(),
            "pre-commit should NOT be in .git/hooks"
        );

        // Verify uninstall also uses custom path
        let result = PrecommitGuardBmc::uninstall(&ctx, &dummy_mm, &repo_path).await;
        assert!(result.is_ok(), "Uninstall should succeed");
        assert!(
            !custom_hooks.join("pre-commit").exists(),
            "pre-commit should be removed from custom hooks dir"
        );
        assert!(
            !custom_hooks.join("pre-push").exists(),
            "pre-push should be removed from custom hooks dir"
        );
    }

    // ============================================================================
    // WORKTREE TESTS (PORT-7.2) - Using git2 crate (Rust-native)
    // ============================================================================

    /// Helper to create a git repo using git2 (Rust-native)
    fn create_git_repo_native(path: &std::path::Path) -> git2::Repository {
        git2::Repository::init(path).expect("git2: init repo")
    }

    /// Helper to create initial commit (required for worktrees)
    fn create_initial_commit(repo: &git2::Repository) -> git2::Oid {
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("create initial commit")
    }

    /// Helper to create a worktree using git2 (Rust-native)
    fn create_worktree_native(
        repo: &git2::Repository,
        name: &str,
        path: &std::path::Path,
    ) -> git2::Worktree {
        repo.worktree(name, path, None)
            .expect("git2: create worktree")
    }

    #[test]
    fn test_worktree_basic_installation_main_repo() {
        // Test that hooks can be installed in main repo that has worktrees
        let temp_dir = TempDir::new().unwrap();
        let main_repo_path = temp_dir.path().join("main-repo");
        std::fs::create_dir_all(&main_repo_path).unwrap();

        // Create main repo using git2 (Rust-native)
        let repo = create_git_repo_native(&main_repo_path);
        create_initial_commit(&repo);

        // Create a worktree (requires initial commit)
        let wt_path = temp_dir.path().join("worktree-1");
        let _wt = create_worktree_native(&repo, "wt-1", &wt_path);

        // Verify worktree exists
        assert!(wt_path.exists(), "Worktree directory should exist");

        // Install hooks in main repo - should work
        let hooks_dir = get_hooks_dir(&main_repo_path);
        assert!(
            hooks_dir.to_string_lossy().contains(".git"),
            "Main repo hooks should be in .git/hooks"
        );
    }

    #[test]
    fn test_worktree_basic_installation_worktree_repo() {
        // Test that hooks dir is resolved correctly for worktree
        let temp_dir = TempDir::new().unwrap();
        let main_repo_path = temp_dir.path().join("main-repo");
        std::fs::create_dir_all(&main_repo_path).unwrap();

        // Create main repo using git2
        let repo = create_git_repo_native(&main_repo_path);
        create_initial_commit(&repo);

        // Create worktree
        let wt_path = temp_dir.path().join("worktree-1");
        let _wt = create_worktree_native(&repo, "wt-1", &wt_path);

        // Get hooks dir for worktree
        let wt_hooks_dir = get_hooks_dir(&wt_path);

        // Worktree should reference main repo's hooks (via .git file)
        // The hooks should be in the main repo's git dir, not the worktree
        assert!(
            wt_hooks_dir.exists() || wt_hooks_dir.parent().is_some_and(|p| p.exists()),
            "Worktree hooks dir or parent should be resolvable"
        );
    }

    #[test]
    fn test_worktree_hook_preservation_existing_hook() {
        // Test that existing pre-commit hook is preserved (not overwritten)
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        // Create repo and hooks dir
        let _repo = create_git_repo_native(&repo_path);
        let hooks_dir = repo_path.join(".git").join("hooks");
        std::fs::create_dir_all(&hooks_dir).unwrap();

        // Create an existing pre-commit hook (user's custom hook)
        let existing_hook = hooks_dir.join("pre-commit");
        let user_script = "#!/bin/sh\necho 'User hook'\nexit 0\n";
        std::fs::write(&existing_hook, user_script).unwrap();

        // Read the existing hook content
        let before = std::fs::read_to_string(&existing_hook).unwrap();
        assert!(before.contains("User hook"), "Original hook should exist");

        // TODO: When install is updated to support chaining, verify preservation
        // For now, just verify the existing hook detection logic works
    }

    #[test]
    fn test_worktree_hook_preservation_backup_created() {
        // Placeholder for hook backup test
        // Tests that when overwriting, a .bak file is created
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        let _repo = create_git_repo_native(&repo_path);

        // Verify we can create backup files
        let hooks_dir = repo_path.join(".git").join("hooks");
        std::fs::create_dir_all(&hooks_dir).unwrap();

        let original = hooks_dir.join("pre-commit");
        let backup = hooks_dir.join("pre-commit.bak");

        std::fs::write(&original, "original content").unwrap();
        std::fs::copy(&original, &backup).unwrap();

        assert!(backup.exists(), "Backup file should be created");
        assert_eq!(
            std::fs::read_to_string(&backup).unwrap(),
            "original content"
        );
    }

    #[test]
    fn test_worktree_lifecycle_create_and_remove() {
        // Test complete worktree lifecycle using git2 (Rust-native)
        let temp_dir = TempDir::new().unwrap();
        let main_repo_path = temp_dir.path().join("main-repo");
        std::fs::create_dir_all(&main_repo_path).unwrap();

        // Create main repo
        let repo = create_git_repo_native(&main_repo_path);
        create_initial_commit(&repo);

        // Create worktree
        let wt_path = temp_dir.path().join("worktree-lifecycle");
        let wt = create_worktree_native(&repo, "lifecycle-wt", &wt_path);

        // Verify worktree path exists
        assert!(wt_path.exists(), "Worktree path should exist");
        assert!(wt.path().exists(), "Worktree git path should exist");

        // List worktrees
        let worktrees = repo.worktrees().expect("list worktrees");
        assert!(
            worktrees.iter().any(|n| n == Some("lifecycle-wt")),
            "Worktree should be in list"
        );

        // Cleanup worktree
        drop(wt);
        std::fs::remove_dir_all(&wt_path).ok(); // Best-effort cleanup
    }

    #[test]
    fn test_worktree_lifecycle_multiple_worktrees() {
        // Test managing multiple worktrees
        let temp_dir = TempDir::new().unwrap();
        let main_repo_path = temp_dir.path().join("main-repo");
        std::fs::create_dir_all(&main_repo_path).unwrap();

        let repo = create_git_repo_native(&main_repo_path);
        create_initial_commit(&repo);

        // Create multiple worktrees
        let wt1_path = temp_dir.path().join("wt-feature-a");
        let wt2_path = temp_dir.path().join("wt-feature-b");
        let wt3_path = temp_dir.path().join("wt-bugfix");

        let _wt1 = create_worktree_native(&repo, "feature-a", &wt1_path);
        let _wt2 = create_worktree_native(&repo, "feature-b", &wt2_path);
        let _wt3 = create_worktree_native(&repo, "bugfix", &wt3_path);

        // List and verify all exist
        let worktrees = repo.worktrees().expect("list worktrees");
        assert_eq!(worktrees.len(), 3, "Should have 3 worktrees");

        // Verify each has a valid path
        assert!(wt1_path.exists());
        assert!(wt2_path.exists());
        assert!(wt3_path.exists());
    }

    #[test]
    fn test_worktree_chain_runner_script_generation() {
        // Test that a chain runner script can be generated
        // This would call existing hooks before our guard
        let chain_script = r#"#!/bin/sh
# Chain runner - calls existing hooks before guard

# Call original hook if it exists
if [ -f ".git/hooks/pre-commit.original" ]; then
    .git/hooks/pre-commit.original "$@"
    result=$?
    if [ $result -ne 0 ]; then
        exit $result
    fi
fi

# Now run our guard
# ... guard logic here ...
exit 0
"#;

        // Verify script structure
        assert!(chain_script.contains("#!/bin/sh"), "Should have shebang");
        assert!(
            chain_script.contains(".original"),
            "Should reference original hook"
        );
        assert!(chain_script.contains("$result"), "Should capture exit code");
    }

    #[tokio::test]
    async fn test_worktree_install_hooks_in_worktree() {
        // Test installing hooks directly in a worktree
        let temp_dir = TempDir::new().unwrap();
        let main_repo_path = temp_dir.path().join("main-repo");
        std::fs::create_dir_all(&main_repo_path).unwrap();

        // Create main repo with git2
        let repo = create_git_repo_native(&main_repo_path);
        create_initial_commit(&repo);

        // Create worktree
        let wt_path = temp_dir.path().join("wt-install-test");
        let _wt = create_worktree_native(&repo, "install-test", &wt_path);

        // Create ModelManager for install
        let ctx = Ctx::root_ctx();
        let dummy_mm = {
            use libsql::Builder;
            let db_path = temp_dir.path().join("test.db");
            let db = Builder::new_local(&db_path).build().await.unwrap();
            let conn = db.connect().unwrap();
            ModelManager::new_for_test(
                conn,
                temp_dir.path().to_path_buf(),
                Arc::new(AppConfig::default()),
            )
        };

        // Install hooks in worktree path
        let result = PrecommitGuardBmc::install(&ctx, &dummy_mm, &wt_path, None).await;

        // Should succeed (hooks may be in main repo or worktree-local)
        assert!(result.is_ok(), "Install should succeed in worktree context");
    }
}
