//! # Model Layer - BMC Pattern Implementation
//!
//! This module contains all Backend Model Controllers (BMCs) and their
//! associated data structures for the MCP Agent Mail system.
//!
//! ## BMC Pattern
//!
//! Each entity has:
//! - **Data Struct**: Serializable model (e.g., `Agent`, `Message`)
//! - **ForCreate Struct**: Input for creation operations
//! - **Bmc Struct**: Stateless controller with async CRUD methods
//!
//! ## Available Controllers
//!
//! | BMC | Description |
//! |-----|-------------|
//! | `agent::AgentBmc` | AI agent registration and profiles |
//! | `message::MessageBmc` | Inter-agent messaging |
//! | `project::ProjectBmc` | Project management |
//! | `file_reservation::FileReservationBmc` | File locking coordination |
//! | `build_slot::BuildSlotBmc` | CI/CD slot management |
//! | `macro_def::MacroDefBmc` | Workflow macro definitions |
//! | `attachment::AttachmentBmc` | File attachments |
//! | `activity::ActivityBmc` | Unified activity feed |
//! | `tool_metric::ToolMetricBmc` | Tool usage analytics |
//! | `overseer_message::OverseerMessageBmc` | Human escalation messages |
//!
//! ## ModelManager
//!
//! The [`ModelManager`] provides centralized access to:
//! - Database connections (libSQL)
//! - Git repository operations
//! - Concurrency control via `git_lock`

pub mod activity;
pub mod agent;
pub mod agent_capabilities;
pub mod agent_link;
pub mod archive_browser;
pub mod attachment;
pub mod build_slot;
pub mod escalation;
pub mod export;
pub mod file_reservation;
pub mod identity;
pub mod macro_def;
pub mod message;
pub mod message_recipient;
pub mod orchestration;
pub mod overseer_message;
pub mod precommit_guard;
pub mod product;
pub mod project;
pub mod project_sibling_suggestion;
pub mod time_travel;
pub mod tool_metric;

use crate::Result;
use crate::store::archive_lock::{ArchiveLock, LockGuard};
use crate::store::repo_cache::RepoCache;
use crate::store::{self, Db};
use git2::Repository;
use lib_common::config::AppConfig;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Default LRU cache capacity for git repositories.
/// Each repo uses ~10-50 FDs, so 8 repos = ~400 FDs max.
const DEFAULT_REPO_CACHE_SIZE: usize = 8;

/// Default archive lock timeout in seconds
const DEFAULT_ARCHIVE_LOCK_TIMEOUT_SECS: u64 = 30;

#[derive(Clone)]
pub struct ModelManager {
    pub(crate) db: Db,
    pub repo_root: PathBuf,
    /// Mutex to serialize git operations - git2's index locking doesn't handle
    /// high concurrency well, so we serialize commits at the application level.
    /// This is critical for supporting 100+ concurrent agents.
    pub git_lock: Arc<Mutex<()>>,
    /// LRU cache for git repositories to prevent file descriptor exhaustion.
    /// NIST Control: SC-5 (DoS Protection)
    repo_cache: Arc<RepoCache>,
    /// File-based advisory lock for cross-process coordination.
    /// Handles stale lock cleanup from crashed processes.
    /// NIST Control: AU-9 (Audit Log Protection)
    archive_lock: Arc<ArchiveLock>,
    /// Application configuration.
    pub app_config: Arc<AppConfig>,
}

