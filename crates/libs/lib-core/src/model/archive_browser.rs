//! Archive Browser - Git History Exploration
//!
//! Provides functionality to browse and explore git archive history:
//! - List commits (paginated)
//! - View commit details (message, author, files changed)
//! - Browse files at a specific commit
//! - View file content at a commit
//! - File history (commits that touched a file)
//! - Activity timeline (aggregate commit activity)
//!
//! # Example
//!
//! ```ignore
//! use lib_core::model::archive_browser::{ArchiveBrowserBmc, CommitFilter};
//!
//! // List recent commits
//! let commits = ArchiveBrowserBmc::list_commits(&ctx, &mm, None, 20).await?;
//!
//! // Get commit details
//! let details = ArchiveBrowserBmc::commit_details(&ctx, &mm, "abc123").await?;
//!
//! // Browse files at commit
//! let files = ArchiveBrowserBmc::list_files_at(&ctx, &mm, "abc123", "").await?;
//! ```

use crate::model::ModelManager;
use crate::store::git_store;
use crate::{Ctx, Error, Result};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Summary information about a commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    /// Short commit SHA (7 chars)
    pub short_sha: String,
    /// Full commit SHA (40 chars)
    pub full_sha: String,
    /// Commit message (first line)
    pub message: String,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Commit timestamp (UTC)
    pub timestamp: DateTime<Utc>,
    /// Number of files changed (if available)
    pub files_changed: Option<usize>,
}

/// Detailed information about a commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetails {
    /// Full commit SHA
    pub sha: String,
    /// Full commit message
    pub message: String,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Parent commit SHAs
    pub parents: Vec<String>,
    /// Files added in this commit
    pub files_added: Vec<String>,
    /// Files modified in this commit
    pub files_modified: Vec<String>,
    /// Files deleted in this commit
    pub files_deleted: Vec<String>,
}

/// A file/directory entry at a specific commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// File or directory name
    pub name: String,
    /// Full path within repository
    pub path: String,
    /// Whether this is a directory
    pub is_directory: bool,
    /// File size in bytes (None for directories)
    pub size: Option<u64>,
}

/// File content at a specific commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    /// File path
    pub path: String,
    /// File content (UTF-8)
    pub content: String,
    /// Commit SHA this content is from
    pub commit_sha: String,
    /// Content size in bytes
    pub size: usize,
}

/// A commit that touched a specific file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHistoryEntry {
    /// Commit SHA
    pub sha: String,
    /// Commit message (first line)
    pub message: String,
    /// Author name
    pub author_name: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Change type: added, modified, deleted
    pub change_type: String,
}

/// Activity summary for a time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySummary {
    /// Start of the period
    pub period_start: DateTime<Utc>,
    /// End of the period
    pub period_end: DateTime<Utc>,
    /// Total commits in period
    pub commit_count: usize,
    /// Commits per day
    pub commits_by_day: HashMap<String, usize>,
    /// Commits per author
    pub commits_by_author: HashMap<String, usize>,
    /// Most active files
    pub most_changed_files: Vec<(String, usize)>,
}

/// Filter options for listing commits.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommitFilter {
    /// Filter by author name (partial match)
    pub author: Option<String>,
    /// Filter by path (commits that touched this path)
    pub path: Option<String>,
    /// Filter commits after this date
    pub since: Option<DateTime<Utc>>,
    /// Filter commits before this date
    pub until: Option<DateTime<Utc>>,
    /// Filter by message content (partial match)
    pub message_contains: Option<String>,
}

/// Archive Browser Backend Model Controller.
pub struct ArchiveBrowserBmc;

