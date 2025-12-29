//! Tests for precommit guard and file reservation tool implementations
//! Target: Improve coverage for precommit.rs and files.rs

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::redundant_field_names
)]

use libsql::Builder;
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    agent_capabilities::{AgentCapabilityBmc, AgentCapabilityForCreate},
    file_reservation::FileReservationBmc,
    project::ProjectBmc,
};
use mouchak_mail_mcp::tools::{
    FileReservationParams, FileReservationPathsParams, ForceReleaseReservationParams,
    InstallPrecommitGuardParams, ListReservationsParams, ReleaseReservationParams,
    RenewFileReservationParams, UninstallPrecommitGuardParams,
};
use mouchak_mail_mcp::tools::{files, precommit};
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_precommit_files.db");
    let archive_root = temp_dir.path().join("archive");
    std::fs::create_dir_all(&archive_root).unwrap();

    let db = Builder::new_local(&db_path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let _ = conn.execute("PRAGMA journal_mode=WAL;", ()).await;

    let schema1 = include_str!("../../../../migrations/001_initial_schema.sql");
    conn.execute_batch(schema1).await.unwrap();
    let schema2 = include_str!("../../../../migrations/002_agent_capabilities.sql");
    conn.execute_batch(schema2).await.unwrap();
    let schema3 = include_str!("../../../../migrations/003_tool_metrics.sql");
    conn.execute_batch(schema3).await.unwrap();
    let schema4 = include_str!("../../../../migrations/004_attachments.sql");
    conn.execute_batch(schema4).await.unwrap();

    let app_config = Arc::new(AppConfig::default());
    let mm = ModelManager::new_for_test(conn, archive_root, app_config);
    (Arc::new(mm), temp_dir)
}

async fn setup_project_and_agent_with_capability(mm: &Arc<ModelManager>) -> (i64, i64, String) {
    let ctx = Ctx::root_ctx();
    let project_slug = format!("test-files-project-{}", Uuid::new_v4());
    let project_id = ProjectBmc::create(&ctx, mm, &project_slug, "Test Files Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id: project_id,
        name: "file_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent with file capabilities".to_string(),
    };
    let agent_id = AgentBmc::create(&ctx, mm, agent_c).await.unwrap();

    // Grant file_reservation_paths capability
    let cap = AgentCapabilityForCreate {
        agent_id: agent_id.into(),
        capability: "file_reservation_paths".to_string(),
        granted_by: None,
        expires_at: None,
    };
    AgentCapabilityBmc::create(&ctx, mm, cap).await.unwrap();

    (project_id.into(), agent_id.into(), project_slug)
}

// ==============================================================================
// precommit guard tests
// ==============================================================================

#[tokio::test]
async fn test_install_precommit_guard_impl_success() {
    let (mm, temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    // Create a fake git repo structure
    let repo_path = temp.path().join("fake-repo");
    std::fs::create_dir_all(repo_path.join(".git")).unwrap();

    let params = InstallPrecommitGuardParams {
        project_slug,
        target_repo_path: repo_path.to_string_lossy().to_string(),
    };

    let result = precommit::install_precommit_guard_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "Install should succeed: {:?}", result);

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("installed"));

    // Verify hook was created
    let hook_path = repo_path.join(".git").join("hooks").join("pre-commit");
    assert!(hook_path.exists(), "Hook file should exist");
}