impl ModelManager {
    /// Constructor
    pub async fn new(app_config: Arc<AppConfig>) -> Result<Self> {
        let db = store::new_db_pool().await?;
        // Default to "data/archive" for now, similar to Python's default or configurable
        let repo_root = std::env::current_dir()?.join("data").join("archive");
        std::fs::create_dir_all(&repo_root)?;

        // Auto-initialize git repository if not exists
        crate::store::git_store::init_or_open_repo(&repo_root)?;

        // Read cache size from env, default to 8
        let cache_size = std::env::var("GIT_REPO_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_REPO_CACHE_SIZE);

        // Initialize archive lock and cleanup any stale locks from crashed processes
        let archive_lock = Arc::new(ArchiveLock::new(&repo_root));
        Self::cleanup_stale_locks(&archive_lock).await;

        Ok(ModelManager {
            db,
            repo_root,
            git_lock: Arc::new(Mutex::new(())),
            repo_cache: Arc::new(RepoCache::new(cache_size)),
            archive_lock,
            app_config,
        })
    }

    /// Constructor for testing with custom db connection and paths
    /// This is public so integration tests can use it
    pub fn new_for_test(db: Db, repo_root: PathBuf, app_config: Arc<AppConfig>) -> Self {
        let archive_lock = Arc::new(ArchiveLock::new(&repo_root));
        ModelManager {
            db,
            repo_root: repo_root.clone(),
            git_lock: Arc::new(Mutex::new(())),
            repo_cache: Arc::new(RepoCache::default()),
            archive_lock,
            app_config,
        }
    }

    /// Cleanup stale locks from crashed processes on startup.
    /// NIST Control: AU-9 (Audit Log Protection)
    async fn cleanup_stale_locks(archive_lock: &ArchiveLock) {
        // Try to acquire lock with short timeout - if we get it, no stale lock
        // If we timeout but the lock is from a dead process, it will be cleaned
        let timeout = std::time::Duration::from_millis(100);
        match archive_lock
            .acquire(Some("startup-cleanup".into()), timeout)
            .await
        {
            Ok(_guard) => {
                // Lock acquired and released - no stale lock present
                info!("Archive lock check passed - no stale locks");
            }
            Err(crate::Error::LockTimeout { path, owner_pid }) => {
                // Lock held by another process - check if it's stale
                info!(
                    path = %path,
                    pid = owner_pid,
                    "Archive lock held by another process, will be cleaned if stale"
                );
            }
            Err(e) => {
                // Other error - log and continue
                info!(error = %e, "Error checking archive lock on startup");
            }
        }
    }

    /// Get a cached repository handle for the repo_root.
    ///
    /// Uses LRU cache to prevent file descriptor exhaustion.
    /// The returned `Arc<Mutex<Repository>>` must be locked before use.
    ///
    /// # Example
    /// ```ignore
    /// let repo_arc = mm.get_repo().await?;
    /// let repo = repo_arc.lock().await;
    /// git_store::commit_file(&*repo, ...)?;
    /// ```
    pub async fn get_repo(&self) -> Result<Arc<Mutex<Repository>>> {
        self.repo_cache.get(&self.repo_root).await
    }

    /// Acquire advisory file lock for longer archive operations.
    ///
    /// The returned guard automatically releases the lock when dropped.
    /// Use this for operations that span multiple git commits or need
    /// cross-process coordination.
    ///
    /// # Arguments
    /// * `agent` - Optional agent name for lock ownership tracking
    ///
    /// # Example
    /// ```ignore
    /// let _guard = mm.acquire_archive_lock(Some("export-agent")).await?;
    /// // ... perform multiple git operations ...
    /// // Lock automatically released when _guard drops
    /// ```
    pub async fn acquire_archive_lock(&self, agent: Option<String>) -> Result<LockGuard<'_>> {
        let timeout = std::time::Duration::from_secs(DEFAULT_ARCHIVE_LOCK_TIMEOUT_SECS);
        self.archive_lock.acquire(agent, timeout).await
    }

    /// Returns the sqlx db pool reference.
    /// (Only for the model layer)
    pub(in crate::model) fn db(&self) -> &Db {
        &self.db
    }

    /// Returns the db connection for integration tests
    /// This should only be used in test code
    pub fn db_for_test(&self) -> &Db {
        &self.db
    }

    /// Health check - verify database connectivity
    pub async fn health_check(&self) -> Result<bool> {
        let stmt = self.db.prepare("SELECT 1").await?;
        let mut rows = stmt.query(()).await?;
        Ok(rows.next().await?.is_some())
    }
}
