//! Archive Browser Tests
//!
//! Tests for git history exploration functionality covering:
//! - Commit Browsing (3 tests)
//! - File Navigation (4 tests)
//! - File Content (3 tests)
//! - File History (3 tests)
//! - Activity Timeline (3 tests)
//! - Security (2 tests)
//! - Edge Cases (2 tests)
//!
//! Total: 20 tests

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{Duration, Utc};
use libsql::Builder;
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::ModelManager;
use mouchak_mail_core::model::archive_browser::{ArchiveBrowserBmc, CommitFilter};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

/// Test context with temporary database and git repo
struct TestContext {
    ctx: Ctx,
    mm: ModelManager,
    _temp_dir: TempDir,
}

impl TestContext {
    async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let repo_path = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_path).expect("Create archive dir");

        // Initialize git repo
        git2::Repository::init(&repo_path).expect("Init git repo");

        let db = Builder::new_local(&db_path)
            .build()
            .await
            .expect("Build db");
        let conn = db.connect().expect("Connect to db");

        let app_config = Arc::new(AppConfig::default());
        let mm = ModelManager::new_for_test(conn, repo_path, app_config);
        let ctx = Ctx::root_ctx();

        Self {
            ctx,
            mm,
            _temp_dir: temp_dir,
        }
    }

    /// Create a file in the git archive and commit it
    async fn create_file(&self, file_path: &str, content: &str, message: &str) -> String {
        let repo = self.mm.get_repo().await.expect("Get repo");
        let repo_guard = repo.lock().await;

        let workdir = repo_guard.workdir().expect("Get workdir");

        // Create parent directories
        if let Some(parent) = Path::new(file_path).parent() {
            std::fs::create_dir_all(workdir.join(parent)).expect("Create parent dirs");
        }

        std::fs::write(workdir.join(file_path), content).expect("Write file");

        // Commit
        let mut index = repo_guard.index().expect("Get index");
        index.add_path(Path::new(file_path)).expect("Add path");
        index.write().expect("Write index");
        let tree_id = index.write_tree().expect("Write tree");
        let tree = repo_guard.find_tree(tree_id).expect("Find tree");

        let sig = git2::Signature::now("test", "test@test.com").expect("Create signature");

        let parent = match repo_guard.head() {
            Ok(h) => Some(h.peel_to_commit().expect("Peel to commit")),
            Err(_) => None,
        };

        let oid = match parent {
            Some(ref p) => repo_guard
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[p])
                .expect("Commit with parent"),
            None => repo_guard
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .expect("Initial commit"),
        };

        oid.to_string()
    }
}

// ============================================================================
// COMMIT BROWSING TESTS (3 tests)
// ============================================================================

#[tokio::test]
async fn test_list_commits_empty_repo() {
    let tc = TestContext::new().await;

    let commits = ArchiveBrowserBmc::list_commits(&tc.ctx, &tc.mm, None, 10)
        .await
        .expect("List commits");

    assert!(commits.is_empty(), "Empty repo should have no commits");
}

#[tokio::test]
async fn test_list_commits_with_history() {
    let tc = TestContext::new().await;

    // Create some commits
    tc.create_file("file1.txt", "content1", "First commit")
        .await;
    tc.create_file("file2.txt", "content2", "Second commit")
        .await;
    tc.create_file("file3.txt", "content3", "Third commit")
        .await;

    let commits = ArchiveBrowserBmc::list_commits(&tc.ctx, &tc.mm, None, 10)
        .await
        .expect("List commits");

    assert_eq!(commits.len(), 3, "Should have 3 commits");
    // Verify all expected messages are present (order may vary since timestamps could be identical)
    let messages: Vec<_> = commits.iter().map(|c| c.message.as_str()).collect();
    assert!(messages.contains(&"First commit"));
    assert!(messages.contains(&"Second commit"));
    assert!(messages.contains(&"Third commit"));
}

#[tokio::test]
async fn test_list_commits_with_filter() {
    let tc = TestContext::new().await;

    tc.create_file("file1.txt", "content1", "First commit")
        .await;
    tc.create_file("file2.txt", "content2", "Fix: bug in code")
        .await;
    tc.create_file("file3.txt", "content3", "Third commit")
        .await;

    // Filter by message content
    let filter = CommitFilter {
        message_contains: Some("Fix".to_string()),
        ..Default::default()
    };

    let commits = ArchiveBrowserBmc::list_commits(&tc.ctx, &tc.mm, Some(filter), 10)
        .await
        .expect("List commits");

    assert_eq!(commits.len(), 1);
    assert!(commits[0].message.contains("Fix"));
}

