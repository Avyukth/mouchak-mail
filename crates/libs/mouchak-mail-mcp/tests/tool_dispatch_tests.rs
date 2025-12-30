//! Tests for MCP tool dispatch logic
//!
//! Target: Coverage for tool alias resolution and worktree-dependent filtering
//! in lib-mcp/src/tools/mod.rs

#![allow(clippy::unwrap_used, clippy::expect_used)]

use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::model::ModelManager;
use mouchak_mail_mcp::tools::{MouchakMailService, get_tool_schemas};
use std::sync::Arc;

const BUILD_SLOT_TOOLS: &[&str] = &[
    "acquire_build_slot",
    "release_build_slot",
    "renew_build_slot",
];

mod get_tool_schemas_filtering {
    use super::*;

    #[test]
    fn excludes_build_slots_when_worktrees_disabled() {
        let schemas = get_tool_schemas(false);
        let tool_names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();

        for build_slot_tool in BUILD_SLOT_TOOLS {
            assert!(
                !tool_names.contains(build_slot_tool),
                "Tool '{}' should be excluded when worktrees disabled, but found in: {:?}",
                build_slot_tool,
                tool_names
            );
        }
    }

    #[test]
    fn includes_build_slots_when_worktrees_enabled() {
        let schemas = get_tool_schemas(true);
        let tool_names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();

        for build_slot_tool in BUILD_SLOT_TOOLS {
            assert!(
                tool_names.contains(build_slot_tool),
                "Tool '{}' should be included when worktrees enabled, but not found in: {:?}",
                build_slot_tool,
                tool_names
            );
        }
    }

    #[test]
    fn includes_non_build_slot_tools_regardless_of_worktrees() {
        let schemas_disabled = get_tool_schemas(false);
        let schemas_enabled = get_tool_schemas(true);

        let common_tools = [
            "send_message",
            "list_inbox",
            "register_agent",
            "ensure_project",
        ];

        for tool in common_tools {
            let found_disabled = schemas_disabled.iter().any(|s| s.name == tool);
            let found_enabled = schemas_enabled.iter().any(|s| s.name == tool);

            assert!(
                found_disabled,
                "Tool '{}' should be present when worktrees disabled",
                tool
            );
            assert!(
                found_enabled,
                "Tool '{}' should be present when worktrees enabled",
                tool
            );
        }
    }

    #[test]
    fn returns_non_empty_list() {
        let schemas = get_tool_schemas(false);
        assert!(
            !schemas.is_empty(),
            "get_tool_schemas should return at least some tools"
        );
        assert!(
            schemas.len() > 10,
            "Expected many tools, got only {}",
            schemas.len()
        );
    }
}

mod service_worktrees_config {
    use super::*;

    #[tokio::test]
    async fn service_respects_worktrees_enabled_false() {
        let mm = Arc::new(
            ModelManager::new(Arc::new(AppConfig::default()))
                .await
                .expect("Failed to create ModelManager"),
        );
        let service = MouchakMailService::new_with_mm(mm, false);

        assert!(
            !service.worktrees_enabled(),
            "Service should have worktrees_enabled=false"
        );
    }

    #[tokio::test]
    async fn service_respects_worktrees_enabled_true() {
        let mm = Arc::new(
            ModelManager::new(Arc::new(AppConfig::default()))
                .await
                .expect("Failed to create ModelManager"),
        );
        let service = MouchakMailService::new_with_mm(mm, true);

        assert!(
            service.worktrees_enabled(),
            "Service should have worktrees_enabled=true"
        );
    }
}

mod tool_alias_resolution {
    use super::*;

    #[test]
    fn fetch_inbox_resolves_to_list_inbox() {
        let resolved = MouchakMailService::resolve_tool_alias("fetch_inbox");
        assert_eq!(resolved, Some("list_inbox"));
    }

    #[test]
    fn check_inbox_resolves_to_list_inbox() {
        let resolved = MouchakMailService::resolve_tool_alias("check_inbox");
        assert_eq!(resolved, Some("list_inbox"));
    }

    #[test]
    fn release_file_reservations_resolves() {
        let resolved = MouchakMailService::resolve_tool_alias("release_file_reservations");
        assert_eq!(resolved, Some("release_file_reservations_by_path"));
    }

    #[test]
    fn renew_file_reservations_resolves() {
        let resolved = MouchakMailService::resolve_tool_alias("renew_file_reservations");
        assert_eq!(resolved, Some("renew_file_reservations_by_agent"));
    }