impl ArchiveBrowserBmc {
    /// List commits with optional filtering and pagination.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - Request context
    /// * `mm` - Model manager
    /// * `filter` - Optional filter criteria
    /// * `limit` - Maximum number of commits to return
    ///
    /// # Returns
    ///
    /// A vector of commit summaries.
    pub async fn list_commits(
        _ctx: &Ctx,
        mm: &ModelManager,
        filter: Option<CommitFilter>,
        limit: usize,
    ) -> Result<Vec<CommitSummary>> {
        let repo_arc = mm.get_repo().await?;
        let repo = repo_arc.lock().await;

        let head = match repo.head() {
            Ok(h) => h,
            Err(e)
                if e.code() == git2::ErrorCode::NotFound
                    || e.code() == git2::ErrorCode::UnbornBranch =>
            {
                return Ok(Vec::new());
            }
            Err(e) => return Err(Error::from(e)),
        };

        let head_commit = head.peel_to_commit()?;
        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_commit.id())?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let filter = filter.unwrap_or_default();
        let mut commits = Vec::new();
        let mut count = 0;

        for oid_result in revwalk {
            if count >= limit {
                break;
            }

            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;

            // Apply filters
            if let Some(ref since) = filter.since {
                let commit_time = Utc
                    .timestamp_opt(commit.time().seconds(), 0)
                    .single()
                    .unwrap_or_else(Utc::now);
                if commit_time < *since {
                    continue;
                }
            }

            if let Some(ref until) = filter.until {
                let commit_time = Utc
                    .timestamp_opt(commit.time().seconds(), 0)
                    .single()
                    .unwrap_or_else(Utc::now);
                if commit_time > *until {
                    continue;
                }
            }

            if let Some(ref author) = filter.author {
                let commit_author = commit.author();
                let commit_author_name = commit_author.name().unwrap_or("").to_string();
                if !commit_author_name
                    .to_lowercase()
                    .contains(&author.to_lowercase())
                {
                    continue;
                }
            }

            if let Some(ref msg_filter) = filter.message_contains {
                let msg = commit.message().unwrap_or("");
                if !msg.to_lowercase().contains(&msg_filter.to_lowercase()) {
                    continue;
                }
            }

            // Path filter requires diff analysis - skip for now if specified
            if filter.path.is_some() {
                // TODO: Implement path-based filtering via diff
            }

            let timestamp = Utc
                .timestamp_opt(commit.time().seconds(), 0)
                .single()
                .unwrap_or_else(Utc::now);

            commits.push(CommitSummary {
                short_sha: oid.to_string()[..7].to_string(),
                full_sha: oid.to_string(),
                message: commit
                    .message()
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string(),
                author_name: commit.author().name().unwrap_or("unknown").to_string(),
                author_email: commit
                    .author()
                    .email()
                    .unwrap_or("unknown@unknown")
                    .to_string(),
                timestamp,
                files_changed: None, // Would require diff calculation
            });

            count += 1;
        }

