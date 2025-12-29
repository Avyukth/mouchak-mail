//! Tests for build slot tool implementations
//!
//! Target: Full coverage for lib-mcp/src/tools/builds.rs

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::redundant_field_names
)]

use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::{
    ModelManager,
    agent::{AgentBmc, AgentForCreate},
    build_slot::BuildSlotBmc,
    project::ProjectBmc,
};
use mouchak_mail_mcp::tools::builds;
use mouchak_mail_mcp::tools::{AcquireBuildSlotParams, ReleaseBuildSlotParams, RenewBuildSlotParams};
use libsql::Builder;
use std::sync::Arc;
use tempfile::TempDir;

async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_builds.db");
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

async fn setup_project_and_agent(mm: &Arc<ModelManager>) -> (i64, i64, String) {
    let ctx = Ctx::root_ctx();
    let project_slug = "test-builds-project";
    let project_id = ProjectBmc::create(&ctx, mm, project_slug, "Test Builds Project")
        .await
        .unwrap();

    let agent_c = AgentForCreate {
        project_id: project_id,
        name: "build_agent".to_string(),
        program: "claude".to_string(),
        model: "opus".to_string(),
        task_description: "Build agent for testing".to_string(),
    };
    let agent_id = AgentBmc::create(&ctx, mm, agent_c).await.unwrap();

    (project_id.into(), agent_id.into(), project_slug.to_string())
}

// ==============================================================================
// acquire_build_slot_impl tests
// ==============================================================================

#[tokio::test]
async fn test_acquire_build_slot_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "build_agent".to_string(),
        slot_name: "ci-build".to_string(),
        ttl_seconds: Some(1800),
    };

    let result = builds::acquire_build_slot_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "acquire_build_slot should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Acquired build slot"));
    assert!(text.contains("ci-build"));
}

#[tokio::test]
async fn test_acquire_build_slot_impl_default_ttl() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "build_agent".to_string(),
        slot_name: "default-ttl-slot".to_string(),
        ttl_seconds: None, // Uses default 1800
    };

    let result = builds::acquire_build_slot_impl(&ctx, &mm, params).await;
    assert!(
        result.is_ok(),
        "acquire_build_slot with default TTL should succeed"
    );

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("default-ttl-slot"));
}

#[tokio::test]
async fn test_acquire_build_slot_impl_invalid_project() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = AcquireBuildSlotParams {
        project_slug: "nonexistent-project".to_string(),
        agent_name: "some_agent".to_string(),
        slot_name: "test-slot".to_string(),
        ttl_seconds: None,
    };

    let result = builds::acquire_build_slot_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for invalid project");
}

#[tokio::test]
async fn test_acquire_build_slot_impl_invalid_agent() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "nonexistent_agent".to_string(),
        slot_name: "test-slot".to_string(),
        ttl_seconds: None,
    };

    let result = builds::acquire_build_slot_impl(&ctx, &mm, params).await;
    assert!(result.is_err(), "Should fail for invalid agent");
    let err = result.unwrap_err();
    assert!(err.message.contains("Agent not found"));
}

#[tokio::test]
async fn test_acquire_build_slot_impl_multiple_slots() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    // Acquire first slot
    let params1 = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "build_agent".to_string(),
        slot_name: "slot-1".to_string(),
        ttl_seconds: Some(600),
    };
    let result1 = builds::acquire_build_slot_impl(&ctx, &mm, params1).await;
    assert!(result1.is_ok(), "First slot acquisition should succeed");

    // Acquire second slot with different name
    let params2 = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "build_agent".to_string(),
        slot_name: "slot-2".to_string(),
        ttl_seconds: Some(600),
    };
    let result2 = builds::acquire_build_slot_impl(&ctx, &mm, params2).await;
    assert!(result2.is_ok(), "Second slot acquisition should succeed");
}

#[tokio::test]
async fn test_acquire_build_slot_impl_short_ttl() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (_, _, project_slug) = setup_project_and_agent(&mm).await;

    let params = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "build_agent".to_string(),
        slot_name: "short-lived".to_string(),
        ttl_seconds: Some(60), // 1 minute
    };

    let result = builds::acquire_build_slot_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

