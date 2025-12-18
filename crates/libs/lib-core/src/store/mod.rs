//! Low-level storage operations for database and Git.
//!
//! This module provides the storage layer for lib-core, handling:
//!
//! - **Database connections**: SQLite via libsql with optimized settings
//! - **Git storage**: Audit trail for entities via git2
//!
//! # Architecture
//!
//! All data is stored in two places:
//! 1. **SQLite database** (`data/mcp_agent_mail.db`) - Primary storage for queries
//! 2. **Git repository** - Audit log for entity changes (via `git_store` submodule)
//!
//! # Database Path Resolution
//!
//! The database path is resolved in this order:
//! 1. `DATABASE_PATH` environment variable (absolute path)
//! 2. Relative to `CARGO_WORKSPACE_DIR` if set (for cargo run)
//! 3. Walk up from current directory to find Cargo.toml with `[workspace]`
//! 4. Fall back to current working directory
//!
//! This ensures the same database is used regardless of which directory
//! commands are run from.
//!
//! # Database Configuration
//!
//! The database is configured for high-concurrency scenarios:
//! - WAL mode for concurrent reads during writes
//! - 30-second busy timeout for lock contention
//! - 64MB cache for reduced I/O
//!
//! # Example
//!
//! ```no_run
//! use lib_core::store::new_db_pool;
//!
//! async fn setup() -> lib_core::Result<()> {
//!     let db = new_db_pool().await?;
//!     // Database is ready with migrations applied
//!     Ok(())
//! }
//! ```

use crate::Result;
use libsql::{Builder, Connection};
use std::path::PathBuf;

/// Resolves the database path, ensuring consistency regardless of CWD.
///
/// Resolution order:
/// 1. `DATABASE_PATH` env var (absolute path)
/// 2. `CARGO_WORKSPACE_DIR` env var + "data/mcp_agent_mail.db"
/// 3. Walk up directories to find workspace root (contains Cargo.toml with [workspace])
/// 4. Fall back to CWD + "data/mcp_agent_mail.db"
fn resolve_db_path() -> PathBuf {
    // 1. Check for explicit DATABASE_PATH
    if let Ok(path) = std::env::var("DATABASE_PATH") {
        let p = PathBuf::from(&path);
        tracing::info!("Using DATABASE_PATH: {}", p.display());
        return p;
    }

    // 2. Check for CARGO_WORKSPACE_DIR (set by some cargo configurations)
    if let Ok(workspace_dir) = std::env::var("CARGO_WORKSPACE_DIR") {
        let p = PathBuf::from(workspace_dir)
            .join("data")
            .join("mcp_agent_mail.db");
        tracing::info!("Using CARGO_WORKSPACE_DIR: {}", p.display());
        return p;
    }

    // 3. Walk up to find workspace root (Cargo.toml with [workspace])
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd.as_path();
        loop {
            let cargo_toml = dir.join("Cargo.toml");
            if cargo_toml.exists() {
                // Check if this is the workspace root
                if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                    if contents.contains("[workspace]") {
                        let p = dir.join("data").join("mcp_agent_mail.db");
                        tracing::info!("Found workspace root, using: {}", p.display());
                        return p;
                    }
                }
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }

        // 4. Fall back to CWD
        let p = cwd.join("data").join("mcp_agent_mail.db");
        tracing::warn!("No workspace root found, using CWD: {}", p.display());
        return p;
    }

    // Ultimate fallback
    PathBuf::from("data/mcp_agent_mail.db")
}

/// Type alias for database connections.
///
/// Uses libsql's [`Connection`] for SQLite access.
pub type Db = Connection;

/// Git storage operations for audit logging.
pub mod git_store;

/// LRU cache for repository handles (PORT-2.1, SC-5 DoS protection).
pub mod repo_cache;

/// Stale lock cleanup for archive operations (PORT-2.2, AU-9 audit protection).
pub mod archive_lock;

/// Creates a new database connection pool with migrations applied.
///
/// This function:
/// 1. Creates the `data/` directory if needed
/// 2. Opens or creates the SQLite database
/// 3. Applies concurrency optimizations (WAL, timeouts, cache)
/// 4. Runs all migrations
///
/// # Returns
///
/// A configured database connection ready for use.
///
/// # Errors
///
/// Returns an error if:
/// - Directory creation fails
/// - Database cannot be opened
/// - Migrations fail
///
/// # Example
///
/// ```no_run
/// use lib_core::store::new_db_pool;
///
/// # async fn example() -> lib_core::Result<()> {
/// let db = new_db_pool().await?;
/// # Ok(())
/// # }
/// ```
pub async fn new_db_pool() -> Result<Db> {
    // Resolve database path (handles CWD-independence)
    let db_path = resolve_db_path();

    // Ensure data directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    tracing::info!("Opening database at: {}", db_path.display());
    let db = Builder::new_local(&db_path).build().await?;
    let conn = db.connect()?;

    // SQLite concurrency optimizations for high-load scenarios
    // WAL mode: enables concurrent reads during writes
    let _ = conn.execute("PRAGMA journal_mode=WAL;", ()).await;
    // busy_timeout: wait up to 30 seconds when database is locked (instead of failing immediately)
    // This is critical for 100+ concurrent agents writing simultaneously
    let _ = conn.execute("PRAGMA busy_timeout=30000;", ()).await;
    // synchronous=NORMAL: good balance of safety and performance with WAL
    let _ = conn.execute("PRAGMA synchronous=NORMAL;", ()).await;
    // cache_size: increase cache to reduce disk I/O (negative = KB, so -64000 = 64MB)
    let _ = conn.execute("PRAGMA cache_size=-64000;", ()).await;

    // Apply all migrations in order
    // Note: SQLite's IF NOT EXISTS makes this idempotent for table creation
    let migrations = [
        include_str!("../../../../../migrations/001_initial_schema.sql"),
        include_str!("../../../../../migrations/002_agent_capabilities.sql"),
        include_str!("../../../../../migrations/003_tool_metrics.sql"),
        include_str!("../../../../../migrations/004_attachments.sql"),
    ];

    for migration in &migrations {
        conn.execute_batch(migration).await?;
    }

    Ok(conn)
}

/// Gets a database connection for executing queries.
///
/// This is a helper function for obtaining a connection to the local database.
/// Currently uses the same local file as [`new_db_pool`].
///
/// # Arguments
///
/// * `_db_url` - Database URL (currently ignored, reserved for future Turso support)
///
/// # Returns
///
/// A database connection.
///
/// # Note
///
/// This function signature is designed for future Turso remote database support.
/// Currently it opens the local SQLite file regardless of the URL parameter.
pub async fn get_db_connection(_db_url: &str) -> Result<Connection> {
    // For local file, we just open it using the resolved path
    // This function signature might need adjustment for Turso remote later
    let db_path = resolve_db_path();
    let db = Builder::new_local(&db_path).build().await?;
    let conn = db.connect()?;
    Ok(conn)
}