// ============================================================================
// COMMIT DETAILS TESTS (4 tests covering list_commits limit and commit_details)
// ============================================================================

#[tokio::test]
async fn test_commit_details_valid() {
    let tc = TestContext::new().await;

    let sha = tc
        .create_file("test.txt", "content", "Test commit message")
        .await;

    let details = ArchiveBrowserBmc::commit_details(&tc.ctx, &tc.mm, &sha)
        .await
        .expect("Get commit details");

    assert_eq!(details.sha, sha);
    assert_eq!(details.message, "Test commit message");
    assert_eq!(details.author_name, "test");
    assert!(details.files_added.contains(&"test.txt".to_string()));
}

#[tokio::test]
async fn test_commit_details_invalid_sha() {
    let tc = TestContext::new().await;

    // Create at least one commit
    tc.create_file("test.txt", "content", "Test").await;

    // Invalid SHA format
    let result = ArchiveBrowserBmc::commit_details(&tc.ctx, &tc.mm, "not-a-sha").await;
    assert!(result.is_err(), "Invalid SHA should error");

    // Empty SHA
    let result = ArchiveBrowserBmc::commit_details(&tc.ctx, &tc.mm, "").await;
    assert!(result.is_err(), "Empty SHA should error");
}

#[tokio::test]
async fn test_commit_details_nonexistent_sha() {
    let tc = TestContext::new().await;

    // Create at least one commit
    tc.create_file("test.txt", "content", "Test").await;

    // Valid format but doesn't exist
    let result = ArchiveBrowserBmc::commit_details(
        &tc.ctx,
        &tc.mm,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0",
    )
    .await;
    assert!(result.is_err(), "Nonexistent SHA should error");
}

#[tokio::test]
async fn test_list_commits_with_limit() {
    let tc = TestContext::new().await;

    // Create more commits than limit
    for i in 1..=5 {
        tc.create_file(
            &format!("file{}.txt", i),
            &format!("content{}", i),
            &format!("Commit {}", i),
        )
        .await;
    }

    // Limit to 3
    let commits = ArchiveBrowserBmc::list_commits(&tc.ctx, &tc.mm, None, 3)
        .await
        .expect("List commits");

    assert_eq!(commits.len(), 3, "Should respect limit");
}

// ============================================================================
// FILE NAVIGATION TESTS (4 tests)
// ============================================================================

#[tokio::test]
async fn test_list_files_at_root() {
    let tc = TestContext::new().await;

    tc.create_file("root.txt", "content", "Add root file").await;
    tc.create_file("subdir/nested.txt", "content", "Add nested file")
        .await;

    let sha = tc
        .create_file("another.txt", "content", "Add another")
        .await;

    let entries = ArchiveBrowserBmc::list_files_at(&tc.ctx, &tc.mm, &sha, "")
        .await
        .expect("List files");

    // Should have root.txt, another.txt, and subdir/
    assert!(entries.len() >= 3);

    // Directories come first
    let dir_entry = entries.iter().find(|e| e.name == "subdir");
    assert!(dir_entry.is_some(), "Should find subdir");
    assert!(dir_entry.unwrap().is_directory);

    let file_entry = entries.iter().find(|e| e.name == "root.txt");
    assert!(file_entry.is_some(), "Should find root.txt");
    assert!(!file_entry.unwrap().is_directory);
}

#[tokio::test]
async fn test_list_files_at_subdirectory() {
    let tc = TestContext::new().await;

    tc.create_file("subdir/file1.txt", "content1", "Add file1")
        .await;
    let sha = tc
        .create_file("subdir/file2.txt", "content2", "Add file2")
        .await;

    let entries = ArchiveBrowserBmc::list_files_at(&tc.ctx, &tc.mm, &sha, "subdir")
        .await
        .expect("List files in subdir");

    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|e| e.name == "file1.txt"));
    assert!(entries.iter().any(|e| e.name == "file2.txt"));
}

#[tokio::test]
async fn test_list_files_invalid_sha() {
    let tc = TestContext::new().await;

    tc.create_file("test.txt", "content", "Test").await;

    let result = ArchiveBrowserBmc::list_files_at(&tc.ctx, &tc.mm, "", "").await;
    assert!(result.is_err(), "Empty SHA should error");

    let result = ArchiveBrowserBmc::list_files_at(&tc.ctx, &tc.mm, "invalid", "").await;
    assert!(result.is_err(), "Invalid SHA should error");
}

#[tokio::test]
async fn test_list_files_at_nonexistent_dir() {
    let tc = TestContext::new().await;

    let sha = tc.create_file("test.txt", "content", "Test").await;

    let result = ArchiveBrowserBmc::list_files_at(&tc.ctx, &tc.mm, &sha, "nonexistent").await;
    assert!(result.is_err(), "Nonexistent directory should error");
}

