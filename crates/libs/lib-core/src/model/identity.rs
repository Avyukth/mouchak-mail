//! Identity resolution for MCP Agent Mail.
//!
//! Provides stable identity for projects that survives:
//! - Worktree changes (uses git common dir)
//! - WSL2 path variations (/mnt/c/ vs C:/)
//! - Case sensitivity differences (respects core.ignorecase)
//!
//! Identity is resolved in order:
//! 1. Committed marker file (.agent-mail-identity in repo)
//! 2. Private marker file (.agent-mail-identity.local)
//! 3. Git remote fingerprint (origin URL hash)
//! 4. Git common dir path hash
//! 5. Directory path hash (fallback for non-git)

use sha1::Digest;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

/// Marker file for committed project identity.
pub const COMMITTED_MARKER: &str = ".agent-mail-identity";

/// Marker file for private/local project identity.
pub const PRIVATE_MARKER: &str = ".agent-mail-identity.local";

/// Result of identity resolution with source information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIdentity {
    /// The stable identity hash (40-char hex).
    pub identity: String,
    /// Source of the identity.
    pub source: IdentitySource,
    /// Original path used for resolution.
    pub original_path: PathBuf,
    /// Normalized path (after WSL2 normalization).
    pub normalized_path: PathBuf,
    /// Git common dir if available.
    pub git_common_dir: Option<PathBuf>,
    /// Whether case-insensitive mode is active.
    pub case_insensitive: bool,
}

/// Source of the resolved identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    /// From committed .agent-mail-identity file.
    CommittedMarker,
    /// From private .agent-mail-identity.local file.
    PrivateMarker,
    /// From git remote URL fingerprint.
    RemoteFingerprint,
    /// From git common dir path.
    GitCommonDir,
    /// From directory path (non-git fallback).
    DirectoryPath,
}

impl std::fmt::Display for IdentitySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommittedMarker => write!(f, "committed_marker"),
            Self::PrivateMarker => write!(f, "private_marker"),
            Self::RemoteFingerprint => write!(f, "remote_fingerprint"),
            Self::GitCommonDir => write!(f, "git_common_dir"),
            Self::DirectoryPath => write!(f, "directory_path"),
        }
    }
}

/// Identity resolution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdentityMode {
    /// Full git-aware resolution (default).
    #[default]
    GitAware,
    /// Directory-only mode (no git).
    DirectoryOnly,
}

/// Resolve project identity for the given path.
///
/// This function provides a stable identity that:
/// - Is consistent across worktrees (uses git common dir)
/// - Handles WSL2 path normalization
/// - Respects git's case sensitivity settings
///
/// # Arguments
/// * `path` - Path to resolve identity for
/// * `mode` - Resolution mode (GitAware or DirectoryOnly)
///
/// # Returns
/// Resolved identity with source information
pub fn resolve_identity(path: &Path, mode: IdentityMode) -> ResolvedIdentity {
    let original_path = path.to_path_buf();
    let normalized_path = normalize_wsl2_path(path);

    // Default values for non-git case
    let git_common_dir = None;
    let mut case_insensitive = false;

    // Try git-based resolution if in GitAware mode
    if mode == IdentityMode::GitAware {
        // Get git common dir (handles worktrees)
        if let Some(common_dir) = get_git_common_dir(&normalized_path) {
            // Check case sensitivity setting
            case_insensitive = get_core_ignorecase(&normalized_path);

            // 1. Try committed marker
            if let Some(identity) = read_marker_file(&common_dir, COMMITTED_MARKER) {
                return ResolvedIdentity {
                    identity,
                    source: IdentitySource::CommittedMarker,
                    original_path,
                    normalized_path,
                    git_common_dir: Some(common_dir),
                    case_insensitive,
                };
            }

            // 2. Try private marker
            if let Some(identity) = read_marker_file(&common_dir, PRIVATE_MARKER) {
                return ResolvedIdentity {
                    identity,
                    source: IdentitySource::PrivateMarker,
                    original_path,
                    normalized_path,
                    git_common_dir: Some(common_dir),
                    case_insensitive,
                };
            }

            // 3. Try remote fingerprint
            if let Some(identity) = get_remote_fingerprint(&normalized_path) {
                return ResolvedIdentity {
                    identity,
                    source: IdentitySource::RemoteFingerprint,
                    original_path,
                    normalized_path,
                    git_common_dir: Some(common_dir),
                    case_insensitive,
                };
            }

            // 4. Use git common dir hash
            let identity = hash_path(&common_dir, case_insensitive);
            return ResolvedIdentity {
                identity,
                source: IdentitySource::GitCommonDir,
                original_path,
                normalized_path,
                git_common_dir: Some(common_dir),
                case_insensitive,
            };
        }
    }

    // 5. Fallback to directory path hash
    let identity = hash_path(&normalized_path, case_insensitive);
    ResolvedIdentity {
        identity,
        source: IdentitySource::DirectoryPath,
        original_path,
        normalized_path,
        git_common_dir,
        case_insensitive,
    }
}

