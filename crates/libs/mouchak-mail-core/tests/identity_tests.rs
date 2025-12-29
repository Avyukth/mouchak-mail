//! Identity Resolution Tests
//!
//! Tests for project identity resolution covering:
//! - Directory mode (non-git)
//! - Git common dir handling (worktrees)
//! - WSL2 path normalization
//! - Case sensitivity (core.ignorecase)
//! - Marker file precedence
//! - Remote fingerprint fallback

#![allow(clippy::unwrap_used, clippy::expect_used)]

use mouchak_mail_core::model::identity::{
    COMMITTED_MARKER, IdentityMode, IdentitySource, PRIVATE_MARKER, get_core_ignorecase,
    get_git_common_dir, get_remote_fingerprint, normalize_wsl2_path, resolve_identity,
    same_identity,
};
use std::path::Path;
use tempfile::TempDir;

/// Helper to create a git repository using git2 (Rust-native)
fn create_git_repo(path: &Path) -> git2::Repository {
    git2::Repository::init(path).expect("Failed to init git repo")
}

/// Helper to create initial commit (required for worktrees)
fn create_initial_commit(repo: &git2::Repository) -> git2::Oid {
    let sig = git2::Signature::now("Test", "test@test.com").unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .expect("create initial commit")
}

/// Helper to create a worktree
fn create_worktree(repo: &git2::Repository, name: &str, path: &Path) -> git2::Worktree {
    repo.worktree(name, path, None)
        .expect("Failed to create worktree")
}

/// Helper to set git config value
fn set_git_config(repo_path: &Path, key: &str, value: &str) {
    std::process::Command::new("git")
        .args(["-C", &repo_path.to_string_lossy()])
        .args(["config", key, value])
        .output()
        .expect("git config");
}

// ============================================================================
// TEST 1: test_identity_dir_mode_without_repo
// ============================================================================

#[test]
fn test_identity_dir_mode_without_repo() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    // Resolve identity in directory-only mode (no git)
    let identity = resolve_identity(path, IdentityMode::DirectoryOnly);

    assert_eq!(
        identity.source,
        IdentitySource::DirectoryPath,
        "Should use directory path source in DirectoryOnly mode"
    );
    assert!(
        !identity.identity.is_empty(),
        "Identity should not be empty"
    );
    assert_eq!(identity.identity.len(), 40, "SHA-1 hash should be 40 chars");
    assert!(
        identity.git_common_dir.is_none(),
        "No git common dir in DirectoryOnly mode"
    );
}

// ============================================================================
// TEST 2: test_identity_mode_git_common_dir_without_repo_falls_back
// ============================================================================

#[test]
fn test_identity_mode_git_common_dir_without_repo_falls_back() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    // Not a git repo, but using GitAware mode
    let identity = resolve_identity(path, IdentityMode::GitAware);

    // Should fall back to directory path since there's no git repo
    assert_eq!(
        identity.source,
        IdentitySource::DirectoryPath,
        "GitAware mode should fall back to DirectoryPath when not in git repo"
    );
    assert!(
        identity.git_common_dir.is_none(),
        "No git common dir for non-repo"
    );
}

// ============================================================================
// TEST 3: test_identity_same_across_worktrees
// ============================================================================

#[test]
fn test_identity_same_across_worktrees() {
    let temp_dir = TempDir::new().unwrap();
    let main_repo_path = temp_dir.path().join("main-repo");
    std::fs::create_dir_all(&main_repo_path).unwrap();

    // Create main repo
    let repo = create_git_repo(&main_repo_path);
    create_initial_commit(&repo);

    // Create worktree
    let wt_path = temp_dir.path().join("worktree-1");
    let _wt = create_worktree(&repo, "wt-1", &wt_path);

    // Verify both paths exist
    assert!(main_repo_path.exists(), "Main repo should exist");
    assert!(wt_path.exists(), "Worktree should exist");

    // Identity should be the same for main repo and worktree
    let main_identity = resolve_identity(&main_repo_path, IdentityMode::GitAware);
    let wt_identity = resolve_identity(&wt_path, IdentityMode::GitAware);

    assert_eq!(
        main_identity.identity, wt_identity.identity,
        "Main repo and worktree should have same identity"
    );

    // Both should have git_common_dir pointing to main repo's .git
    assert!(main_identity.git_common_dir.is_some());
    assert!(wt_identity.git_common_dir.is_some());

    // Verify using same_identity helper
    assert!(
        same_identity(&main_repo_path, &wt_path),
        "same_identity should return true for main repo and worktree"
    );
}

// ============================================================================
// TEST 4: test_wsl2_path_normalization
// ============================================================================

