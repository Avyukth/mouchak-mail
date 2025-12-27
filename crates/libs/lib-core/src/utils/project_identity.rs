//! Privacy-safe project slug generation ported from Python mcp_agent_mail.
//!
//! All modes now generate privacy-safe slugs that don't leak filesystem paths
//! or usernames. The format is `{project-name}-{hash}` where hash is derived
//! from the full path to ensure uniqueness.

use lib_common::config::ProjectIdentityMode;
use sha1::{Digest, Sha1};
use std::path::Path;

pub fn compute_project_slug(
    human_key: &str,
    mode: ProjectIdentityMode,
    remote_name: &str,
) -> String {
    match mode {
        ProjectIdentityMode::Dir => compute_dir_slug_safe(human_key),
        ProjectIdentityMode::GitRemote => compute_git_remote_slug(human_key, remote_name)
            .unwrap_or_else(|| compute_dir_slug_safe(human_key)),
        ProjectIdentityMode::GitToplevel => {
            compute_git_toplevel_slug(human_key).unwrap_or_else(|| compute_dir_slug_safe(human_key))
        }
        ProjectIdentityMode::GitCommonDir => compute_git_common_dir_slug(human_key)
            .unwrap_or_else(|| compute_dir_slug_safe(human_key)),
    }
}

fn compute_dir_slug_safe(path: &str) -> String {
    let path_obj = Path::new(path);
    let last_component = path_obj
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let slug_name = slug::slugify(last_component);
    let hash = short_sha1(path_obj.to_str().unwrap_or(last_component), 8);
    format!("{slug_name}-{hash}")
}

fn short_sha1(text: &str, n: usize) -> String {
    let mut hasher = Sha1::new();
    hasher.update(text.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)[..n.min(40)].to_string()
}

fn normalize_remote_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    let (host, path) = if url.starts_with("git@") {
        let rest = url.strip_prefix("git@")?;
        let (host, path) = rest.split_once(':')?;
        (host.to_string(), path.to_string())
    } else if url.starts_with("https://") || url.starts_with("http://") {
        let without_scheme = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))?;
        let (host, path) = without_scheme.split_once('/')?;
        (host.to_string(), path.to_string())
    } else {
        return None;
    };

    if host.is_empty() {
        return None;
    }

    let path = path.trim_start_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);

    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }

    let owner = parts[0];
    let repo = parts[1];

    Some(format!("{host}/{owner}/{repo}"))
}

fn compute_git_remote_slug(path: &str, remote_name: &str) -> Option<String> {
    let repo = git2::Repository::discover(path).ok()?;
    let remote = repo.find_remote(remote_name).ok()?;
    let remote_url = remote.url()?;

    let normalized = normalize_remote_url(remote_url)?;
    let repo_name = normalized.rsplit('/').next().unwrap_or("repo");
    let hash = short_sha1(&normalized, 10);

    Some(format!("{repo_name}-{hash}"))
}

fn compute_git_toplevel_slug(path: &str) -> Option<String> {
    let repo = git2::Repository::discover(path).ok()?;
    let workdir = repo.workdir()?;
    let workdir_real = workdir.canonicalize().ok()?;
    let dir_name = workdir_real.file_name()?.to_str()?;
    let hash = short_sha1(workdir_real.to_str()?, 10);

    Some(format!("{dir_name}-{hash}"))
}

