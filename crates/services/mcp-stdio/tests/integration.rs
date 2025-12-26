//! Integration tests for MCP Agent Mail stdio service
//!
//! These tests verify end-to-end functionality of the MCP tools
//! by testing the lib-core models directly.

use lib_common::config::AppConfig;
use lib_core::ModelManager;
use std::sync::Arc;
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
        let schema002 = include_str!("../../../../migrations/002_agent_capabilities.sql");
        conn.execute_batch(schema002).await.unwrap();
        let schema3 = include_str!("../../../../migrations/003_tool_metrics.sql");
        conn.execute_batch(schema3).await.unwrap();
        let schema4 = include_str!("../../../../migrations/004_attachments.sql");
        conn.execute_batch(schema4).await.unwrap();

        let app_config = Arc::new(AppConfig::default());
        let mm = ModelManager::new_for_test(conn, archive_root, app_config);
        (Arc::new(mm), temp_dir)
    }

    #[tokio::test]
    async fn test_project_lifecycle() {
        use lib_core::Ctx;
        use lib_core::model::project::ProjectBmc;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // Create project
        let id = ProjectBmc::create(&ctx, &mm, "test-project", "/path/to/project")
            .await
            .expect("Failed to create project");
        assert!(i64::from(id) > 0);

        // Get project
        let project = ProjectBmc::get_by_slug(&ctx, &mm, "test-project")
            .await
            .expect("Failed to get project");
        assert_eq!(project.slug, "test-project");
        assert_eq!(project.human_key, "/path/to/project");
    }

    #[tokio::test]
    async fn test_agent_registration() {
        use lib_core::Ctx;
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::project::ProjectBmc;

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
            program: "antigravity".to_string(),
            model: "gemini-2.0-pro".to_string(),
            task_description: "Integration test agent".to_string(),
        };

        let agent_id = AgentBmc::create(&ctx, &mm, agent_c)
            .await
            .expect("Failed to register agent");
        assert!(i64::from(agent_id) > 0);

        // Verify agent exists
        let agent = AgentBmc::get_by_name(&ctx, &mm, project_id, "TestAgent")
            .await
            .expect("Failed to find agent");
        assert_eq!(agent.program, "antigravity");
    }

    #[tokio::test]
    async fn test_file_reservation_workflow() {
        use lib_core::Ctx;
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
        use lib_core::model::project::ProjectBmc;

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
        assert!(i64::from(res_id) > 0);

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
        use lib_core::Ctx;
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
        use lib_core::model::project::ProjectBmc;

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
        assert!(i64::from(res_a_id) > 0);

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
        assert!(i64::from(res_b_id) > 0);

        // Both reservations should be active
        let active = FileReservationBmc::list_active_for_project(&ctx, &mm, project_id)
            .await
            .expect("Failed to list reservations");
        assert_eq!(
            active.len(),
            2,
            "Both agents should have active reservations"
        );

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
        use lib_core::Ctx;
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::build_slot::{BuildSlotBmc, BuildSlotForCreate};
        use lib_core::model::project::ProjectBmc;

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
            project_id: project_id.into(),
            agent_id: agent_id.into(),
            slot_name: "ci-main".to_string(),
            ttl_seconds: 1800, // 30 minutes
        };

        let slot_id = BuildSlotBmc::acquire(&ctx, &mm, slot_c)
            .await
            .expect("Failed to acquire build slot");
        assert!(i64::from(slot_id) > 0);

        // Verify slot exists via list_active
        let active = BuildSlotBmc::list_active(&ctx, &mm, project_id.into())
            .await
            .expect("Failed to list active slots");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].slot_name, "ci-main");

        // Release slot
        BuildSlotBmc::release(&ctx, &mm, slot_id)
            .await
            .expect("Failed to release slot");

        // Verify released
        let active_after = BuildSlotBmc::list_active(&ctx, &mm, project_id.into())
            .await
            .unwrap();
        assert_eq!(active_after.len(), 0);
    }

    #[tokio::test]
    async fn test_contact_workflow() {
        use lib_core::Ctx;
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::agent_link::{AgentLinkBmc, AgentLinkForCreate};
        use lib_core::model::project::ProjectBmc;

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
            a_project_id: project_a_id.into(),
            a_agent_id: agent_a_id.into(),
            b_project_id: project_b_id.into(),
            b_agent_id: agent_b_id.into(),
            reason: "Want to collaborate".to_string(),
        };
        let link_id = AgentLinkBmc::request_contact(&ctx, &mm, link_c)
            .await
            .expect("Failed to request contact");
        assert!(i64::from(link_id) > 0);

        // Check pending requests for Agent B
        let pending =
            AgentLinkBmc::list_pending_requests(&ctx, &mm, project_b_id.into(), agent_b_id.into())
                .await
                .expect("Failed to list pending requests");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, "pending");

        // Accept contact
        AgentLinkBmc::respond_contact(&ctx, &mm, link_id, true)
            .await
            .expect("Failed to accept contact");

        // Verify accepted - check contacts list
        let contacts =
            AgentLinkBmc::list_contacts(&ctx, &mm, project_a_id.into(), agent_a_id.into())
                .await
                .expect("Failed to list contacts");
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].status, "accepted");
    }

    #[tokio::test]
    async fn test_macro_workflow() {
        use lib_core::Ctx;
        use lib_core::model::macro_def::{MacroDefBmc, MacroDefForCreate};
        use lib_core::model::project::ProjectBmc;

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
            project_id: project_id.into(),
            name: "start_review".to_string(),
            description: "Start a code review workflow".to_string(),
            steps: vec![steps],
        };

        let macro_id = MacroDefBmc::create(&ctx, &mm, macro_c)
            .await
            .expect("Failed to create macro");
        assert!(i64::from(macro_id) > 0);

        // List macros
        let macros = MacroDefBmc::list(&ctx, &mm, project_id.into())
            .await
            .expect("Failed to list macros");
        assert_eq!(macros.len(), 6);
        assert!(macros.iter().any(|m| m.name == "start_review"));
    }

    #[tokio::test]
    async fn test_product_workflow() {
        use lib_core::Ctx;
        use lib_core::model::product::ProductBmc;
        use lib_core::model::project::ProjectBmc;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // Create a product (multi-repo coordinator)
        let product = ProductBmc::ensure(&ctx, &mm, "enterprise-suite", "Enterprise Suite")
            .await
            .expect("Failed to ensure product");
        assert!(product.id > 0);
        assert_eq!(product.product_uid, "enterprise-suite");

        // Create two projects
        let project_a_id = ProjectBmc::create(&ctx, &mm, "frontend", "/enterprise/frontend")
            .await
            .unwrap();
        let project_b_id = ProjectBmc::create(&ctx, &mm, "backend", "/enterprise/backend")
            .await
            .unwrap();

        // Link both projects to the product
        let link_a = ProductBmc::link_project(&ctx, &mm, product.id, project_a_id.into())
            .await
            .expect("Failed to link frontend");
        assert!(link_a > 0);

        let link_b = ProductBmc::link_project(&ctx, &mm, product.id, project_b_id.into())
            .await
            .expect("Failed to link backend");
        assert!(link_b > 0);

        // Get linked projects
        let linked = ProductBmc::get_linked_projects(&ctx, &mm, product.id)
            .await
            .expect("Failed to get linked projects");
        assert_eq!(linked.len(), 2);
        assert!(linked.contains(&project_a_id.into()));
        assert!(linked.contains(&project_b_id.into()));

        // List all products with their linked projects
        let all_products = ProductBmc::list_all(&ctx, &mm)
            .await
            .expect("Failed to list products");
        assert_eq!(all_products.len(), 1);
        assert_eq!(all_products[0].project_ids.len(), 2);

        // Unlink a project
        let unlinked = ProductBmc::unlink_project(&ctx, &mm, product.id, project_a_id.into())
            .await
            .expect("Failed to unlink");
        assert!(unlinked);

        // Verify only one project remains linked
        let linked_after = ProductBmc::get_linked_projects(&ctx, &mm, product.id)
            .await
            .unwrap();
        assert_eq!(linked_after.len(), 1);
        assert_eq!(linked_after[0], i64::from(project_b_id));
    }

    #[tokio::test]
    async fn test_export_mailbox() {
        use lib_core::Ctx;
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::export::{ExportBmc, ExportFormat, ScrubMode};
        use lib_core::model::message::{MessageBmc, MessageForCreate};
        use lib_core::model::project::ProjectBmc;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // Setup: Create project, agents, and messages
        let project_id = ProjectBmc::create(&ctx, &mm, "export-test", "/export/test")
            .await
            .unwrap();

        let agent_c = AgentForCreate {
            project_id,
            name: "ExportAgent".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "Export test".to_string(),
        };
        let sender_id = AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

        let recipient_c = AgentForCreate {
            project_id,
            name: "Recipient".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "Recipient".to_string(),
        };
        let recipient_id = AgentBmc::create(&ctx, &mm, recipient_c).await.unwrap();

        // Send a message
        let msg_c = MessageForCreate {
            project_id: project_id.into(),
            sender_id: sender_id.into(),
            recipient_ids: vec![recipient_id.into()],
            cc_ids: None,
            bcc_ids: None,
            subject: "Test Export Message".to_string(),
            body_md: "This message should appear in the export.".to_string(),
            thread_id: None,
            importance: None,
            ack_required: false,
        };
        MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

        // Export as JSON
        let json_export = ExportBmc::export_mailbox(
            &ctx,
            &mm,
            "export-test",
            ExportFormat::Json,
            ScrubMode::None,
            false,
        )
        .await
        .expect("JSON export should succeed");
        assert_eq!(json_export.format, "json");
        assert_eq!(json_export.message_count, 1);
        assert!(json_export.content.contains("Test Export Message"));

        // Export as HTML
        let html_export = ExportBmc::export_mailbox(
            &ctx,
            &mm,
            "export-test",
            ExportFormat::Html,
            ScrubMode::None,
            false,
        )
        .await
        .expect("HTML export should succeed");
        assert_eq!(html_export.format, "html");
        assert!(html_export.content.contains("<html>"));
        assert!(html_export.content.contains("Test Export Message"));

        // Export as Markdown
        let md_export = ExportBmc::export_mailbox(
            &ctx,
            &mm,
            "export-test",
            ExportFormat::Markdown,
            ScrubMode::None,
            false,
        )
        .await
        .expect("Markdown export should succeed");
        assert_eq!(md_export.format, "markdown");
        assert!(md_export.content.contains("# Mailbox Export:"));
        assert!(md_export.content.contains("Test Export Message"));
    }

    #[tokio::test]
    async fn test_git_archive_workflow() {
        use lib_core::Ctx;
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::message::{MessageBmc, MessageForCreate};
        use lib_core::model::project::ProjectBmc;

        let (mm, _temp) = create_test_mm().await;
        let ctx = Ctx::root_ctx();

        // 1. Create project
        let project_id = ProjectBmc::create(&ctx, &mm, "archive-test", "/archive/test")
            .await
            .expect("Failed to create project");

        // 2. Create agent & message (so we have content)
        let agent_c = AgentForCreate {
            project_id,
            name: "Archiver".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: "Archive test".to_string(),
        };
        let agent_id = AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

        let msg_c = MessageForCreate {
            project_id: project_id.into(),
            sender_id: agent_id.into(),
            recipient_ids: vec![agent_id.into()], // self
            cc_ids: None,
            bcc_ids: None,
            subject: "Archive Me".to_string(),
            body_md: "Content to be archived".to_string(),
            thread_id: None,
            importance: None,
            ack_required: false,
        };
        MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

        // 3. Sync to archive
        let oid = ProjectBmc::sync_to_archive(&ctx, &mm, project_id, "Test archive commit")
            .await
            .expect("Sync should succeed");

        println!("Commit OID: {}", oid);
        assert!(!oid.is_empty());

        // 4. Verify file exists in repo (we can inspect the temp dir directly)
        // mm.repo_root is inside _temp
        let mailbox_path = mm
            .repo_root
            .join("projects")
            .join("archive-test")
            .join("mailbox.json");
        assert!(mailbox_path.exists());

        let content = std::fs::read_to_string(mailbox_path).unwrap();
        assert!(content.contains("Archive Me"));
    }
}