/// Normalize WSL2 paths to Windows format.
///
/// Converts `/mnt/c/Users/...` to `C:/Users/...` for consistent
/// identity resolution across WSL2 and native Windows.
pub fn normalize_wsl2_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();

    // Check for WSL2 mount pattern: /mnt/{drive}/...
    if path_str.starts_with("/mnt/") && path_str.len() > 6 {
        let chars: Vec<char> = path_str.chars().collect();
        // Check if 6th char is the drive letter and 7th is '/'
        if chars.len() > 6 && chars[5].is_ascii_alphabetic() && chars[6] == '/' {
            let drive = chars[5].to_ascii_uppercase();
            let rest = &path_str[7..]; // Skip "/mnt/X/"
            let normalized = format!("{}:/{}", drive, rest);
            debug!(original = %path_str, normalized = %normalized, "WSL2 path normalized");
            return PathBuf::from(normalized);
        }
    }

    path.to_path_buf()
}

/// Get the git common directory for a path.
///
/// For regular repos, returns the .git directory.
/// For worktrees, returns the main repo's .git directory.
/// Returns canonicalized path for consistent comparison across worktrees.
pub fn get_git_common_dir(path: &Path) -> Option<PathBuf> {
    // Try git rev-parse --git-common-dir
    let output = Command::new("git")
        .args(["-C", &path.to_string_lossy()])
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let common_dir = String::from_utf8(output.stdout).ok()?.trim().to_string();

    if common_dir.is_empty() {
        return None;
    }

    let common_path = PathBuf::from(&common_dir);
    // If relative, resolve from path
    let resolved = if common_path.is_absolute() {
        common_path
    } else {
        path.join(&common_path)
    };

    // Canonicalize to get consistent path across worktrees
    // (e.g., /tmp/xxx/worktree/../main/.git -> /tmp/xxx/main/.git)
    resolved.canonicalize().ok().or(Some(resolved))
}