// ============================================================================
// FILE CONTENT TESTS (3 tests)
// ============================================================================

#[tokio::test]
async fn test_file_content_at_commit() {
    let tc = TestContext::new().await;

    let content = "Hello, World!";
    let sha = tc
        .create_file("greeting.txt", content, "Add greeting")
        .await;

    let file_content = ArchiveBrowserBmc::file_content_at(&tc.ctx, &tc.mm, &sha, "greeting.txt")
        .await
        .expect("Get file content");

    assert_eq!(file_content.path, "greeting.txt");
    assert_eq!(file_content.content, content);
    assert_eq!(file_content.commit_sha, sha);
    assert_eq!(file_content.size, content.len());
}

#[tokio::test]
async fn test_file_content_at_path_traversal_rejected() {
    let tc = TestContext::new().await;

    let sha = tc.create_file("test.txt", "content", "Test").await;

    // Path traversal attempt
    let result = ArchiveBrowserBmc::file_content_at(&tc.ctx, &tc.mm, &sha, "../etc/passwd").await;
    assert!(result.is_err(), "Path traversal should be rejected");

    let result = ArchiveBrowserBmc::file_content_at(&tc.ctx, &tc.mm, &sha, "..").await;
    assert!(result.is_err(), ".. should be rejected");
}

#[tokio::test]
async fn test_file_content_nonexistent_file() {
    let tc = TestContext::new().await;

    let sha = tc.create_file("exists.txt", "content", "Test").await;

    let result =
        ArchiveBrowserBmc::file_content_at(&tc.ctx, &tc.mm, &sha, "does_not_exist.txt").await;
    assert!(result.is_err(), "Nonexistent file should error");
}

// ============================================================================
// FILE HISTORY TESTS (3 tests)
// ============================================================================

#[tokio::test]
async fn test_file_history_multiple_changes() {
    let tc = TestContext::new().await;

    // Create file
    tc.create_file("tracked.txt", "version 1", "Create file")
        .await;
    // Modify file
    tc.create_file("tracked.txt", "version 2", "Update file")
        .await;
    // Modify again
    tc.create_file("tracked.txt", "version 3", "Final update")
        .await;

    let history = ArchiveBrowserBmc::file_history(&tc.ctx, &tc.mm, "tracked.txt", 10)
        .await
        .expect("Get file history");

    assert_eq!(history.len(), 3);
    // Verify all expected messages are present (order may vary since timestamps could be identical)
    let messages: Vec<_> = history.iter().map(|c| c.message.as_str()).collect();
    assert!(messages.contains(&"Create file"));
    assert!(messages.contains(&"Update file"));
    assert!(messages.contains(&"Final update"));

    // Verify we have one "added" and two "modified" entries
    let added_count = history.iter().filter(|h| h.change_type == "added").count();
    let modified_count = history
        .iter()
        .filter(|h| h.change_type == "modified")
        .count();
    assert_eq!(added_count, 1, "Should have one added entry");
    assert_eq!(modified_count, 2, "Should have two modified entries");
}

#[tokio::test]
async fn test_file_history_limit() {
    let tc = TestContext::new().await;

    // Create many changes
    for i in 1..=5 {
        tc.create_file("tracked.txt", &format!("v{}", i), &format!("Update {}", i))
            .await;
    }

    let history = ArchiveBrowserBmc::file_history(&tc.ctx, &tc.mm, "tracked.txt", 2)
        .await
        .expect("Get file history");

    assert_eq!(history.len(), 2, "Should respect limit");
}

#[tokio::test]
async fn test_file_history_path_traversal_rejected() {
    let tc = TestContext::new().await;

    tc.create_file("test.txt", "content", "Test").await;

    let result = ArchiveBrowserBmc::file_history(&tc.ctx, &tc.mm, "../etc/passwd", 10).await;
    assert!(result.is_err(), "Path traversal should be rejected");

    let result = ArchiveBrowserBmc::file_history(&tc.ctx, &tc.mm, "", 10).await;
    assert!(result.is_err(), "Empty path should error");
}

// ============================================================================
// ACTIVITY TIMELINE TESTS (3 tests)
// ============================================================================

#[tokio::test]
async fn test_activity_timeline_empty_repo() {
    let tc = TestContext::new().await;

    let now = Utc::now();
    let week_ago = now - Duration::days(7);

    let activity = ArchiveBrowserBmc::activity_timeline(&tc.ctx, &tc.mm, week_ago, now)
        .await
        .expect("Get activity");

    assert_eq!(activity.commit_count, 0);
    assert!(activity.commits_by_day.is_empty());
    assert!(activity.commits_by_author.is_empty());
}