        Ok(commits)
    }

    /// Get detailed information about a specific commit.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - Request context
    /// * `mm` - Model manager
    /// * `sha` - Commit SHA (short or full)
    ///
    /// # Returns
    ///
    /// Detailed commit information including file changes.
    pub async fn commit_details(_ctx: &Ctx, mm: &ModelManager, sha: &str) -> Result<CommitDetails> {
        // Validate SHA format
        if sha.is_empty() {
            return Err(Error::InvalidInput(
                "Commit SHA cannot be empty".to_string(),
            ));
        }
        if !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::InvalidInput("Invalid commit SHA format".to_string()));
        }

        let repo_arc = mm.get_repo().await?;
        let repo = repo_arc.lock().await;

        // Find the commit by partial or full SHA
        let oid = git2::Oid::from_str(sha)
            .map_err(|_| Error::InvalidInput(format!("Invalid SHA: {}", sha)))?;
        let commit = repo.find_commit(oid)?;

        let timestamp = Utc
            .timestamp_opt(commit.time().seconds(), 0)
            .single()
            .unwrap_or_else(Utc::now);

        // Get parent SHAs
        let parents: Vec<String> = (0..commit.parent_count())
            .filter_map(|i| commit.parent_id(i).ok().map(|oid| oid.to_string()))
            .collect();

        // Calculate file changes by diffing with parent
        let (files_added, files_modified, files_deleted) =
            Self::calculate_file_changes(&repo, &commit)?;

        Ok(CommitDetails {
            sha: oid.to_string(),
            message: commit.message().unwrap_or("").to_string(),
            author_name: commit.author().name().unwrap_or("unknown").to_string(),
            author_email: commit
                .author()
                .email()
                .unwrap_or("unknown@unknown")
                .to_string(),
            timestamp,
            parents,
            files_added,
            files_modified,
            files_deleted,
        })
    }

    /// Calculate file changes between a commit and its parent.
    fn calculate_file_changes(
        repo: &git2::Repository,
        commit: &git2::Commit,
    ) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

        for delta in diff.deltas() {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            match delta.status() {
                git2::Delta::Added => added.push(path),
                git2::Delta::Modified => modified.push(path),
                git2::Delta::Deleted => deleted.push(path),
                git2::Delta::Renamed | git2::Delta::Copied => modified.push(path),
                _ => {}
            }
        }

        Ok((added, modified, deleted))
    }

    /// List files at a specific commit.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - Request context
    /// * `mm` - Model manager
    /// * `sha` - Commit SHA
    /// * `dir_path` - Directory path within the repository (empty for root)
    ///
    /// # Returns
    ///
    /// A vector of file/directory entries.
    pub async fn list_files_at(
        _ctx: &Ctx,
        mm: &ModelManager,
        sha: &str,
        dir_path: &str,
    ) -> Result<Vec<FileEntry>> {
        if sha.is_empty() {
            return Err(Error::InvalidInput(
                "Commit SHA cannot be empty".to_string(),
            ));
        }

        let repo_arc = mm.get_repo().await?;
        let repo = repo_arc.lock().await;

        let oid = git2::Oid::from_str(sha)
            .map_err(|_| Error::InvalidInput(format!("Invalid SHA: {}", sha)))?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

        // Navigate to target directory if specified
        let target_tree = if dir_path.is_empty() {
            tree
        } else {
            let entry = tree.get_path(std::path::Path::new(dir_path))?;
            let obj = entry.to_object(&repo)?;
            obj.peel_to_tree()?
        };

        let mut entries = Vec::new();
        for entry in target_tree.iter() {
            let name = entry.name().unwrap_or("").to_string();
            let path = if dir_path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", dir_path, name)
            };

            let is_directory = entry.kind() == Some(git2::ObjectType::Tree);
            let size = if !is_directory {
                entry
                    .to_object(&repo)
                    .ok()
                    .and_then(|obj| obj.as_blob().map(|b| b.size() as u64))
            } else {
                None
            };

            entries.push(FileEntry {
                name,
                path,
                is_directory,
                size,
            });
        }

        // Sort: directories first, then by name
        entries.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(entries)
    }

    /// Get file content at a specific commit.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - Request context
    /// * `mm` - Model manager
    /// * `sha` - Commit SHA
    /// * `file_path` - File path within the repository
    ///
    /// # Returns
    ///
    /// The file content.
    pub async fn file_content_at(
        _ctx: &Ctx,
        mm: &ModelManager,
        sha: &str,
        file_path: &str,
    ) -> Result<FileContent> {
        if sha.is_empty() {
            return Err(Error::InvalidInput(
                "Commit SHA cannot be empty".to_string(),
            ));
        }
        if file_path.is_empty() {
            return Err(Error::InvalidInput("File path cannot be empty".to_string()));
        }

        // Security: reject path traversal
        if file_path.contains("..") {
            return Err(Error::InvalidInput(
                "Path traversal not allowed".to_string(),
            ));
        }

        let repo_arc = mm.get_repo().await?;
        let repo = repo_arc.lock().await;

        let oid = git2::Oid::from_str(sha)
            .map_err(|_| Error::InvalidInput(format!("Invalid SHA: {}", sha)))?;

        let content = git_store::read_file_at_commit(&repo, oid, file_path)?
            .ok_or_else(|| Error::NotFound)?;

        Ok(FileContent {
            path: file_path.to_string(),
            content: content.clone(),
            commit_sha: sha.to_string(),
            size: content.len(),
        })
    }

    /// Get history of commits that touched a specific file.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - Request context
    /// * `mm` - Model manager
    /// * `file_path` - File path to get history for
    /// * `limit` - Maximum number of commits to return
    ///
    /// # Returns
    ///
    /// A vector of commits that modified the file.
    pub async fn file_history(
        _ctx: &Ctx,
        mm: &ModelManager,
        file_path: &str,
        limit: usize,
    ) -> Result<Vec<FileHistoryEntry>> {
        if file_path.is_empty() {
            return Err(Error::InvalidInput("File path cannot be empty".to_string()));
        }
        if file_path.contains("..") {
            return Err(Error::InvalidInput(
                "Path traversal not allowed".to_string(),
            ));
        }

        let repo_arc = mm.get_repo().await?;
        let repo = repo_arc.lock().await;

        let head = match repo.head() {
            Ok(h) => h,
            Err(e)
                if e.code() == git2::ErrorCode::NotFound
                    || e.code() == git2::ErrorCode::UnbornBranch =>
            {
                return Ok(Vec::new());
            }
            Err(e) => return Err(Error::from(e)),
        };

        let head_commit = head.peel_to_commit()?;
        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_commit.id())?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut history = Vec::new();
        let path = std::path::Path::new(file_path);

        for oid_result in revwalk {
            if history.len() >= limit {
                break;
            }

            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            let tree = commit.tree()?;

            // Check if this commit has the file
            let has_file = tree.get_path(path).is_ok();

            // Check parent to determine change type
            let parent_has_file = if commit.parent_count() > 0 {
                if let Ok(parent) = commit.parent(0) {
                    parent.tree()?.get_path(path).is_ok()
                } else {
                    false
                }
            } else {
                false
            };

            let change_type = match (parent_has_file, has_file) {
                (false, true) => "added",
                (true, true) => {
                    // Need to diff to confirm modification
                    if commit.parent_count() > 0 {
                        if let Ok(parent) = commit.parent(0) {
                            let diff =
                                repo.diff_tree_to_tree(Some(&parent.tree()?), Some(&tree), None)?;

                            let mut touched = false;
                            for delta in diff.deltas() {
                                if let Some(p) = delta.new_file().path() {
                                    if p == path {
                                        touched = true;
                                        break;
                                    }
                                }
                                if let Some(p) = delta.old_file().path() {
                                    if p == path {
                                        touched = true;
                                        break;
                                    }
                                }
                            }

                            if !touched {
                                continue; // File wasn't touched in this commit
                            }
                        }
                    }
                    "modified"
                }
                (true, false) => "deleted",
                (false, false) => continue, // File doesn't exist in either
            };

            let timestamp = Utc
                .timestamp_opt(commit.time().seconds(), 0)
                .single()
                .unwrap_or_else(Utc::now);

            history.push(FileHistoryEntry {
                sha: oid.to_string(),
                message: commit
                    .message()
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string(),
                author_name: commit.author().name().unwrap_or("unknown").to_string(),
                timestamp,
                change_type: change_type.to_string(),
            });
        }

        Ok(history)
    }

    /// Get activity summary for a time period.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - Request context
    /// * `mm` - Model manager
    /// * `since` - Start of the period
    /// * `until` - End of the period
    ///
    /// # Returns
    ///
    /// Aggregate activity statistics.
    pub async fn activity_timeline(
        _ctx: &Ctx,
        mm: &ModelManager,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<ActivitySummary> {
        let repo_arc = mm.get_repo().await?;
        let repo = repo_arc.lock().await;

        let head = match repo.head() {
            Ok(h) => h,
            Err(e)
                if e.code() == git2::ErrorCode::NotFound
                    || e.code() == git2::ErrorCode::UnbornBranch =>
            {
                return Ok(ActivitySummary {
                    period_start: since,
                    period_end: until,
                    commit_count: 0,
                    commits_by_day: HashMap::new(),
                    commits_by_author: HashMap::new(),
                    most_changed_files: Vec::new(),
                });
            }
            Err(e) => return Err(Error::from(e)),
        };

        let head_commit = head.peel_to_commit()?;
        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_commit.id())?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut commit_count = 0;
        let mut commits_by_day: HashMap<String, usize> = HashMap::new();
        let mut commits_by_author: HashMap<String, usize> = HashMap::new();
        let mut file_change_counts: HashMap<String, usize> = HashMap::new();

        for oid_result in revwalk {
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;

            let commit_time = Utc
                .timestamp_opt(commit.time().seconds(), 0)
                .single()
                .unwrap_or_else(Utc::now);

            // Filter by time range
            if commit_time < since {
                break; // Commits are sorted by time, so we can stop
            }
            if commit_time > until {
                continue;
            }

            commit_count += 1;

            // Count by day
            let day_key = commit_time.format("%Y-%m-%d").to_string();
            *commits_by_day.entry(day_key).or_insert(0) += 1;

            // Count by author
            let author = commit.author().name().unwrap_or("unknown").to_string();
            *commits_by_author.entry(author).or_insert(0) += 1;

            // Count file changes (expensive, so only if reasonable commit count)
            if commit_count <= 500 {
                if let Ok((added, modified, deleted)) = Self::calculate_file_changes(&repo, &commit)
                {
                    for path in added.iter().chain(modified.iter()).chain(deleted.iter()) {
                        *file_change_counts.entry(path.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Get top 10 most changed files
        let mut file_counts: Vec<_> = file_change_counts.into_iter().collect();
        file_counts.sort_by(|a, b| b.1.cmp(&a.1));
        let most_changed_files: Vec<_> = file_counts.into_iter().take(10).collect();

        Ok(ActivitySummary {
            period_start: since,
            period_end: until,
            commit_count,
            commits_by_day,
            commits_by_author,
            most_changed_files,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests for struct validation
    #[test]
    fn test_commit_filter_default() {
        let filter = CommitFilter::default();
        assert!(filter.author.is_none());
        assert!(filter.path.is_none());
        assert!(filter.since.is_none());
        assert!(filter.until.is_none());
        assert!(filter.message_contains.is_none());
    }

    #[test]
    fn test_commit_summary_serialization() {
        let summary = CommitSummary {
            short_sha: "abc1234".to_string(),
            full_sha: "abc1234567890def1234567890abc1234567890de".to_string(),
            message: "Test commit".to_string(),
            author_name: "Test Author".to_string(),
            author_email: "test@example.com".to_string(),
            timestamp: Utc::now(),
            files_changed: Some(3),
        };

        let json = serde_json::to_string(&summary).expect("Serialize");
        let deserialized: CommitSummary = serde_json::from_str(&json).expect("Deserialize");

        assert_eq!(summary.short_sha, deserialized.short_sha);
        assert_eq!(summary.full_sha, deserialized.full_sha);
        assert_eq!(summary.message, deserialized.message);
    }

    #[test]
    fn test_file_entry_serialization() {
        let entry = FileEntry {
            name: "test.json".to_string(),
            path: "dir/test.json".to_string(),
            is_directory: false,
            size: Some(1024),
        };

        let json = serde_json::to_string(&entry).expect("Serialize");
        let deserialized: FileEntry = serde_json::from_str(&json).expect("Deserialize");

        assert_eq!(entry.name, deserialized.name);
        assert_eq!(entry.is_directory, deserialized.is_directory);
        assert_eq!(entry.size, deserialized.size);
    }

    #[test]
    fn test_commit_details_serialization() {
        let details = CommitDetails {
            sha: "abc1234567890def1234567890abc1234567890de".to_string(),
            message: "Full commit message\n\nWith body".to_string(),
            author_name: "Test Author".to_string(),
            author_email: "test@example.com".to_string(),
            timestamp: Utc::now(),
            parents: vec!["parent123".to_string()],
            files_added: vec!["new.txt".to_string()],
            files_modified: vec!["changed.txt".to_string()],
            files_deleted: vec![],
        };

        let json = serde_json::to_string(&details).expect("Serialize");
        let deserialized: CommitDetails = serde_json::from_str(&json).expect("Deserialize");

        assert_eq!(details.sha, deserialized.sha);
        assert_eq!(details.parents, deserialized.parents);
        assert_eq!(details.files_added, deserialized.files_added);
    }
}