#[tokio::test]
async fn test_install_precommit_guard_impl_creates_hooks_dir() {
    let (mm, temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    // Create git dir but no hooks dir
    let repo_path = temp.path().join("repo-no-hooks");
    std::fs::create_dir_all(repo_path.join(".git")).unwrap();

    let params = InstallPrecommitGuardParams {
        project_slug,
        target_repo_path: repo_path.to_string_lossy().to_string(),
    };

    let result = precommit::install_precommit_guard_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    // Verify hooks directory was created
    let hooks_dir = repo_path.join(".git").join("hooks");
    assert!(hooks_dir.exists());
}

#[tokio::test]
async fn test_install_precommit_guard_impl_invalid_project() {
    let (mm, temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let repo_path = temp.path().join("some-repo");
    std::fs::create_dir_all(repo_path.join(".git")).unwrap();

    let params = InstallPrecommitGuardParams {
        project_slug: "nonexistent-project".to_string(),
        target_repo_path: repo_path.to_string_lossy().to_string(),
    };

    let result = precommit::install_precommit_guard_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_uninstall_precommit_guard_impl_success() {
    let (mm, temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    // First install the guard
    let repo_path = temp.path().join("uninstall-test-repo");
    std::fs::create_dir_all(repo_path.join(".git")).unwrap();

    let install_params = InstallPrecommitGuardParams {
        project_slug,
        target_repo_path: repo_path.to_string_lossy().to_string(),
    };
    precommit::install_precommit_guard_impl(&ctx, &mm, install_params)
        .await
        .unwrap();

    // Now uninstall
    let params = UninstallPrecommitGuardParams {
        target_repo_path: repo_path.to_string_lossy().to_string(),
    };

    let result = precommit::uninstall_precommit_guard_impl(params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("uninstalled"));

    // Verify hook was removed
    let hook_path = repo_path.join(".git").join("hooks").join("pre-commit");
    assert!(!hook_path.exists(), "Hook file should be removed");
}

#[tokio::test]
async fn test_uninstall_precommit_guard_impl_no_hook() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path().join("no-hook-repo");
    std::fs::create_dir_all(repo_path.join(".git").join("hooks")).unwrap();

    let params = UninstallPrecommitGuardParams {
        target_repo_path: repo_path.to_string_lossy().to_string(),
    };

    let result = precommit::uninstall_precommit_guard_impl(params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("No pre-commit hook found"));
}

#[tokio::test]
async fn test_uninstall_precommit_guard_impl_foreign_hook() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path().join("foreign-hook-repo");
    let hooks_dir = repo_path.join(".git").join("hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap();

    // Write a non-Agent-Mail hook
    std::fs::write(
        hooks_dir.join("pre-commit"),
        "#!/bin/sh\necho 'Foreign hook'\nexit 0",
    )
    .unwrap();

    let params = UninstallPrecommitGuardParams {
        target_repo_path: repo_path.to_string_lossy().to_string(),
    };

    let result = precommit::uninstall_precommit_guard_impl(params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("not a Mouchak Mail guard"));

    // Hook should not be removed
    assert!(hooks_dir.join("pre-commit").exists());
}

// ==============================================================================
// file reservation tests
// ==============================================================================

#[tokio::test]
async fn test_reserve_file_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    let params = FileReservationParams {
        project_slug,
        agent_name: "file_agent".to_string(),
        path_pattern: "src/**/*.rs".to_string(),
        ttl_seconds: Some(3600),
        exclusive: Some(true),
        reason: Some("Development work".to_string()),
    };

    let result = files::reserve_file_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "Reserve should succeed: {:?}", result);

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Reserved") || text.contains("reservation"));
}

#[tokio::test]
async fn test_reserve_file_impl_default_values() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    let params = FileReservationParams {
        project_slug,
        agent_name: "file_agent".to_string(),
        path_pattern: "src/main.rs".to_string(),
        ttl_seconds: None, // Uses default
        exclusive: None,   // Uses default
        reason: None,      // Uses default
    };

    let result = files::reserve_file_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_reserve_file_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = FileReservationParams {
        project_slug: "nonexistent".to_string(),
        agent_name: "agent".to_string(),
        path_pattern: "*.rs".to_string(),
        ttl_seconds: None,
        exclusive: None,
        reason: None,
    };

    let result = files::reserve_file_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_reserve_file_impl_invalid_agent() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    let params = FileReservationParams {
        project_slug,
        agent_name: "nonexistent_agent".to_string(),
        path_pattern: "*.rs".to_string(),
        ttl_seconds: None,
        exclusive: None,
        reason: None,
    };

    let result = files::reserve_file_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_reserve_file_impl_no_capability() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    // Create agent without file_reservation_paths capability
    let project_slug = "no-cap-project";
    let project_id = ProjectBmc::create(&ctx, &mm, project_slug, "No Cap Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id: project_id,
        name: "no_cap_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent without file capability".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = FileReservationParams {
        project_slug: project_slug.to_string(),
        agent_name: "no_cap_agent".to_string(),
        path_pattern: "*.rs".to_string(),
        ttl_seconds: None,
        exclusive: None,
        reason: None,
    };

    let result = files::reserve_file_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{:?}", err).contains("capability"));
}

#[tokio::test]
async fn test_list_reservations_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    let params = ListReservationsParams { project_slug };

    let result = files::list_reservations_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_list_reservations_impl_with_reservations() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    // First create a reservation
    let reserve_params = FileReservationParams {
        project_slug: project_slug.clone(),
        agent_name: "file_agent".to_string(),
        path_pattern: "test/**/*.rs".to_string(),
        ttl_seconds: Some(3600),
        exclusive: Some(true),
        reason: Some("Testing".to_string()),
    };
    files::reserve_file_impl(&ctx, &mm, reserve_params)
        .await
        .unwrap();

    // Now list
    let params = ListReservationsParams { project_slug };

    let result = files::list_reservations_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("test/**/*.rs") || text.contains("reservation"));
}

