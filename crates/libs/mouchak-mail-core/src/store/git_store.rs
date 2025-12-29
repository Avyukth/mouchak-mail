//! Git-based storage for entity audit trails.
//!
//! This module provides Git operations for maintaining an audit log of all
//! entity changes. Every create, update, or delete operation is committed
//! to a Git repository for full traceability.
//!
//! # Architecture
//!
//! Entities are stored as files in a Git repository:
//! - Each entity type has its own directory (e.g., `agents/`, `messages/`)
//! - Entity data is serialized to JSON
//! - Each change creates a Git commit with author attribution
//!
//! # Example
//!
//! ```no_run
//! use mouchak_mail_core::store::git_store::{init_or_open_repo, commit_file};
//!
//! # fn example() -> mouchak_mail_core::Result<()> {
//! let repo = init_or_open_repo("data/audit")?;
//! let content = r#"{"id": 1, "name": "agent-1"}"#;
//! commit_file(&repo, "agents/1.json", content, "Create agent-1", "system", "system@local")?;
//! # Ok(())
//! # }
//! ```

use crate::Result;
use git2::{Error as GitError, Oid, Repository, Signature, Tree};
use std::path::Path;

/// Initializes or opens a Git repository at the given path.
///
/// If a `.git` directory exists at the path, opens the existing repository.
/// Otherwise, initializes a new repository.
///
/// # Arguments
///
/// * `path` - Path to the repository root directory
///
/// # Returns
///
/// A [`Repository`] handle for Git operations.
///
/// # Example
///
/// ```no_run
/// use mouchak_mail_core::store::git_store::init_or_open_repo;
///
/// # fn example() -> mouchak_mail_core::Result<()> {
/// let repo = init_or_open_repo("data/audit")?;
/// # Ok(())
/// # }
/// ```
pub fn init_or_open_repo<P: AsRef<Path>>(path: P) -> Result<Repository> {
    let path_ref = path.as_ref();
    // Check if THIS directory is a git repo (has .git subdirectory),
    // not just if `discover` can find a repo up the tree
    let git_dir = path_ref.join(".git");
    if git_dir.exists() {
        Repository::open(path_ref).map_err(crate::Error::from)
    } else {
        Repository::init(path).map_err(crate::Error::from)
    }
}

/// Opens an existing Git repository at the given path.
///
/// Unlike [`init_or_open_repo`], this function fails if no repository exists.
///
/// # Arguments
///
/// * `path` - Path to the repository root directory
///
/// # Returns
///
/// A [`Repository`] handle for Git operations.
///
/// # Errors
///
/// Returns an error if the repository does not exist.
pub fn open_repo<P: AsRef<Path>>(path: P) -> Result<Repository> {
    Repository::open(path).map_err(crate::Error::from)
}

/// Creates a commit with the given tree and signature
fn create_commit(
    repo: &Repository,
    tree: &Tree,
    signature: &Signature,
    message: &str,
) -> Result<Oid> {
    let parent_commit_opt = find_last_commit(repo)?;
    let commit_oid = match parent_commit_opt {
        Some(ref parent) => {
            repo.commit(Some("HEAD"), signature, signature, message, tree, &[parent])?
        }
        None => repo.commit(Some("HEAD"), signature, signature, message, tree, &[])?,
    };
    Ok(commit_oid)
}

/// Commits a file to the repository with the given content.
///
/// Writes the content to the file path and creates a commit.
/// Creates parent directories if they don't exist.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `file_path` - Relative path within the repository
/// * `content` - File content to write
/// * `message` - Commit message
/// * `author_name` - Git author name
/// * `author_email` - Git author email
///
/// # Returns
///
/// The OID of the created commit.
///
/// # Example
///
/// ```no_run
/// use mouchak_mail_core::store::git_store::{init_or_open_repo, commit_file};
///
/// # fn example() -> mouchak_mail_core::Result<()> {
/// let repo = init_or_open_repo("data/audit")?;
/// commit_file(
///     &repo,
///     "agents/agent-1.json",
///     r#"{"name": "agent-1"}"#,
///     "Create agent",
///     "system",
///     "system@local"
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn commit_file<P: AsRef<Path>>(
    repo: &Repository,
    file_path: P,
    content: &str,
    message: &str,
    author_name: &str,
    author_email: &str,
) -> Result<Oid> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::from_str("No working directory"))?;
    let full_path = workdir.join(file_path.as_ref());

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, content)?;

    let mut index = repo.index()?;
    index.add_path(file_path.as_ref())?;
    let tree = repo.find_tree(index.write_tree()?)?;
    let signature = Signature::now(author_name, author_email)?;

    create_commit(repo, &tree, &signature, message)
}

/// Commits multiple existing files to the repository in a single commit.
///
/// Unlike [`commit_file`], this function expects the files to already exist
/// on disk. It stages all provided paths and creates a single commit.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `paths` - Slice of relative paths to commit
/// * `message` - Commit message
/// * `author_name` - Git author name
/// * `author_email` - Git author email
///
/// # Returns
///
/// The OID of the created commit.
pub fn commit_paths<P: AsRef<Path>>(
    repo: &Repository,
    paths: &[P],
    message: &str,
    author_name: &str,
    author_email: &str,
) -> Result<Oid> {
    let mut index = repo.index()?;
    for path in paths {
        index.add_path(path.as_ref())?;
    }
    let tree = repo.find_tree(index.write_tree()?)?;
    let signature = Signature::now(author_name, author_email)?;

    create_commit(repo, &tree, &signature, message)
}

