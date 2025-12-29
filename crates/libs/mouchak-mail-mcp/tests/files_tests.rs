#![allow(clippy::unwrap_used, clippy::expect_used)]

use libsql::Builder;
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    agent_capabilities::{AgentCapabilityBmc, AgentCapabilityForCreate},
    project::ProjectBmc,
};
use mouchak_mail_mcp::tools::files;
use mouchak_mail_mcp::tools::{
    FileReservationParams, FileReservationPathsParams, ForceReleaseReservationParams,
    ListReservationsParams, ReleaseReservationParams, RenewFileReservationParams,
};
use std::sync::Arc;
use tempfile::TempDir;

fn extract_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .first()
        .map(|c| format!("{:?}", c))
        .unwrap_or_default()
}

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_files.db");
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

async fn setup_project_with_agent(mm: &Arc<ModelManager>, suffix: &str) -> (String, String) {
    let ctx = Ctx::root_ctx();

    let project_slug = format!("files-project-{}", suffix);
    let project_id = ProjectBmc::create(
        &ctx,
        mm,
        &project_slug,
        &format!("Files Project {}", suffix),
    )
    .await
    .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: format!("file_agent_{}", suffix),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "File reservation agent".to_string(),
    };
    let agent_id = AgentBmc::create(&ctx, mm, agent_c).await.unwrap();

    let cap = AgentCapabilityForCreate {
        agent_id: agent_id.into(),
        capability: "file_reservation_paths".to_string(),
        granted_by: None,
        expires_at: None,
    };
    AgentCapabilityBmc::create(&ctx, mm, cap).await.unwrap();

    (project_slug, format!("file_agent_{}", suffix))
}

#[tokio::test]
async fn test_reserve_file_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, agent_name) = setup_project_with_agent(&mm, "reserve").await;

    let params = FileReservationParams {
        project_slug,
        agent_name,
        path_pattern: "src/**/*.rs".to_string(),
        exclusive: Some(true),
        reason: Some("Working on source files".to_string()),
        ttl_seconds: Some(3600),
    };

    let result = files::reserve_file_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Reserved"));
    assert!(output.contains("src/**/*.rs"));
}

#[tokio::test]
async fn test_reserve_file_impl_invalid_path() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, agent_name) = setup_project_with_agent(&mm, "invalid_path").await;

    // Test absolute path rejection (the only path validation currently implemented)
    let params = FileReservationParams {
        project_slug,
        agent_name,
        path_pattern: "/etc/passwd".to_string(),
        exclusive: Some(true),
        reason: None,
        ttl_seconds: None,
    };

    let result = files::reserve_file_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("relative") || err.message.contains("leading /"),
        "Should reject absolute path: {}",
        err.message
    );
}

#[tokio::test]
async fn test_reserve_file_impl_agent_without_capability() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "no-cap-project";
    let project_id = ProjectBmc::create(&ctx, &mm, project_slug, "No Cap Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id,
        name: "no_cap_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Agent without file capability".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

    let params = FileReservationParams {
        project_slug: project_slug.to_string(),
        agent_name: "no_cap_agent".to_string(),
        path_pattern: "src/main.rs".to_string(),
        exclusive: Some(true),
        reason: None,
        ttl_seconds: None,
    };

    let result = files::reserve_file_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("capability"));
}

#[tokio::test]
async fn test_list_reservations_impl_empty() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "empty-reservations";
    ProjectBmc::create(&ctx, &mm, project_slug, "Empty Reservations Project")
        .await
        .unwrap();

    let params = ListReservationsParams {
        project_slug: project_slug.to_string(),
    };

    let result = files::list_reservations_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Active reservations"));
    assert!(output.contains("(0)"));
}

#[tokio::test]
async fn test_list_reservations_impl_with_reservations() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, agent_name) = setup_project_with_agent(&mm, "list_res").await;

    let reserve_params = FileReservationParams {
        project_slug: project_slug.clone(),
        agent_name,
        path_pattern: "Cargo.toml".to_string(),
        exclusive: Some(true),
        reason: Some("Editing cargo manifest".to_string()),
        ttl_seconds: Some(3600),
    };
    files::reserve_file_impl(&ctx, &mm, reserve_params)
        .await
        .unwrap();

    let params = ListReservationsParams { project_slug };

    let result = files::list_reservations_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Cargo.toml"));
    assert!(output.contains("exclusive: true"));
}

#[tokio::test]
async fn test_release_reservation_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, agent_name) = setup_project_with_agent(&mm, "release").await;

    let reserve_params = FileReservationParams {
        project_slug: project_slug.clone(),
        agent_name,
        path_pattern: "test.rs".to_string(),
        exclusive: Some(true),
        reason: None,
        ttl_seconds: Some(3600),
    };
    let reserve_result = files::reserve_file_impl(&ctx, &mm, reserve_params)
        .await
        .unwrap();

    let output = extract_text(&reserve_result);
    let reservation_id: i64 = output
        .split("reservation id:")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
        .expect("Should extract reservation id");

    let params = ReleaseReservationParams { reservation_id };

    let result = files::release_reservation_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Released reservation"));
}

#[tokio::test]
async fn test_force_release_reservation_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, agent_name) = setup_project_with_agent(&mm, "force_release").await;

    let reserve_params = FileReservationParams {
        project_slug: project_slug.clone(),
        agent_name,
        path_pattern: "locked.rs".to_string(),
        exclusive: Some(true),
        reason: Some("This will be force released".to_string()),
        ttl_seconds: Some(7200),
    };
    let reserve_result = files::reserve_file_impl(&ctx, &mm, reserve_params)
        .await
        .unwrap();

    let output = extract_text(&reserve_result);
    let reservation_id: i64 = output
        .split("reservation id:")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
        .expect("Should extract reservation id");

    let params = ForceReleaseReservationParams { reservation_id };

    let result = files::force_release_reservation_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Force released"));
}