// ==============================================================================
// renew_build_slot_impl tests
// ==============================================================================

#[tokio::test]
async fn test_renew_build_slot_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, agent_id, _) = setup_project_and_agent(&mm).await;

    // First acquire a slot to get its ID
    use mouchak_mail_core::model::build_slot::BuildSlotForCreate;
    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "renew-test".to_string(),
        ttl_seconds: 600,
    };
    let slot_id = BuildSlotBmc::acquire(&ctx, &mm, slot_c).await.unwrap();

    // Now renew it
    let params = RenewBuildSlotParams {
        slot_id,
        ttl_seconds: Some(3600),
    };

    let result = builds::renew_build_slot_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "renew_build_slot should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Renewed build slot"));
    assert!(text.contains(&slot_id.to_string()));
}

#[tokio::test]
async fn test_renew_build_slot_impl_default_ttl() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, agent_id, _) = setup_project_and_agent(&mm).await;

    // Acquire a slot
    use mouchak_mail_core::model::build_slot::BuildSlotForCreate;
    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "renew-default".to_string(),
        ttl_seconds: 600,
    };
    let slot_id = BuildSlotBmc::acquire(&ctx, &mm, slot_c).await.unwrap();

    // Renew with default TTL
    let params = RenewBuildSlotParams {
        slot_id,
        ttl_seconds: None, // Uses default 1800
    };

    let result = builds::renew_build_slot_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_renew_build_slot_impl_nonexistent_slot_is_noop() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = RenewBuildSlotParams {
        slot_id: 999999,
        ttl_seconds: Some(1800),
    };

    let result = builds::renew_build_slot_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_renew_build_slot_impl_multiple_renewals() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, agent_id, _) = setup_project_and_agent(&mm).await;

    // Acquire a slot
    use mouchak_mail_core::model::build_slot::BuildSlotForCreate;
    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "multi-renew".to_string(),
        ttl_seconds: 600,
    };
    let slot_id = BuildSlotBmc::acquire(&ctx, &mm, slot_c).await.unwrap();

    // Renew multiple times
    for i in 1..=3 {
        let params = RenewBuildSlotParams {
            slot_id,
            ttl_seconds: Some(600 * i),
        };
        let result = builds::renew_build_slot_impl(&ctx, &mm, params).await;
        assert!(result.is_ok(), "Renewal {} should succeed", i);
    }
}

// ==============================================================================
// release_build_slot_impl tests
// ==============================================================================

#[tokio::test]
async fn test_release_build_slot_impl_success() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, agent_id, _) = setup_project_and_agent(&mm).await;

    // Acquire a slot first
    use mouchak_mail_core::model::build_slot::BuildSlotForCreate;
    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "release-test".to_string(),
        ttl_seconds: 600,
    };
    let slot_id = BuildSlotBmc::acquire(&ctx, &mm, slot_c).await.unwrap();

    // Release it
    let params = ReleaseBuildSlotParams { slot_id };

    let result = builds::release_build_slot_impl(&ctx, &mm, params).await;
    assert!(result.is_ok(), "release_build_slot should succeed");

    let text = format!("{:?}", result.unwrap());
    assert!(text.contains("Released build slot"));
    assert!(text.contains(&slot_id.to_string()));
}

#[tokio::test]
async fn test_release_build_slot_impl_nonexistent_slot_is_noop() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();

    let params = ReleaseBuildSlotParams { slot_id: 999999 };

    let result = builds::release_build_slot_impl(&ctx, &mm, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_release_build_slot_impl_double_release_is_idempotent() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, agent_id, _) = setup_project_and_agent(&mm).await;

    // Acquire a slot first
    use mouchak_mail_core::model::build_slot::BuildSlotForCreate;
    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "double-release".to_string(),
        ttl_seconds: 600,
    };
    let slot_id = BuildSlotBmc::acquire(&ctx, &mm, slot_c).await.unwrap();

    // Release it first time
    let params1 = ReleaseBuildSlotParams { slot_id };
    let result1 = builds::release_build_slot_impl(&ctx, &mm, params1).await;
    assert!(result1.is_ok(), "First release should succeed");

    let params2 = ReleaseBuildSlotParams { slot_id };
    let result2 = builds::release_build_slot_impl(&ctx, &mm, params2).await;
    assert!(result2.is_ok());
}

