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
pub mod attachment;
pub mod build_slot;
pub mod export;
pub mod file_reservation;
pub mod macro_def;
pub mod message;
pub mod message_recipient;
pub mod overseer_message;
pub mod precommit_guard;
pub mod product;
pub mod project;
pub mod project_sibling_suggestion;
pub mod tool_metric;

use crate::Result;
use crate::store::repo_cache::RepoCache;
use crate::store::{self, Db};
use git2::Repository;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Default LRU cache capacity for git repositories.
/// Each repo uses ~10-50 FDs, so 8 repos = ~400 FDs max.
const DEFAULT_REPO_CACHE_SIZE: usize = 8;

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
}

impl ModelManager {
    /// Constructor
    pub async fn new() -> Result<Self> {
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

        Ok(ModelManager {
            db,
            repo_root,
            git_lock: Arc::new(Mutex::new(())),
            repo_cache: Arc::new(RepoCache::new(cache_size)),
        })
    }

    /// Constructor for testing with custom db connection and paths
    /// This is public so integration tests can use it
    pub fn new_for_test(db: Db, repo_root: PathBuf) -> Self {
        ModelManager {
            db,
            repo_root,
            git_lock: Arc::new(Mutex::new(())),
            repo_cache: Arc::new(RepoCache::default()),
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