/// Finds the last commit in the repository, returns None if no commits exist.
fn find_last_commit(repo: &Repository) -> Result<Option<git2::Commit<'_>>> {
    let head = repo.head();
    match head {
        Ok(head) => {
            let obj = head.resolve()?.peel(git2::ObjectType::Commit)?;
            let commit = obj.into_commit().map_err(|obj_not_commit| {
                GitError::from_str(&format!(
                    "Object is not a commit: {:?}",
                    obj_not_commit.id()
                ))
            })?;
            Ok(Some(commit))
        }
        Err(ref e)
            if e.code() == git2::ErrorCode::NotFound
                || e.code() == git2::ErrorCode::UnbornBranch =>
        {
            Ok(None)
        } // Empty repo
        Err(e) => Err(crate::Error::from(e)),
    }
}

/// Reads the content of a file from the repository at HEAD.
///
/// Retrieves the file content from the current HEAD commit, not from
/// the working directory.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `file_path` - Relative path within the repository
///
/// # Returns
///
/// The file content as a string.
///
/// # Errors
///
/// Returns an error if:
/// - No HEAD commit exists
/// - The file doesn't exist in HEAD
/// - The object is not a blob
pub fn read_file_content<P: AsRef<Path>>(repo: &Repository, file_path: P) -> Result<String> {
    let head = repo.head()?;
    let tree = head.peel_to_tree()?;
    let entry = tree.get_path(file_path.as_ref())?;
    let object = entry.to_object(repo)?;
    let blob = object
        .as_blob()
        .ok_or_else(|| GitError::from_str("Object is not a blob"))?;
    Ok(String::from_utf8_lossy(blob.content()).into_owned())
}

/// Reads the content of a file at a specific commit.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `commit_oid` - The OID of the commit to read from
/// * `file_path` - Relative path within the repository
///
/// # Returns
///
/// The file content as a string, or None if the file doesn't exist at that commit.
pub fn read_file_at_commit<P: AsRef<Path>>(
    repo: &Repository,
    commit_oid: Oid,
    file_path: P,
) -> Result<Option<String>> {
    let commit = repo.find_commit(commit_oid)?;
    let tree = commit.tree()?;

    match tree.get_path(file_path.as_ref()) {
        Ok(entry) => {
            let object = entry.to_object(repo)?;
            let blob = object
                .as_blob()
                .ok_or_else(|| GitError::from_str("Object is not a blob"))?;
            Ok(Some(String::from_utf8_lossy(blob.content()).into_owned()))
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(crate::Error::from(e)),
    }
}

/// Finds the latest commit before (or at) a given timestamp.
///
/// Walks the commit history from HEAD and returns the first commit
/// whose timestamp is less than or equal to the given time.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `before_time` - Unix timestamp (seconds since epoch)
///
/// # Returns
///
/// The OID of the commit, or None if no commits exist before the timestamp.
pub fn find_commit_before(repo: &Repository, before_time: i64) -> Result<Option<Oid>> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(e)
            if e.code() == git2::ErrorCode::NotFound
                || e.code() == git2::ErrorCode::UnbornBranch =>
        {
            return Ok(None);
        }
        Err(e) => return Err(crate::Error::from(e)),
    };

    let head_commit = head.peel_to_commit()?;

    // Walk commits from HEAD to find the latest one before the timestamp
    let mut revwalk = repo.revwalk()?;
    revwalk.push(head_commit.id())?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let commit_time = commit.time().seconds();

        if commit_time <= before_time {
            return Ok(Some(oid));
        }
    }

    Ok(None)
}

/// Lists all files in a directory at a specific commit.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `commit_oid` - The OID of the commit to read from
/// * `dir_path` - Directory path within the repository (empty for root)
///
/// # Returns
///
/// A vector of (file_name, content) tuples for files in the directory.
pub fn list_files_at_commit<P: AsRef<Path>>(
    repo: &Repository,
    commit_oid: Oid,
    dir_path: P,
) -> Result<Vec<(String, String)>> {
    let commit = repo.find_commit(commit_oid)?;
    let tree = commit.tree()?;

    let subtree = if dir_path.as_ref().as_os_str().is_empty() {
        tree
    } else {
        let entry = tree.get_path(dir_path.as_ref())?;
        let obj = entry.to_object(repo)?;
        obj.peel_to_tree()?
    };

    let mut files = Vec::new();
    for entry in subtree.iter() {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let name = entry.name().unwrap_or("").to_string();
            let obj = entry.to_object(repo)?;
            if let Some(blob) = obj.as_blob() {
                let content = String::from_utf8_lossy(blob.content()).into_owned();
                files.push((name, content));
            }
        }
    }

    Ok(files)
}

/// Gets the timestamp of a commit.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `commit_oid` - The OID of the commit
///
/// # Returns
///
/// Unix timestamp (seconds since epoch) of the commit.
pub fn get_commit_time(repo: &Repository, commit_oid: Oid) -> Result<i64> {
    let commit = repo.find_commit(commit_oid)?;
    Ok(commit.time().seconds())
}

/// Commits the deletion of a path (file or directory) from the repository.
///
/// Removes the path from the Git index and creates a commit recording the deletion.
/// The path should already be removed from the filesystem before calling this.
///
/// # Arguments
///
/// * `repo` - The Git repository
/// * `path` - Relative path to remove from the index
/// * `message` - Commit message
/// * `author_name` - Git author name
/// * `author_email` - Git author email
///
/// # Returns
///
/// The OID of the created commit.
pub fn commit_deletion<P: AsRef<Path>>(
    repo: &Repository,
    path: P,
    message: &str,
    author_name: &str,
    author_email: &str,
) -> Result<Oid> {
    let mut index = repo.index()?;
    index.remove_dir(path.as_ref(), 0)?;
    index.write()?;

    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let signature = Signature::now(author_name, author_email)?;

    create_commit(repo, &tree, &signature, message)
}