// ==============================================================================
// Integration tests - full workflow
// ==============================================================================

#[tokio::test]
async fn test_build_slot_full_lifecycle() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, agent_id, project_slug) = setup_project_and_agent(&mm).await;

    // 1. Acquire slot via tool
    let acquire_params = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "build_agent".to_string(),
        slot_name: "lifecycle-test".to_string(),
        ttl_seconds: Some(300),
    };
    let acquire_result = builds::acquire_build_slot_impl(&ctx, &mm, acquire_params).await;
    assert!(acquire_result.is_ok());

    // Extract slot_id from the response text (crude but works for testing)
    // Actually, let's use the BMC directly to get the slot ID for renew/release
    use mouchak_mail_core::model::build_slot::BuildSlotForCreate;
    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "lifecycle-test-2".to_string(),
        ttl_seconds: 300,
    };
    let slot_id = BuildSlotBmc::acquire(&ctx, &mm, slot_c).await.unwrap();

    // 2. Renew the slot
    let renew_params = RenewBuildSlotParams {
        slot_id,
        ttl_seconds: Some(600),
    };
    let renew_result = builds::renew_build_slot_impl(&ctx, &mm, renew_params).await;
    assert!(renew_result.is_ok());

    // 3. Release the slot
    let release_params = ReleaseBuildSlotParams { slot_id };
    let release_result = builds::release_build_slot_impl(&ctx, &mm, release_params).await;
    assert!(release_result.is_ok());

    let renew_after_release = RenewBuildSlotParams {
        slot_id,
        ttl_seconds: Some(300),
    };
    let result = builds::renew_build_slot_impl(&ctx, &mm, renew_after_release).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_acquire_build_slot_impl_slot_conflict() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent(&mm).await;

    let agent2_c = AgentForCreate {
        project_id: project_id.into(),
        name: "conflict_agent".to_string(),
        program: "claude".to_string(),
        model: "sonnet".to_string(),
        task_description: "Conflict test agent".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent2_c).await.unwrap();

    let params1 = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "build_agent".to_string(),
        slot_name: "exclusive-slot".to_string(),
        ttl_seconds: Some(3600),
    };
    let result1 = builds::acquire_build_slot_impl(&ctx, &mm, params1).await;
    assert!(result1.is_ok(), "First acquisition should succeed");

    let params2 = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "conflict_agent".to_string(),
        slot_name: "exclusive-slot".to_string(),
        ttl_seconds: Some(3600),
    };
    let result2 = builds::acquire_build_slot_impl(&ctx, &mm, params2).await;
    assert!(
        result2.is_err(),
        "Second acquisition should fail - slot already held"
    );
}

#[tokio::test]
async fn test_acquire_build_slot_impl_with_second_agent() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, _, project_slug) = setup_project_and_agent(&mm).await;

    // Create a second agent
    let agent2_c = AgentForCreate {
        project_id: project_id.into(),
        name: "build_agent_2".to_string(),
        program: "claude".to_string(),
        model: "sonnet".to_string(),
        task_description: "Second build agent".to_string(),
    };
    AgentBmc::create(&ctx, &mm, agent2_c).await.unwrap();

    // Both agents can acquire different slots
    let params1 = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "build_agent".to_string(),
        slot_name: "agent1-slot".to_string(),
        ttl_seconds: Some(600),
    };
    let result1 = builds::acquire_build_slot_impl(&ctx, &mm, params1).await;
    assert!(result1.is_ok());

    let params2 = AcquireBuildSlotParams {
        project_slug: project_slug.clone(),
        agent_name: "build_agent_2".to_string(),
        slot_name: "agent2-slot".to_string(),
        ttl_seconds: Some(600),
    };
    let result2 = builds::acquire_build_slot_impl(&ctx, &mm, params2).await;
    assert!(result2.is_ok());
}