    #[test]
    fn list_file_reservations_resolves() {
        let resolved = MouchakMailService::resolve_tool_alias("list_file_reservations");
        assert_eq!(resolved, Some("list_reservations"));
    }

    #[test]
    fn list_project_agents_resolves() {
        let resolved = MouchakMailService::resolve_tool_alias("list_project_agents");
        assert_eq!(resolved, Some("list_agents"));
    }

    #[test]
    fn unknown_tool_returns_none() {
        let resolved = MouchakMailService::resolve_tool_alias("send_message");
        assert_eq!(resolved, None);

        let resolved = MouchakMailService::resolve_tool_alias("nonexistent_tool");
        assert_eq!(resolved, None);
    }
}

mod server_handler_list_tools {
    use super::*;

    #[tokio::test]
    async fn excludes_build_slot_tools_when_worktrees_disabled() {
        let mm = Arc::new(
            ModelManager::new(Arc::new(AppConfig::default()))
                .await
                .expect("Failed to create ModelManager"),
        );
        let service = MouchakMailService::new_with_mm(mm, false);

        let tools = service.list_tools_filtered();
        let tool_names: Vec<&str> = tools.iter().map(|t| &*t.name).collect();

        for build_slot_tool in BUILD_SLOT_TOOLS {
            assert!(
                !tool_names.contains(build_slot_tool),
                "list_tools_filtered should exclude '{}' when worktrees disabled, found in: {:?}",
                build_slot_tool,
                tool_names
            );
        }
    }

    #[tokio::test]
    async fn includes_build_slot_tools_when_worktrees_enabled() {
        let mm = Arc::new(
            ModelManager::new(Arc::new(AppConfig::default()))
                .await
                .expect("Failed to create ModelManager"),
        );
        let service = MouchakMailService::new_with_mm(mm, true);

        let tools = service.list_tools_filtered();
        let tool_names: Vec<&str> = tools.iter().map(|t| &*t.name).collect();

        for build_slot_tool in BUILD_SLOT_TOOLS {
            assert!(
                tool_names.contains(build_slot_tool),
                "list_tools_filtered should include '{}' when worktrees enabled, not found in: {:?}",
                build_slot_tool,
                tool_names
            );
        }
    }
}

mod build_slot_rejection {
    use super::*;

    #[tokio::test]
    async fn rejects_build_slot_tools_when_worktrees_disabled() {
        let mm = Arc::new(
            ModelManager::new(Arc::new(AppConfig::default()))
                .await
                .expect("Failed to create ModelManager"),
        );
        let service = MouchakMailService::new_with_mm(mm, false);

        for build_slot_tool in BUILD_SLOT_TOOLS {
            let rejection = service.check_build_slot_rejection(build_slot_tool);
            assert!(
                rejection.is_some(),
                "Tool '{}' should be rejected when worktrees disabled",
                build_slot_tool
            );

            let err_msg = rejection.unwrap();
            assert!(
                err_msg.contains("WORKTREES_ENABLED"),
                "Error for '{}' should mention WORKTREES_ENABLED, got: {}",
                build_slot_tool,
                err_msg
            );
        }
    }

    #[tokio::test]
    async fn allows_build_slot_tools_when_worktrees_enabled() {
        let mm = Arc::new(
            ModelManager::new(Arc::new(AppConfig::default()))
                .await
                .expect("Failed to create ModelManager"),
        );
        let service = MouchakMailService::new_with_mm(mm, true);

        for build_slot_tool in BUILD_SLOT_TOOLS {
            let rejection = service.check_build_slot_rejection(build_slot_tool);
            assert!(
                rejection.is_none(),
                "Tool '{}' should NOT be rejected when worktrees enabled, got: {:?}",
                build_slot_tool,
                rejection
            );
        }
    }

    #[tokio::test]
    async fn allows_non_build_slot_tools_regardless_of_worktrees() {
        let mm = Arc::new(
            ModelManager::new(Arc::new(AppConfig::default()))
                .await
                .expect("Failed to create ModelManager"),
        );
        let service_disabled = MouchakMailService::new_with_mm(mm.clone(), false);
        let service_enabled = MouchakMailService::new_with_mm(mm, true);

        let non_build_tools = ["send_message", "list_inbox", "register_agent"];

        for tool in non_build_tools {
            assert!(
                service_disabled.check_build_slot_rejection(tool).is_none(),
                "Tool '{}' should not be rejected when worktrees disabled",
                tool
            );
            assert!(
                service_enabled.check_build_slot_rejection(tool).is_none(),
                "Tool '{}' should not be rejected when worktrees enabled",
                tool
            );
        }
    }
}
