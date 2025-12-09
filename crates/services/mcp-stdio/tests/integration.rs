//! Integration tests for MCP Agent Mail stdio service
//!
//! These tests verify end-to-end functionality of the MCP tools
//! by testing the lib-core models directly.

use std::sync::Arc;
use lib_core::ModelManager;
use tempfile::TempDir;

mod tools_tests {
    use super::*;

    /// Create a test model manager with isolated database
    async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
        use libsql::Builder;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let archive_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();

        // Create database
        let db = Builder::new_local(&db_path).build().await.unwrap();
        let conn = db.connect().unwrap();

        // Apply migrations
        let _ = conn.execute("PRAGMA journal_mode=WAL;", ()).await;
        let schema = include_str!("../../../../migrations/001_initial_schema.sql");
        conn.execute_batch(schema).await.unwrap();

        let mm = ModelManager::new_for_test(conn, archive_root);
        (Arc::new(mm), temp_dir)
    }

    #[tokio::test]
    async fn test_project_lifecycle() {
        use lib_core::model::project::ProjectBmc;
        use lib_core::Ctx;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // Create project
        let id = ProjectBmc::create(&ctx, &mm, "test-project", "/path/to/project")
            .await
            .expect("Failed to create project");
        assert!(id > 0);

        // Get project
        let project = ProjectBmc::get_by_slug(&ctx, &mm, "test-project")
            .await
            .expect("Failed to get project");
        assert_eq!(project.slug, "test-project");
        assert_eq!(project.human_key, "/path/to/project");
    }

    #[tokio::test]
    async fn test_agent_registration() {
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::project::ProjectBmc;
        use lib_core::Ctx;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // Create project first
        let project_id = ProjectBmc::create(&ctx, &mm, "agent-project", "/agent/test")
            .await
            .unwrap();

        // Register agent
        let agent_c = AgentForCreate {
            project_id,
            name: "TestAgent".to_string(),
            program: "claude-code".to_string(),
            model: "claude-3-opus".to_string(),
            task_description: "Integration test agent".to_string(),
        };

        let agent_id = AgentBmc::create(&ctx, &mm, agent_c)
            .await
            .expect("Failed to register agent");
        assert!(agent_id > 0);

        // Verify agent exists
        let agent = AgentBmc::get_by_name(&ctx, &mm, project_id, "TestAgent")
            .await
            .expect("Failed to find agent");
        assert_eq!(agent.program, "claude-code");
    }

    #[tokio::test]
    async fn test_file_reservation_workflow() {
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
        use lib_core::model::project::ProjectBmc;
        use lib_core::Ctx;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // Setup: Create project and agent
        let project_id = ProjectBmc::create(&ctx, &mm, "res-project", "/res/test")
            .await
            .unwrap();

        let agent_c = AgentForCreate {
            project_id,
            name: "ResAgent".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "Res test".to_string(),
        };
        let agent_id = AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

        // Create reservation
        let expires = chrono::Utc::now().naive_utc() + chrono::Duration::hours(1);
        let fr_c = FileReservationForCreate {
            project_id,
            agent_id,
            path_pattern: "src/**/*.rs".to_string(),
            exclusive: true,
            reason: "Integration test".to_string(),
            expires_ts: expires,
        };

        let res_id = FileReservationBmc::create(&ctx, &mm, fr_c)
            .await
            .expect("Failed to create reservation");
        assert!(res_id > 0);

        // List active reservations
        let active = FileReservationBmc::list_active_for_project(&ctx, &mm, project_id)
            .await
            .expect("Failed to list reservations");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].path_pattern, "src/**/*.rs");

        // Release reservation
        FileReservationBmc::release(&ctx, &mm, res_id)
            .await
            .expect("Failed to release reservation");

        // Verify released
        let active_after = FileReservationBmc::list_active_for_project(&ctx, &mm, project_id)
            .await
            .unwrap();
        assert_eq!(active_after.len(), 0);
    }

    #[tokio::test]
    async fn test_file_reservation_conflict() {
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
        use lib_core::model::project::ProjectBmc;
        use lib_core::Ctx;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // Setup: Create project and two agents
        let project_id = ProjectBmc::create(&ctx, &mm, "conflict-project", "/conflict/test")
            .await
            .unwrap();

        let agent_a_c = AgentForCreate {
            project_id,
            name: "AgentAlpha".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "First agent".to_string(),
        };
        let agent_a_id = AgentBmc::create(&ctx, &mm, agent_a_c).await.unwrap();

        let agent_b_c = AgentForCreate {
            project_id,
            name: "AgentBeta".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "Second agent".to_string(),
        };
        let agent_b_id = AgentBmc::create(&ctx, &mm, agent_b_c).await.unwrap();

        // Agent A reserves src/**/*.rs exclusively
        let expires = chrono::Utc::now().naive_utc() + chrono::Duration::hours(1);
        let fr_a = FileReservationForCreate {
            project_id,
            agent_id: agent_a_id,
            path_pattern: "src/**/*.rs".to_string(),
            exclusive: true,
            reason: "Agent A working on src".to_string(),
            expires_ts: expires,
        };
        let res_a_id = FileReservationBmc::create(&ctx, &mm, fr_a)
            .await
            .expect("Agent A should reserve successfully");
        assert!(res_a_id > 0);

        // Agent B tries to reserve overlapping path (should still succeed in advisory model)
        let fr_b = FileReservationForCreate {
            project_id,
            agent_id: agent_b_id,
            path_pattern: "src/main.rs".to_string(),
            exclusive: true,
            reason: "Agent B needs main.rs".to_string(),
            expires_ts: expires,
        };
        let res_b_id = FileReservationBmc::create(&ctx, &mm, fr_b)
            .await
            .expect("Agent B reservation should succeed (advisory model allows conflicts)");
        assert!(res_b_id > 0);

        // Both reservations should be active
        let active = FileReservationBmc::list_active_for_project(&ctx, &mm, project_id)
            .await
            .expect("Failed to list reservations");
        assert_eq!(active.len(), 2, "Both agents should have active reservations");

        // Force release Agent A's reservation
        FileReservationBmc::force_release(&ctx, &mm, res_a_id)
            .await
            .expect("Force release should succeed");

        // Only Agent B's reservation should remain
        let active_after = FileReservationBmc::list_active_for_project(&ctx, &mm, project_id)
            .await
            .unwrap();
        assert_eq!(active_after.len(), 1);
        assert_eq!(active_after[0].id, res_b_id);
    }

    #[tokio::test]
    async fn test_build_slot_workflow() {
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::build_slot::{BuildSlotBmc, BuildSlotForCreate};
        use lib_core::model::project::ProjectBmc;
        use lib_core::Ctx;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // Setup: Create project and agent
        let project_id = ProjectBmc::create(&ctx, &mm, "build-project", "/build/test")
            .await
            .unwrap();

        let agent_c = AgentForCreate {
            project_id,
            name: "BuildAgent".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "Build test".to_string(),
        };
        let agent_id = AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

        // Acquire build slot (uses ttl_seconds, not expires_ts)
        let slot_c = BuildSlotForCreate {
            project_id,
            agent_id,
            slot_name: "ci-main".to_string(),
            ttl_seconds: 1800, // 30 minutes
        };

        let slot_id = BuildSlotBmc::acquire(&ctx, &mm, slot_c)
            .await
            .expect("Failed to acquire build slot");
        assert!(slot_id > 0);

        // Verify slot exists via list_active
        let active = BuildSlotBmc::list_active(&ctx, &mm, project_id)
            .await
            .expect("Failed to list active slots");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].slot_name, "ci-main");

        // Release slot
        BuildSlotBmc::release(&ctx, &mm, slot_id)
            .await
            .expect("Failed to release slot");

        // Verify released
        let active_after = BuildSlotBmc::list_active(&ctx, &mm, project_id)
            .await
            .unwrap();
        assert_eq!(active_after.len(), 0);
    }

    #[tokio::test]
    async fn test_contact_workflow() {
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::agent_link::{AgentLinkBmc, AgentLinkForCreate};
        use lib_core::model::project::ProjectBmc;
        use lib_core::Ctx;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // Setup: Create two projects with agents
        let project_a_id = ProjectBmc::create(&ctx, &mm, "project-a", "/project/a")
            .await
            .unwrap();
        let project_b_id = ProjectBmc::create(&ctx, &mm, "project-b", "/project/b")
            .await
            .unwrap();

        let agent_a_c = AgentForCreate {
            project_id: project_a_id,
            name: "AgentA".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "Agent A".to_string(),
        };
        let agent_a_id = AgentBmc::create(&ctx, &mm, agent_a_c).await.unwrap();

        let agent_b_c = AgentForCreate {
            project_id: project_b_id,
            name: "AgentB".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "Agent B".to_string(),
        };
        let agent_b_id = AgentBmc::create(&ctx, &mm, agent_b_c).await.unwrap();

        // Request contact using struct
        let link_c = AgentLinkForCreate {
            a_project_id: project_a_id,
            a_agent_id: agent_a_id,
            b_project_id: project_b_id,
            b_agent_id: agent_b_id,
            reason: "Want to collaborate".to_string(),
        };
        let link_id = AgentLinkBmc::request_contact(&ctx, &mm, link_c)
            .await
            .expect("Failed to request contact");
        assert!(link_id > 0);

        // Check pending requests for Agent B
        let pending = AgentLinkBmc::list_pending_requests(&ctx, &mm, project_b_id, agent_b_id)
            .await
            .expect("Failed to list pending requests");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, "pending");

        // Accept contact
        AgentLinkBmc::respond_contact(&ctx, &mm, link_id, true)
            .await
            .expect("Failed to accept contact");

        // Verify accepted - check contacts list
        let contacts = AgentLinkBmc::list_contacts(&ctx, &mm, project_a_id, agent_a_id)
            .await
            .expect("Failed to list contacts");
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].status, "accepted");
    }

    #[tokio::test]
    async fn test_macro_workflow() {
        use lib_core::model::macro_def::{MacroDefBmc, MacroDefForCreate};
        use lib_core::model::project::ProjectBmc;
        use lib_core::Ctx;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // Setup: Create project
        let project_id = ProjectBmc::create(&ctx, &mm, "macro-project", "/macro/test")
            .await
            .unwrap();

        // Register macro
        let steps = serde_json::json!([
            {"action": "reserve_files", "pattern": "src/**/*.rs"},
            {"action": "send_message", "to": "reviewer"}
        ]);

        let macro_c = MacroDefForCreate {
            project_id,
            name: "start_review".to_string(),
            description: "Start a code review workflow".to_string(),
            steps: vec![steps],
        };

        let macro_id = MacroDefBmc::create(&ctx, &mm, macro_c)
            .await
            .expect("Failed to create macro");
        assert!(macro_id > 0);

        // List macros
        let macros = MacroDefBmc::list(&ctx, &mm, project_id)
            .await
            .expect("Failed to list macros");
        assert_eq!(macros.len(), 1);
        assert_eq!(macros[0].name, "start_review");
    }
}
