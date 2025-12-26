//! Common test utilities and fixtures
//!
//! Provides test helpers for setting up isolated test environments.
//!
//! ## Design Principles (TDD/Production Hardening)
//! - Each test gets its own isolated database (unique file in temp dir)
//! - Tests run with sequential test threads to avoid git locking
//! - Cleanup happens automatically via TempDir RAII

#![allow(dead_code)]

use lib_common::config::AppConfig;
use lib_core::{Ctx, ModelManager, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tempfile::TempDir;

/// Global counter for unique database names
static DB_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Test context that manages temporary directories and database setup
///
/// Each test gets a unique database to avoid locking conflicts.
pub struct TestContext {
    pub mm: ModelManager,
    pub ctx: Ctx,
    #[allow(dead_code)]
    temp_dir: TempDir, // Keep alive for duration of test
}

impl TestContext {
    /// Create a new test context with isolated database and git repo
    /// Create a new test context with isolated database and git repo
    pub async fn new() -> Result<Self> {
        Self::new_with_config(AppConfig::default()).await
    }

    pub async fn new_with_config(config: AppConfig) -> Result<Self> {
        // Create unique temp directory for this test
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Get unique counter for database name
        let counter = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!("test_db_{}.db", counter);
        let db_path = temp_dir.path().join(&db_name);

        // Set the archive root to temp dir
        let archive_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&archive_root)?;

        // Create database connection
        let db = create_test_db(&db_path).await?;

        // Create ModelManager with test paths using the test constructor
        let app_config = Arc::new(config);
        let mm = ModelManager::new_for_test(db, archive_root, app_config);
        let ctx = Ctx::root_ctx();

        Ok(Self { mm, ctx, temp_dir })
    }

    /// Get the repo root path for testing
    #[allow(dead_code)]
    pub fn repo_root(&self) -> PathBuf {
        self.mm.repo_root.clone()
    }

    /// Explain the query plan for a given SQL string
    pub async fn explain_query_plan(&self, sql: &str) -> Result<Vec<String>> {
        let db = self.mm.db_for_test();
        let explain_sql = format!("EXPLAIN QUERY PLAN {}", sql);
        let mut rows = db.query(&explain_sql, ()).await?;
        let mut plans = Vec::new();
        while let Some(row) = rows.next().await? {
            // detail is the 4th column (index 3) in EXPLAIN QUERY PLAN
            let detail: String = row.get(3)?;
            plans.push(detail);
        }
        Ok(plans)
    }
}

/// Create an isolated database for testing
async fn create_test_db(db_path: &std::path::Path) -> Result<lib_core::store::Db> {
    use libsql::Builder;

    // Create parent directories
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Build database at custom path
    let db = Builder::new_local(db_path).build().await?;
    let conn = db.connect()?;

    // Apply migrations
    let _ = conn.execute("PRAGMA journal_mode=WAL;", ()).await;

    // Read schema from migrations directory (same path as store/mod.rs uses)
    let schema = include_str!("../../../../../migrations/001_initial_schema.sql");
    conn.execute_batch(schema).await?;
    let schema002 = include_str!("../../../../../migrations/002_agent_capabilities.sql");
    conn.execute_batch(schema002).await?;
    let schema003 = include_str!("../../../../../migrations/003_tool_metrics.sql");
    conn.execute_batch(schema003).await?;
    let schema004 = include_str!("../../../../../migrations/004_attachments.sql");
    conn.execute_batch(schema004).await?;
    let schema005 = include_str!("../../../../../migrations/005_attachments_agent.sql");
    conn.execute_batch(schema005).await?;
    let schema006 = include_str!("../../../../../migrations/006_query_indexes.sql");
    conn.execute_batch(schema006).await?;

    // Verify idempotency: running migrations again should not fail
    conn.execute_batch(schema).await?;
    conn.execute_batch(schema002).await?;
    conn.execute_batch(schema003).await?;
    conn.execute_batch(schema004).await?;
    conn.execute_batch(schema006).await?;

    Ok(conn)
}