#[tokio::test]
async fn test_renew_file_reservation_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, agent_name) = setup_project_with_agent(&mm, "renew").await;

    let reserve_params = FileReservationParams {
        project_slug: project_slug.clone(),
        agent_name,
        path_pattern: "renew.rs".to_string(),
        exclusive: Some(true),
        reason: None,
        ttl_seconds: Some(1800),
    };
    let reserve_result = files::reserve_file_impl(&ctx, &mm, reserve_params)
        .await
        .unwrap();

    let output = extract_text(&reserve_result);
    let reservation_id: i64 = output
        .split("reservation id:")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
        .expect("Should extract reservation id");

    let params = RenewFileReservationParams {
        reservation_id,
        ttl_seconds: Some(7200),
    };

    let result = files::renew_file_reservation_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Renewed reservation"));
}

#[tokio::test]
async fn test_file_reservation_paths_impl_single_path() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, agent_name) = setup_project_with_agent(&mm, "paths_single").await;

    let params = FileReservationPathsParams {
        project_slug,
        agent_name,
        paths: vec!["src/lib.rs".to_string()],
        exclusive: true,
        reason: Some("Single file reservation".to_string()),
        ttl_seconds: Some(3600),
    };

    let result = files::file_reservation_paths_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Granted 1 reservations"));
    assert!(output.contains("src/lib.rs"));
}

#[tokio::test]
async fn test_file_reservation_paths_impl_multiple_paths() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, agent_name) = setup_project_with_agent(&mm, "paths_multi").await;

    let params = FileReservationPathsParams {
        project_slug,
        agent_name,
        paths: vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "Cargo.toml".to_string(),
        ],
        exclusive: true,
        reason: Some("Multiple files".to_string()),
        ttl_seconds: Some(3600),
    };

    let result = files::file_reservation_paths_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Granted 3 reservations"));
}

#[tokio::test]
async fn test_file_reservation_paths_impl_conflict_detection() {
    let (mm, _temp) = create_test_mm().await;
    let _ctx = Ctx::root_ctx();

    let (project_slug, agent_name1) = setup_project_with_agent(&mm, "conflict1").await;

    let project_id = ProjectBmc::get_by_identifier(&Ctx::root_ctx(), &mm, &project_slug)
        .await
        .unwrap()
        .id;

    let agent2_c = AgentForCreate {
        project_id,
        name: "conflict_agent_2".to_string(),
        program: "claude".to_string(),
        model: "sonnet".to_string(),
        task_description: "Second agent for conflict".to_string(),
    };
    let agent2_id = AgentBmc::create(&Ctx::root_ctx(), &mm, agent2_c)
        .await
        .unwrap();

    let cap = AgentCapabilityForCreate {
        agent_id: agent2_id.into(),
        capability: "file_reservation_paths".to_string(),
        granted_by: None,
        expires_at: None,
    };
    AgentCapabilityBmc::create(&Ctx::root_ctx(), &mm, cap)
        .await
        .unwrap();

    let params1 = FileReservationPathsParams {
        project_slug: project_slug.clone(),
        agent_name: agent_name1,
        paths: vec!["src/**/*.rs".to_string()],
        exclusive: true,
        reason: Some("First agent".to_string()),
        ttl_seconds: Some(3600),
    };
    files::file_reservation_paths_impl(&Ctx::root_ctx(), &mm, params1)
        .await
        .unwrap();

    let params2 = FileReservationPathsParams {
        project_slug,
        agent_name: "conflict_agent_2".to_string(),
        paths: vec!["src/main.rs".to_string()],
        exclusive: true,
        reason: Some("Second agent conflicting".to_string()),
        ttl_seconds: Some(3600),
    };

    let result = files::file_reservation_paths_impl(&Ctx::root_ctx(), &mm, params2).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(
        output.contains("conflict") || output.contains("Conflict"),
        "Should detect conflict: {}",
        output
    );
}

#[tokio::test]
async fn test_file_reservation_paths_impl_non_exclusive() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let (project_slug, agent_name) = setup_project_with_agent(&mm, "non_exclusive").await;

    let params = FileReservationPathsParams {
        project_slug,
        agent_name,
        paths: vec!["docs/**/*.md".to_string()],
        exclusive: false,
        reason: Some("Reading docs".to_string()),
        ttl_seconds: Some(1800),
    };

    let result = files::file_reservation_paths_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());

    let output = extract_text(&result.unwrap());
    assert!(output.contains("Granted 1 reservations"));
}

#[tokio::test]
async fn test_file_reservation_paths_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = FileReservationPathsParams {
        project_slug: "nonexistent-project".to_string(),
        agent_name: "agent".to_string(),
        paths: vec!["file.rs".to_string()],
        exclusive: true,
        reason: None,
        ttl_seconds: None,
    };

    let result = files::file_reservation_paths_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("not found") || err.message.contains("Project"));
}

#[tokio::test]
async fn test_file_reservation_paths_impl_invalid_agent() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let project_slug = "paths-invalid-agent";
    ProjectBmc::create(&ctx, &mm, project_slug, "Paths Invalid Agent Project")
        .await
        .unwrap();

    let params = FileReservationPathsParams {
        project_slug: project_slug.to_string(),
        agent_name: "nonexistent_agent".to_string(),
        paths: vec!["file.rs".to_string()],
        exclusive: true,
        reason: None,
        ttl_seconds: None,
    };

    let result = files::file_reservation_paths_impl(&ctx, &mm, params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("not found"));
}