#[tokio::test]
async fn test_release_reservation_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    // First create a reservation
    let reserve_params = FileReservationParams {
        project_slug: project_slug.clone(),
        agent_name: "file_agent".to_string(),
        path_pattern: "release-test/*.rs".to_string(),
        ttl_seconds: Some(3600),
        exclusive: Some(true),
        reason: Some("To be released".to_string()),
    };
    files::reserve_file_impl(&ctx, &mm, reserve_params)
        .await
        .unwrap();

    // Get the reservation ID from the database
    let reservations = FileReservationBmc::list_active_for_project(&ctx, &mm, project_id.into())
        .await
        .unwrap();
    let res_id = reservations.first().unwrap().id;

    // Release by ID
    let params = ReleaseReservationParams {
        reservation_id: res_id,
    };

    let result = files::release_reservation_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_renew_file_reservation_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    // First create a reservation
    let reserve_params = FileReservationParams {
        project_slug: project_slug.clone(),
        agent_name: "file_agent".to_string(),
        path_pattern: "renew-test/*.rs".to_string(),
        ttl_seconds: Some(100), // Short TTL
        exclusive: Some(true),
        reason: Some("To be renewed".to_string()),
    };
    files::reserve_file_impl(&ctx, &mm, reserve_params)
        .await
        .unwrap();

    // Get the reservation ID from the database
    let reservations = FileReservationBmc::list_active_for_project(&ctx, &mm, project_id.into())
        .await
        .unwrap();
    let res_id = reservations.first().unwrap().id;

    // Renew with longer TTL
    let params = RenewFileReservationParams {
        reservation_id: res_id,
        ttl_seconds: Some(3600),
    };

    let result = files::renew_file_reservation_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_force_release_reservation_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    // First create a reservation
    let reserve_params = FileReservationParams {
        project_slug: project_slug.clone(),
        agent_name: "file_agent".to_string(),
        path_pattern: "force-release/*.rs".to_string(),
        ttl_seconds: Some(3600),
        exclusive: Some(true),
        reason: Some("To be force released".to_string()),
    };
    files::reserve_file_impl(&ctx, &mm, reserve_params)
        .await
        .unwrap();

    // Get the reservation ID from the database
    let reservations = FileReservationBmc::list_active_for_project(&ctx, &mm, project_id.into())
        .await
        .unwrap();
    let res_id = reservations.first().unwrap().id;

    // Force release by ID
    let params = ForceReleaseReservationParams {
        reservation_id: res_id,
    };

    let result = files::force_release_reservation_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_reserve_file_paths_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent_with_capability(&mm).await;

    let params = FileReservationPathsParams {
        project_slug,
        agent_name: "file_agent".to_string(),
        paths: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
        ttl_seconds: Some(3600),
        exclusive: true, // Not Option<bool>
        reason: Some("Multiple files".to_string()),
    };

    let result = files::file_reservation_paths_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Reserved") || text.contains("granted") || text.contains("2"));
}