fn compute_git_common_dir_slug(path: &str) -> Option<String> {
    let repo = git2::Repository::discover(path).ok()?;

    let common_dir = if repo.is_worktree() {
        repo.commondir().to_path_buf()
    } else {
        repo.path().to_path_buf()
    };

    let common_dir_real = common_dir.canonicalize().ok()?;
    let hash = short_sha1(common_dir_real.to_str()?, 10);

    Some(format!("repo-{hash}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_sha1_returns_correct_length() {
        let hash = short_sha1("github.com/user/repo", 10);
        assert_eq!(hash.len(), 10);
    }

    #[test]
    fn test_short_sha1_returns_hex_chars_only() {
        let hash = short_sha1("github.com/user/repo", 10);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_short_sha1_is_deterministic() {
        let hash1 = short_sha1("same-input", 10);
        let hash2 = short_sha1("same-input", 10);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_short_sha1_different_inputs_produce_different_hashes() {
        let hash1 = short_sha1("input-one", 10);
        let hash2 = short_sha1("input-two", 10);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_normalize_remote_url_ssh() {
        let result = normalize_remote_url("git@github.com:user/repo.git");
        assert_eq!(result, Some("github.com/user/repo".to_string()));
    }

    #[test]
    fn test_normalize_remote_url_https() {
        let result = normalize_remote_url("https://github.com/user/repo.git");
        assert_eq!(result, Some("github.com/user/repo".to_string()));
    }

    #[test]
    fn test_normalize_remote_url_no_git_suffix() {
        let result = normalize_remote_url("https://github.com/user/repo");
        assert_eq!(result, Some("github.com/user/repo".to_string()));
    }

    #[test]
    fn test_normalize_remote_url_empty_returns_none() {
        assert!(normalize_remote_url("").is_none());
    }

    #[test]
    fn test_normalize_remote_url_invalid_returns_none() {
        assert!(normalize_remote_url("invalid").is_none());
    }

    #[test]
    fn test_normalize_remote_url_incomplete_ssh_returns_none() {
        assert!(normalize_remote_url("git@github.com").is_none());
    }

    #[test]
    fn test_dir_mode_extracts_last_component_only() {
        let slug = compute_project_slug(
            "/home/testuser/myproject",
            ProjectIdentityMode::Dir,
            "origin",
        );
        assert!(slug.starts_with("myproject-"));
        assert!(!slug.contains("testuser"));
        assert!(!slug.contains("home"));
    }

    #[test]
    fn test_dir_mode_appends_hash_for_uniqueness() {
        let slug = compute_project_slug("/some/path/myproject", ProjectIdentityMode::Dir, "origin");
        let parts: Vec<&str> = slug.rsplitn(2, '-').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], "myproject");
        assert_eq!(parts[0].len(), 8);
        assert!(parts[0].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_dir_mode_same_dirname_different_paths_produce_different_slugs() {
        let slug1 = compute_project_slug("/path/one/myproject", ProjectIdentityMode::Dir, "origin");
        let slug2 = compute_project_slug("/path/two/myproject", ProjectIdentityMode::Dir, "origin");
        assert!(slug1.starts_with("myproject-"));
        assert!(slug2.starts_with("myproject-"));
        assert_ne!(slug1, slug2);
    }

    #[test]
    fn test_dir_mode_handles_deep_paths() {
        let slug = compute_project_slug(
            "/very/deep/nested/path/to/api-server",
            ProjectIdentityMode::Dir,
            "origin",
        );
        assert!(slug.starts_with("api-server-"));
        assert!(!slug.contains("very"));
        assert!(!slug.contains("deep"));
        assert!(!slug.contains("nested"));
        assert!(!slug.contains("path"));
    }

    #[cfg(windows)]
    #[test]
    fn test_dir_mode_handles_windows_style_paths() {
        let slug = compute_project_slug(
            "C:\\Users\\Dev\\myproject",
            ProjectIdentityMode::Dir,
            "origin",
        );
        assert!(slug.starts_with("myproject-"));
        assert!(!slug.contains("Users"));
        assert!(!slug.contains("Dev"));
    }

    #[test]
    fn test_dir_mode_slugifies_special_characters() {
        let slug = compute_project_slug(
            "/path/to/My Project Name",
            ProjectIdentityMode::Dir,
            "origin",
        );
        assert!(slug.starts_with("my-project-name-"));
    }

    #[test]
    fn test_compute_dir_slug_safe_fallback_for_empty_path() {
        let slug = compute_dir_slug_safe("");
        assert!(slug.starts_with("project-") || slug.contains("-"));
    }

    #[test]
    fn test_compute_dir_slug_safe_handles_root_path() {
        let slug = compute_dir_slug_safe("/");
        assert!(!slug.is_empty());
    }
}
