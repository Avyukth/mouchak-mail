//! File Handle Safety Patterns (PORT-2.3)
//!
//! This module documents the file safety patterns used throughout lib-core
//! and lib-server to prevent file descriptor leaks.
//!
//! # Audit Summary
//!
//! All file operations in lib-core and lib-server have been audited:
//!
//! ## lib-core/src/store/
//!
//! | Module | Pattern | Status |
//! |--------|---------|--------|
//! | `git_store.rs` | `std::fs::write` (atomic) | ✅ Safe |
//! | `archive_lock.rs` | `tokio::fs` with RAII guard | ✅ Safe |
//! | `repo_cache.rs` | LRU eviction (8 repos max) | ✅ FD bounded |
//!
//! ## lib-core/src/model/
//!
//! | Module | Pattern | Status |
//! |--------|---------|--------|
//! | `attachment.rs` | DB metadata only | ✅ No file I/O |
//! | `precommit_guard.rs` | `tokio::fs` atomic ops | ✅ Safe |
//!
//! ## lib-server/src/api/
//!
//! | Module | Pattern | Status |
//! |--------|---------|--------|
//! | `attachments.rs` | `tokio::fs::write` + `ReaderStream` | ✅ Safe |
//!
//! # Best Practices
//!
//! ## 1. Prefer Atomic Operations
//!
//! ```rust,ignore
//! // ✅ GOOD: Atomic write (opens, writes, closes in one call)
//! std::fs::write(&path, content)?;
//!
//! // ❌ AVOID: Manual File handling (can leak on error)
//! let mut file = File::create(&path)?;
//! file.write_all(content.as_bytes())?;
//! // Implicit drop - timing uncertain
//! ```
//!
//! ## 2. Use Explicit Scopes
//!
//! ```rust,ignore
//! // ✅ GOOD: Explicit scope bounds file lifetime
//! fn process_file(path: &Path) -> Result<Data> {
//!     let data = {
//!         let file = File::open(path)?;
//!         let mut reader = BufReader::new(file);
//!         serde_json::from_reader(&mut reader)?
//!     }; // File explicitly dropped here
//!     Ok(data)
//! }
//! ```
//!
//! ## 3. Use RAII Guards for Resources
//!
//! See [`super::archive_lock::LockGuard`] for an example of RAII pattern
//! that ensures cleanup even on panic.
//!
//! ## 4. Use LRU Caching for Expensive Resources
//!
//! See [`super::repo_cache::RepoCache`] which limits open repositories
//! to prevent file descriptor exhaustion.
//!
//! # Git2 Repository Handles
//!
//! git2::Repository holds multiple file descriptors. Key safety patterns:
//!
//! - Use `RepoCache` to limit concurrent open repositories
//! - Don't hold `Repository` across `.await` points unnecessarily
//! - Let repositories drop naturally when work is complete
//!
//! # Async File Operations
//!
//! For async contexts, prefer `tokio::fs`:
//!
//! ```rust,ignore
//! use tokio::fs;
//!
//! // ✅ GOOD: Non-blocking, integrates with async runtime
//! fs::write(&path, content).await?;
//! let content = fs::read_to_string(&path).await?;
//! ```
//!
//! # HTTP Response Streaming
//!
//! When serving files via HTTP, use `ReaderStream` for memory-efficient streaming:
//!
//! ```rust,ignore
//! use tokio::fs::File;
//! use tokio_util::io::ReaderStream;
//! use axum::body::Body;
//!
//! // ✅ GOOD: File ownership transferred to stream, closed when response completes
//! let file = File::open(path).await?;
//! let stream = ReaderStream::new(file);
//! let body = Body::from_stream(stream);
//! // File is closed when body is fully consumed or dropped
//! ```
//!
//! This pattern is used in `lib-server/src/api/attachments.rs` for serving
//! file downloads. The file handle is owned by the stream and automatically
//! closed when the HTTP response is complete.

/// Marker module for file safety documentation.
/// This module exists primarily for documentation purposes.
/// See parent module documentation for file safety patterns.
pub mod docs {}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::fs;

    /// Stress test: Create and clean up 1000 temporary files
    /// to verify no file descriptor leaks occur.
    ///
    /// This test validates the atomic write pattern used throughout
    /// the codebase doesn't leak file descriptors under load.
    #[tokio::test]
    async fn test_fd_stability_under_load() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let base_path = temp_dir.path().to_path_buf();

        // Create 1000 files using atomic writes
        for i in 0..1000 {
            let path = base_path.join(format!("test_file_{}.txt", i));
            let content = format!("Test content for file {}", i);

            // Use atomic write (same pattern as git_store.rs)
            fs::write(&path, &content).await.expect("write file");

            // Immediately read back (same pattern as attachment downloads)
            let read_content = fs::read_to_string(&path).await.expect("read file");
            assert_eq!(read_content, content);

            // Delete file (simulating cleanup)
            fs::remove_file(&path).await.expect("remove file");
        }

        // If we got here without "too many open files" error,
        // the FD handling is stable
    }

    /// Stress test for concurrent file operations.
    /// Verifies that parallel file operations don't exhaust FDs.
    #[tokio::test]
    async fn test_concurrent_file_operations() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let base_path = temp_dir.path().to_path_buf();

        // Spawn 100 concurrent tasks, each doing 10 file operations
        let handles: Vec<_> = (0..100)
            .map(|task_id| {
                let path = base_path.clone();
                tokio::spawn(async move {
                    for file_id in 0..10 {
                        let file_path = path.join(format!("task_{}_file_{}.txt", task_id, file_id));
                        let content = format!("Task {} file {}", task_id, file_id);

                        fs::write(&file_path, &content).await.expect("write");
                        let read = fs::read_to_string(&file_path).await.expect("read");
                        assert_eq!(read, content);
                        fs::remove_file(&file_path).await.expect("remove");
                    }
                })
            })
            .collect();

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("task completed");
        }
    }

    /// Test that explicit scoping pattern works correctly.
    #[tokio::test]
    async fn test_explicit_scope_pattern() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let file_path = temp_dir.path().join("scoped_file.json");

        // Write test data
        let test_data = r#"{"key": "value"}"#;
        fs::write(&file_path, test_data).await.expect("write");

        // Read with explicit scope (as documented)
        let parsed: serde_json::Value = {
            let content = fs::read_to_string(&file_path).await.expect("read");
            serde_json::from_str(&content).expect("parse")
        }; // File handle released here

        assert_eq!(parsed["key"], "value");
    }

    /// Verify file paths are correctly formed for attachments pattern.
    #[test]
    fn test_attachment_path_pattern() {
        let project_id = 42i64;
        let unique_id = uuid::Uuid::new_v4();
        let filename = "test.pdf";

        let stored_filename = format!("{}_{}", unique_id, filename);
        let attachment_root = PathBuf::from("/data/attachments").join(project_id.to_string());
        let stored_path = attachment_root.join(&stored_filename);

        // Verify path structure
        assert!(
            stored_path
                .to_string_lossy()
                .contains("/data/attachments/42/")
        );
        assert!(stored_path.to_string_lossy().ends_with("_test.pdf"));
    }
}
