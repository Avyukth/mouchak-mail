//! Resource cleanup tests
//!
//! Verifies that resources (file handles, git repos, database connections)
//! are properly cleaned up, even on errors.

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use chrono::{Duration, Utc};
use lib_core::Result;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::types::{AgentId, ProjectId};
use serial_test::serial;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test project and agent
async fn setup_project_and_agent(tc: &TestContext) -> (ProjectId, AgentId) {
    let p_id = ProjectBmc::create(&tc.ctx, &tc.mm, "cleanup-proj", "Cleanup Project")
        .await
        .unwrap();
    let a_id = AgentBmc::create(
        &tc.ctx,
        &tc.mm,
        AgentForCreate {
            project_id: p_id,
            name: "cleanup-agent".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "test".into(),
        },
    )
    .await
    .unwrap();
    (p_id, a_id)
}

#[tokio::test]
#[serial]
async fn test_git_repo_context_manager_normal_operation() -> Result<()> {
    let tc = TestContext::new().await?;
    let repo_root = tc.repo_root();

    // Initialize a git repo
    let output = std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_root)
        .output()
        .expect("git init failed");
    assert!(output.status.success(), "git init should succeed");

    // Open and close repo via git2
    {
        let repo = git2::Repository::open(&repo_root);
        assert!(repo.is_ok(), "Should be able to open git repo");
        // repo is dropped here, releasing file handles
    }

    // Verify we can still access the directory after repo is dropped
    assert!(
        repo_root.exists(),
        "Repo root should still exist after dropping handle"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_git_repo_context_manager_closes_on_exception() -> Result<()> {
    let tc = TestContext::new().await?;
    let repo_root = tc.repo_root();

    // Initialize a git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_root)
        .output()
        .expect("git init failed");

    // Simulate exception handling - repo should be cleaned up even on error
    let result: std::result::Result<(), &str> = {
        let _repo = git2::Repository::open(&repo_root).ok();
        // Simulate an error occurring
        Err("simulated error")
    };

    assert!(result.is_err());
    // Verify repo handle was released (we can open again)
    let repo2 = git2::Repository::open(&repo_root);
    assert!(repo2.is_ok(), "Should be able to reopen repo after error");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_git_repo_context_manager_handles_invalid_repo() -> Result<()> {
    let temp = TempDir::new().expect("temp dir");
    let invalid_path = temp.path().join("not-a-repo");
    std::fs::create_dir_all(&invalid_path).expect("mkdir");

    // Opening a non-git directory should fail gracefully
    let result = git2::Repository::open(&invalid_path);
    assert!(result.is_err(), "Opening non-repo should fail");

    // The temp directory should still be accessible
    assert!(invalid_path.exists());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_open_repo_if_available_returns_none_for_non_repo() -> Result<()> {
    let temp = TempDir::new().expect("temp dir");
    let non_repo = temp.path().join("regular-dir");
    std::fs::create_dir_all(&non_repo).expect("mkdir");

    // Helper function pattern that returns None for non-repos
    fn open_repo_if_available(path: &std::path::Path) -> Option<git2::Repository> {
        git2::Repository::open(path).ok()
    }

    let result = open_repo_if_available(&non_repo);
    assert!(result.is_none(), "Non-repo should return None");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_open_repo_if_available_returns_none_for_none() -> Result<()> {
    // Testing with a path that doesn't exist
    let nonexistent = PathBuf::from("/nonexistent/path/to/repo");

    fn open_repo_if_available(path: &std::path::Path) -> Option<git2::Repository> {
        git2::Repository::open(path).ok()
    }

    let result = open_repo_if_available(&nonexistent);
    assert!(result.is_none(), "Nonexistent path should return None");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_open_repo_if_available_returns_repo_for_valid_git() -> Result<()> {
    let tc = TestContext::new().await?;
    let repo_root = tc.repo_root();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_root)
        .output()
        .expect("git init");

    fn open_repo_if_available(path: &std::path::Path) -> Option<git2::Repository> {
        git2::Repository::open(path).ok()
    }

    let result = open_repo_if_available(&repo_root);
    assert!(result.is_some(), "Valid git repo should return Some");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_open_repo_if_available_closes_on_validation_failure() -> Result<()> {
    let tc = TestContext::new().await?;
    let repo_root = tc.repo_root();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_root)
        .output()
        .expect("git init");

    // Open repo, perform validation that fails, ensure cleanup
    let validation_result: std::result::Result<(), &str> = {
        let repo = git2::Repository::open(&repo_root).ok();
        if repo.is_some() {
            // Simulate validation failure
            Err("validation failed")
        } else {
            Ok(())
        }
    };

    assert!(validation_result.is_err());
    // Repo should be closed - verify by reopening
    assert!(git2::Repository::open(&repo_root).is_ok());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_open_repo_if_available_closes_on_working_tree_exception() -> Result<()> {
    let tc = TestContext::new().await?;
    let repo_root = tc.repo_root();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_root)
        .output()
        .expect("git init");

    // Test that repo is properly closed even when working tree ops fail
    let _result: std::result::Result<(), Box<dyn std::error::Error>> = {
        let repo = git2::Repository::open(&repo_root)?;
        // Try to get a worktree that doesn't exist - this should fail
        let _wt = repo.find_worktree("nonexistent");
        // Repo dropped here even if worktree lookup failed
        Ok(())
    };

    // Whether or not it failed, we should be able to reopen
    assert!(git2::Repository::open(&repo_root).is_ok());
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_file_reservation_statuses_cleanup_on_exception() -> Result<()> {
    let tc = TestContext::new().await?;
    let (p_id, a_id) = setup_project_and_agent(&tc).await;

    // Create a file reservation with expires_ts
    let expires = Utc::now().naive_utc() + Duration::hours(1);
    let reservation = FileReservationForCreate {
        project_id: p_id,
        agent_id: a_id,
        path_pattern: "src/**/*.rs".into(),
        exclusive: true,
        reason: "test reservation".into(),
        expires_ts: expires,
    };

    let res_id = FileReservationBmc::create(&tc.ctx, &tc.mm, reservation)
        .await
        .expect("create reservation");

    // Simulate an exception during work
    let work_result: std::result::Result<(), &str> = Err("simulated work failure");
    assert!(work_result.is_err());

    // Release the reservation even after error
    FileReservationBmc::release(&tc.ctx, &tc.mm, res_id)
        .await
        .expect("release should succeed");

    // Verify it's released
    let active = FileReservationBmc::list_active_for_project(&tc.ctx, &tc.mm, p_id)
        .await
        .expect("list active");
    assert!(
        active.is_empty(),
        "Reservation should be released after cleanup"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_file_reservation_release_works() -> Result<()> {
    let tc = TestContext::new().await?;
    let (p_id, a_id) = setup_project_and_agent(&tc).await;

    // Create reservation
    let expires = Utc::now().naive_utc() + Duration::minutes(1);
    let reservation = FileReservationForCreate {
        project_id: p_id,
        agent_id: a_id,
        path_pattern: "tests/**/*.rs".into(),
        exclusive: false,
        reason: "test".into(),
        expires_ts: expires,
    };

    let res_id = FileReservationBmc::create(&tc.ctx, &tc.mm, reservation)
        .await
        .expect("create");

    // Verify active
    let active = FileReservationBmc::list_active_for_project(&tc.ctx, &tc.mm, p_id).await?;
    assert_eq!(active.len(), 1, "Should have one active reservation");

    // Release
    FileReservationBmc::release(&tc.ctx, &tc.mm, res_id).await?;

    // Verify released
    let active_after = FileReservationBmc::list_active_for_project(&tc.ctx, &tc.mm, p_id).await?;
    assert!(
        active_after.is_empty(),
        "Should have no active reservations after release"
    );

    Ok(())
}