/// Check if git is configured for case-insensitive file system.
pub fn get_core_ignorecase(path: &Path) -> bool {
    let output = Command::new("git")
        .args(["-C", &path.to_string_lossy()])
        .args(["config", "--get", "core.ignorecase"])
        .output()
        .ok();

    output
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Read identity from a marker file.
fn read_marker_file(git_dir: &Path, marker_name: &str) -> Option<String> {
    // Marker is in repo root, not git dir
    // Find repo root from git dir
    let repo_root = if git_dir.ends_with(".git") {
        git_dir.parent()?
    } else {
        // For worktrees, git_dir might be .git/worktrees/xxx
        // We need the main repo root
        let mut current = git_dir;
        loop {
            let parent = current.parent()?;
            if parent.file_name().map_or(false, |n| n == ".git") {
                // Found .git, parent of that is repo root
                break parent.parent()?;
            }
            if current.ends_with(".git") {
                break current.parent()?;
            }
            current = parent;
            if current.as_os_str().is_empty() {
                return None;
            }
        }
    };

    let marker_path = repo_root.join(marker_name);
    std::fs::read_to_string(&marker_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Get fingerprint from git remote origin URL.
pub fn get_remote_fingerprint(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", &path.to_string_lossy()])
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8(output.stdout).ok()?.trim().to_string();

    if url.is_empty() {
        return None;
    }

    // Hash the URL for fingerprint
    Some(hash_string(&url))
}

/// Hash a path to produce identity string.
fn hash_path(path: &Path, case_insensitive: bool) -> String {
    let path_str = path.to_string_lossy();
    let normalized = if case_insensitive {
        path_str.to_lowercase()
    } else {
        path_str.to_string()
    };
    hash_string(&normalized)
}

/// Hash a string using SHA-1.
fn hash_string(s: &str) -> String {
    let mut hasher = sha1::Sha1::new();
    hasher.update(s.as_bytes());
    hex::encode(hasher.finalize())
}

/// Check if two paths resolve to the same identity.
///
/// Useful for verifying worktree identity consistency.
pub fn same_identity(path1: &Path, path2: &Path) -> bool {
    let id1 = resolve_identity(path1, IdentityMode::GitAware);
    let id2 = resolve_identity(path2, IdentityMode::GitAware);
    id1.identity == id2.identity
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_wsl2_path_normalization_mnt_c() {
        let wsl_path = Path::new("/mnt/c/Users/test/project");
        let normalized = normalize_wsl2_path(wsl_path);
        assert_eq!(normalized, PathBuf::from("C:/Users/test/project"));
    }

    #[test]
    fn test_wsl2_path_normalization_mnt_d() {
        let wsl_path = Path::new("/mnt/d/code/repo");
        let normalized = normalize_wsl2_path(wsl_path);
        assert_eq!(normalized, PathBuf::from("D:/code/repo"));
    }

    #[test]
    fn test_wsl2_path_normalization_lowercase_drive() {
        let wsl_path = Path::new("/mnt/e/projects/app");
        let normalized = normalize_wsl2_path(wsl_path);
        // Drive letter should be uppercase
        assert_eq!(normalized, PathBuf::from("E:/projects/app"));
    }

    #[test]
    fn test_wsl2_path_normalization_non_wsl_unchanged() {
        let unix_path = Path::new("/home/user/project");
        let normalized = normalize_wsl2_path(unix_path);
        assert_eq!(normalized, unix_path);

        let windows_path = Path::new("C:/Users/test");
        let normalized = normalize_wsl2_path(windows_path);
        assert_eq!(normalized, windows_path);
    }

    #[test]
    fn test_identity_source_display() {
        assert_eq!(
            IdentitySource::CommittedMarker.to_string(),
            "committed_marker"
        );
        assert_eq!(IdentitySource::PrivateMarker.to_string(), "private_marker");
        assert_eq!(
            IdentitySource::RemoteFingerprint.to_string(),
            "remote_fingerprint"
        );
        assert_eq!(IdentitySource::GitCommonDir.to_string(), "git_common_dir");
        assert_eq!(IdentitySource::DirectoryPath.to_string(), "directory_path");
    }

    #[test]
    fn test_hash_string_deterministic() {
        let hash1 = hash_string("test");
        let hash2 = hash_string("test");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 40); // SHA-1 hex length
    }

    #[test]
    fn test_hash_path_case_sensitive() {
        let hash1 = hash_path(Path::new("/Test/Path"), false);
        let hash2 = hash_path(Path::new("/test/path"), false);
        assert_ne!(hash1, hash2, "Case-sensitive hashes should differ");
    }

    #[test]
    fn test_hash_path_case_insensitive() {
        let hash1 = hash_path(Path::new("/Test/Path"), true);
        let hash2 = hash_path(Path::new("/test/path"), true);
        assert_eq!(hash1, hash2, "Case-insensitive hashes should match");
    }
}