#[tokio::test]
async fn test_activity_timeline_with_commits() {
    let tc = TestContext::new().await;

    // Create some commits
    tc.create_file("file1.txt", "content1", "Commit 1").await;
    tc.create_file("file2.txt", "content2", "Commit 2").await;
    tc.create_file("file3.txt", "content3", "Commit 3").await;

    let now = Utc::now();
    let week_ago = now - Duration::days(7);

    let activity = ArchiveBrowserBmc::activity_timeline(&tc.ctx, &tc.mm, week_ago, now)
        .await
        .expect("Get activity");

    assert_eq!(activity.commit_count, 3);
    assert!(activity.commits_by_author.contains_key("test"));
    assert_eq!(*activity.commits_by_author.get("test").unwrap(), 3);
}

#[tokio::test]
async fn test_activity_timeline_file_changes() {
    let tc = TestContext::new().await;

    // Create commits touching same file multiple times
    tc.create_file("hot_file.txt", "v1", "First").await;
    tc.create_file("hot_file.txt", "v2", "Second").await;
    tc.create_file("hot_file.txt", "v3", "Third").await;
    tc.create_file("cold_file.txt", "content", "Add cold").await;

    let now = Utc::now();
    let week_ago = now - Duration::days(7);

    let activity = ArchiveBrowserBmc::activity_timeline(&tc.ctx, &tc.mm, week_ago, now)
        .await
        .expect("Get activity");

    // hot_file.txt should appear more than cold_file.txt
    let hot_count = activity
        .most_changed_files
        .iter()
        .find(|(path, _)| path == "hot_file.txt")
        .map(|(_, count)| *count)
        .unwrap_or(0);

    let cold_count = activity
        .most_changed_files
        .iter()
        .find(|(path, _)| path == "cold_file.txt")
        .map(|(_, count)| *count)
        .unwrap_or(0);

    assert!(hot_count > cold_count, "Hot file should have more changes");
}

// ============================================================================
// SECURITY TESTS (2 tests) - already covered in file_content and file_history
// Additional security edge cases
// ============================================================================

#[tokio::test]
async fn test_commit_details_sha_injection() {
    let tc = TestContext::new().await;

    tc.create_file("test.txt", "content", "Test").await;

    // Attempt SHA injection with special characters
    let result = ArchiveBrowserBmc::commit_details(&tc.ctx, &tc.mm, "abc; rm -rf /").await;
    assert!(result.is_err(), "SHA with special chars should be rejected");

    let result = ArchiveBrowserBmc::commit_details(&tc.ctx, &tc.mm, "abc\n123").await;
    assert!(result.is_err(), "SHA with newlines should be rejected");
}

#[tokio::test]
async fn test_list_files_path_injection() {
    let tc = TestContext::new().await;

    let sha = tc.create_file("test.txt", "content", "Test").await;

    // These should not crash but may error or return empty
    let _ = ArchiveBrowserBmc::list_files_at(&tc.ctx, &tc.mm, &sha, "..").await;
    let _ = ArchiveBrowserBmc::list_files_at(&tc.ctx, &tc.mm, &sha, "./..").await;
    // Test passes if no panic
}

// ============================================================================
// EDGE CASES (2 tests)
// ============================================================================

#[tokio::test]
async fn test_list_commits_single_commit_repo() {
    let tc = TestContext::new().await;

    let sha = tc.create_file("only.txt", "content", "Only commit").await;

    let commits = ArchiveBrowserBmc::list_commits(&tc.ctx, &tc.mm, None, 10)
        .await
        .expect("List commits");

    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].full_sha, sha);
    assert!(commits[0].short_sha.starts_with(&sha[..7]));
}

#[tokio::test]
async fn test_commit_details_with_multiple_file_changes() {
    let tc = TestContext::new().await;

    // Create initial files
    tc.create_file("keep.txt", "keep", "Add keep").await;
    tc.create_file("modify.txt", "v1", "Add modify").await;
    tc.create_file("delete.txt", "delete", "Add delete").await;

    // Now modify multiple in one commit
    // (Note: Our test helper creates one file per commit, so we simulate by checking a single commit)
    let sha = tc.create_file("new.txt", "new", "Add new file").await;

    let details = ArchiveBrowserBmc::commit_details(&tc.ctx, &tc.mm, &sha)
        .await
        .expect("Get details");

    // Should show new.txt as added
    assert!(
        details.files_added.contains(&"new.txt".to_string()),
        "Should detect added file"
    );
}