#[test]
fn test_wsl2_path_normalization() {
    // Test various WSL2 path patterns
    let test_cases = vec![
        ("/mnt/c/Users/test/project", "C:/Users/test/project"),
        ("/mnt/d/code/repo", "D:/code/repo"),
        ("/mnt/e/work", "E:/work"),
        ("/mnt/f/", "F:/"),
    ];

    for (input, expected) in test_cases {
        let normalized = normalize_wsl2_path(Path::new(input));
        assert_eq!(
            normalized.to_string_lossy(),
            expected,
            "WSL2 path {} should normalize to {}",
            input,
            expected
        );
    }

    // Non-WSL paths should be unchanged
    let unchanged = vec![
        "/home/user/project",
        "/var/log",
        "C:/Windows",
        "./relative/path",
    ];

    for path in unchanged {
        let normalized = normalize_wsl2_path(Path::new(path));
        assert_eq!(
            normalized.to_string_lossy(),
            path,
            "Non-WSL path {} should be unchanged",
            path
        );
    }
}

// ============================================================================
// TEST 5: test_identity_reports_core_ignorecase
// ============================================================================

#[test]
fn test_identity_reports_core_ignorecase() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    // Create repo
    let _repo = create_git_repo(&repo_path);

    // Initially, core.ignorecase should be false (or unset)
    let _identity_before = resolve_identity(&repo_path, IdentityMode::GitAware);

    // Set core.ignorecase = true
    set_git_config(&repo_path, "core.ignorecase", "true");

    // Now it should report case insensitive
    let case_insensitive = get_core_ignorecase(&repo_path);
    assert!(
        case_insensitive,
        "core.ignorecase should be true after setting"
    );

    // Resolve identity again
    let identity_after = resolve_identity(&repo_path, IdentityMode::GitAware);
    assert!(
        identity_after.case_insensitive,
        "ResolvedIdentity should report case_insensitive=true"
    );

    // With case sensitivity, same-case paths should hash identically
    // (This is tested via the hash_path internal function)
}

// ============================================================================
// TEST 6: test_whois_and_projects_resources
// ============================================================================

#[test]
fn test_whois_and_projects_resources() {
    // This test verifies identity can be used to map paths to projects
    let temp_dir = TempDir::new().unwrap();

    // Create two separate repos
    let repo1_path = temp_dir.path().join("project-alpha");
    let repo2_path = temp_dir.path().join("project-beta");
    std::fs::create_dir_all(&repo1_path).unwrap();
    std::fs::create_dir_all(&repo2_path).unwrap();

    let _repo1 = create_git_repo(&repo1_path);
    let _repo2 = create_git_repo(&repo2_path);

    // Each should have unique identity
    let id1 = resolve_identity(&repo1_path, IdentityMode::GitAware);
    let id2 = resolve_identity(&repo2_path, IdentityMode::GitAware);

    assert_ne!(
        id1.identity, id2.identity,
        "Different repos should have different identities"
    );

    // Simulate "whois" - mapping identity to project name
    // (In production, this would look up in database)
    let identity_to_project: std::collections::HashMap<String, &str> = [
        (id1.identity.clone(), "project-alpha"),
        (id2.identity.clone(), "project-beta"),
    ]
    .into();

    // Verify lookup works
    assert_eq!(
        identity_to_project.get(&id1.identity),
        Some(&"project-alpha")
    );
    assert_eq!(
        identity_to_project.get(&id2.identity),
        Some(&"project-beta")
    );
}

// ============================================================================
// TEST 7: test_committed_marker_precedence
// ============================================================================

#[test]
fn test_committed_marker_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    // Create repo with remote (for fingerprint testing)
    let repo = create_git_repo(&repo_path);
    create_initial_commit(&repo);
    set_git_config(
        &repo_path,
        "remote.origin.url",
        "https://github.com/test/repo.git",
    );

    // Without marker, should use remote fingerprint
    let identity_no_marker = resolve_identity(&repo_path, IdentityMode::GitAware);
    assert_eq!(
        identity_no_marker.source,
        IdentitySource::RemoteFingerprint,
        "Without markers, should use remote fingerprint"
    );

    // Create committed marker
    let marker_content = "custom-identity-12345";
    std::fs::write(repo_path.join(COMMITTED_MARKER), marker_content).unwrap();

    // With committed marker, should use it (highest precedence)
    let identity_with_marker = resolve_identity(&repo_path, IdentityMode::GitAware);
    assert_eq!(
        identity_with_marker.source,
        IdentitySource::CommittedMarker,
        "Committed marker should take precedence"
    );
    assert_eq!(
        identity_with_marker.identity.trim(),
        marker_content,
        "Identity should match marker content"
    );
}

// ============================================================================
// TEST 8: test_private_marker_used_when_committed_missing
// ============================================================================

