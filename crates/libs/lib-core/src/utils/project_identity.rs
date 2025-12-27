//! Privacy-safe project slug generation ported from Python mcp_agent_mail.

use lib_common::config::ProjectIdentityMode;
use sha1::{Digest, Sha1};

pub fn compute_project_slug(
    human_key: &str,
    mode: ProjectIdentityMode,
    remote_name: &str,
) -> String {
    match mode {
        ProjectIdentityMode::Dir => crate::utils::slugify(human_key),
        ProjectIdentityMode::GitRemote => compute_git_remote_slug(human_key, remote_name)
            .unwrap_or_else(|| crate::utils::slugify(human_key)),
        ProjectIdentityMode::GitToplevel => {
            compute_git_toplevel_slug(human_key).unwrap_or_else(|| crate::utils::slugify(human_key))
        }
        ProjectIdentityMode::GitCommonDir => compute_git_common_dir_slug(human_key)
            .unwrap_or_else(|| crate::utils::slugify(human_key)),
    }
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
    fn test_short_sha1() {
        let hash = short_sha1("github.com/user/repo", 10);
        assert_eq!(hash.len(), 10);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
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
    fn test_normalize_remote_url_invalid() {
        assert!(normalize_remote_url("").is_none());
        assert!(normalize_remote_url("invalid").is_none());
        assert!(normalize_remote_url("git@github.com").is_none());
    }

    #[test]
    fn test_compute_project_slug_dir_mode() {
        let slug =
            compute_project_slug("/Users/testuser/myproject", ProjectIdentityMode::Dir, "origin");
        assert_eq!(slug, "users-testuser-myproject");
    }
}
