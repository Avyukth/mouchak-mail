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

/// Type alias for database connections.
///
/// Uses libsql's [`Connection`] for SQLite access.
pub type Db = Connection;

/// Git storage operations for audit logging.
pub mod git_store;

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
    // Ensure data directory exists
    let db_path = PathBuf::from("data/mcp_agent_mail.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let _db_url = format!("file:{}", db_path.display());
    let db = Builder::new_local(db_path).build().await?;
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
    // For local file, we just open it
    // This function signature might need adjustment for Turso remote later
    // For now, just reusing new_db_pool logic or similar
    let db = Builder::new_local(PathBuf::from("data/mcp_agent_mail.db"))
        .build()
        .await?;
    let conn = db.connect()?;
    Ok(conn)
}
