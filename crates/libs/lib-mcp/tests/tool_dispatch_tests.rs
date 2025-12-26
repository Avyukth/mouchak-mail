//! Tests for MCP tool dispatch logic
//!
//! Target: Coverage for tool alias resolution and worktree-dependent filtering
//! in lib-mcp/src/tools/mod.rs

#![allow(clippy::unwrap_used, clippy::expect_used)]

use lib_common::config::AppConfig;
use lib_core::model::ModelManager;
use lib_mcp::tools::{get_tool_schemas, AgentMailService};
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

        let common_tools = ["send_message", "list_inbox", "register_agent", "ensure_project"];

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
        let service = AgentMailService::new_with_mm(mm, false);

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
        let service = AgentMailService::new_with_mm(mm, true);

        assert!(
            service.worktrees_enabled(),
            "Service should have worktrees_enabled=true"
        );
    }
}