#[test]
fn test_private_marker_used_when_committed_missing() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    // Create repo with remote
    let repo = create_git_repo(&repo_path);
    create_initial_commit(&repo);
    set_git_config(
        &repo_path,
        "remote.origin.url",
        "https://github.com/test/repo.git",
    );

    // Create only private marker (no committed marker)
    let private_marker_content = "private-local-identity-67890";
    std::fs::write(repo_path.join(PRIVATE_MARKER), private_marker_content).unwrap();

    // Should use private marker since committed doesn't exist
    let identity = resolve_identity(&repo_path, IdentityMode::GitAware);
    assert_eq!(
        identity.source,
        IdentitySource::PrivateMarker,
        "Should use private marker when committed is missing"
    );
    assert_eq!(
        identity.identity.trim(),
        private_marker_content,
        "Identity should match private marker content"
    );

    // Now add committed marker - it should take precedence
    let committed_marker_content = "committed-identity-override";
    std::fs::write(repo_path.join(COMMITTED_MARKER), committed_marker_content).unwrap();

    let identity_with_both = resolve_identity(&repo_path, IdentityMode::GitAware);
    assert_eq!(
        identity_with_both.source,
        IdentitySource::CommittedMarker,
        "Committed marker should take precedence over private"
    );
}

// ============================================================================
// TEST 9: test_remote_fingerprint_when_no_markers
// ============================================================================

#[test]
fn test_remote_fingerprint_when_no_markers() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    // Create repo with remote
    let repo = create_git_repo(&repo_path);
    create_initial_commit(&repo);

    // Set origin URL
    let origin_url = "https://github.com/example/test-repo.git";
    set_git_config(&repo_path, "remote.origin.url", origin_url);

    // Verify remote fingerprint is used when no markers exist
    let identity = resolve_identity(&repo_path, IdentityMode::GitAware);
    assert_eq!(
        identity.source,
        IdentitySource::RemoteFingerprint,
        "Should use remote fingerprint when no markers"
    );

    // Verify fingerprint is deterministic (same URL = same fingerprint)
    let fingerprint1 = get_remote_fingerprint(&repo_path).unwrap();
    let fingerprint2 = get_remote_fingerprint(&repo_path).unwrap();
    assert_eq!(
        fingerprint1, fingerprint2,
        "Remote fingerprint should be deterministic"
    );

    // Different URL should give different fingerprint
    set_git_config(
        &repo_path,
        "remote.origin.url",
        "https://github.com/other/repo.git",
    );
    let different_fingerprint = get_remote_fingerprint(&repo_path).unwrap();
    assert_ne!(
        fingerprint1, different_fingerprint,
        "Different URLs should give different fingerprints"
    );
}

// ============================================================================
// TEST 10: test_git_common_dir_resolution
// ============================================================================

#[test]
fn test_git_common_dir_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let main_repo_path = temp_dir.path().join("main-repo");
    std::fs::create_dir_all(&main_repo_path).unwrap();

    // Create main repo
    let repo = create_git_repo(&main_repo_path);
    create_initial_commit(&repo);

    // Get common dir for main repo
    let main_common_dir = get_git_common_dir(&main_repo_path);
    assert!(
        main_common_dir.is_some(),
        "Main repo should have common dir"
    );

    // Create worktree
    let wt_path = temp_dir.path().join("worktree-test");
    let _wt = create_worktree(&repo, "test-wt", &wt_path);

    // Get common dir for worktree
    let wt_common_dir = get_git_common_dir(&wt_path);
    assert!(wt_common_dir.is_some(), "Worktree should have common dir");

    // Both should point to the same common dir
    // Note: Paths might differ in format, so we compare canonicalized
    let main_canonical = main_common_dir.unwrap().canonicalize().ok();
    let wt_canonical = wt_common_dir.unwrap().canonicalize().ok();

    if let (Some(main), Some(wt)) = (main_canonical, wt_canonical) {
        assert_eq!(
            main, wt,
            "Main repo and worktree should share the same git common dir"
        );
    }
}

// ============================================================================
// Additional edge case tests
// ============================================================================

#[test]
fn test_identity_hash_is_stable() {
    // Same path should always produce same hash
    let path = Path::new("/stable/test/path");

    let identity1 = resolve_identity(path, IdentityMode::DirectoryOnly);
    let identity2 = resolve_identity(path, IdentityMode::DirectoryOnly);

    assert_eq!(
        identity1.identity, identity2.identity,
        "Identity hash should be stable for same path"
    );
}

#[test]
fn test_empty_marker_file_ignored() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    let repo = create_git_repo(&repo_path);
    create_initial_commit(&repo);

    // Create empty marker file
    std::fs::write(repo_path.join(COMMITTED_MARKER), "").unwrap();

    // Empty marker should be ignored, fall back to git common dir
    let identity = resolve_identity(&repo_path, IdentityMode::GitAware);
    assert_ne!(
        identity.source,
        IdentitySource::CommittedMarker,
        "Empty marker should be ignored"
    );
}

#[test]
fn test_whitespace_only_marker_file_ignored() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    let repo = create_git_repo(&repo_path);
    create_initial_commit(&repo);

    // Create whitespace-only marker file
    std::fs::write(repo_path.join(COMMITTED_MARKER), "   \n\t  \n").unwrap();

    // Whitespace-only marker should be ignored
    let identity = resolve_identity(&repo_path, IdentityMode::GitAware);
    assert_ne!(
        identity.source,
        IdentitySource::CommittedMarker,
        "Whitespace-only marker should be ignored"
    );
}
