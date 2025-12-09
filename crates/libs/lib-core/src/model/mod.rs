pub mod agent;
pub mod message;
pub mod project;
pub mod file_reservation;
pub mod product;
pub mod message_recipient;
pub mod agent_link;
pub mod project_sibling_suggestion;
pub mod build_slot;
pub mod overseer_message;
pub mod macro_def;
pub mod export;

use crate::store::{self, Db};
use crate::Result;
use std::path::PathBuf;

#[derive(Clone)]
pub struct ModelManager {
    pub(crate) db: Db,
    pub repo_root: PathBuf,
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

        Ok(ModelManager { db, repo_root })
    }

    /// Constructor for testing with custom db connection and paths
    /// This is public so integration tests can use it
    pub fn new_for_test(db: Db, repo_root: PathBuf) -> Self {
        ModelManager { db, repo_root }
    }

    /// Returns the sqlx db pool reference.
    /// (Only for the model layer)
    pub(in crate::model) fn db(&self) -> &Db {
        &self.db
    }

    /// Health check - verify database connectivity
    pub async fn health_check(&self) -> Result<bool> {
        let stmt = self.db.prepare("SELECT 1").await?;
        let mut rows = stmt.query(()).await?;
        Ok(rows.next().await?.is_some())
    }
}