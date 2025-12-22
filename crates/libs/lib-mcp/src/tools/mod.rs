//! MCP Tool implementations for Agent Mail
//!
//! This module defines all MCP tools that wrap the lib-core functionality.

use anyhow::Result;
use lib_common::config::AppConfig;
use rmcp::{
    ErrorData as McpError,
    handler::server::{ServerHandler, tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolRequestParam, CallToolResult, Content, ListResourcesResult, ListToolsResult,
        PaginatedRequestParam, RawResource, ReadResourceRequestParam, ReadResourceResult, Resource,
        ResourceContents,
    },
    service::{RequestContext, RoleServer},
    tool, tool_router,
};
use serde::Serialize;
use std::sync::Arc;

use lib_core::{
    ctx::Ctx,
    model::{
        ModelManager, agent::AgentBmc, agent_capabilities::AgentCapabilityBmc,
        file_reservation::FileReservationBmc, message::MessageBmc, product::ProductBmc,
        project::ProjectBmc,
    },
};

pub mod agent;
pub mod archive;
pub mod attachments;
pub mod builds;
pub mod contacts;
pub mod files;
pub mod helpers;
pub mod observability;
pub mod outbox;
mod params;
pub mod project;
pub mod reviews;

pub use params::*;

// ============================================================================
// Schema Export Types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterSchema>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParameterSchema {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

/// Names of build slot tools (conditional on worktrees_enabled)
const BUILD_SLOT_TOOLS: &[&str] = &[
    "acquire_build_slot",
    "release_build_slot",
    "renew_build_slot",
];

/// Get schema information for all tools
///
/// When `worktrees_enabled` is false, build slot tools are excluded from the list.
pub fn get_tool_schemas(worktrees_enabled: bool) -> Vec<ToolSchema> {
    get_all_tool_schemas()
        .into_iter()
        .filter(|schema| worktrees_enabled || !BUILD_SLOT_TOOLS.contains(&schema.name.as_str()))
        .collect()
}

/// Get all tool schemas regardless of configuration
fn get_all_tool_schemas() -> Vec<ToolSchema> {
    vec![
        ToolSchema {
            name: "ensure_project".into(),
            description: "Ensure a project exists, creating it if necessary.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug (URL-safe identifier)".into(),
                },
                ParameterSchema {
                    name: "human_key".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Human-readable project name".into(),
                },
            ],
        },
        ToolSchema {
            name: "register_agent".into(),
            description: "Register a new agent in a project.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name".into(),
                },
                ParameterSchema {
                    name: "program".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent program name".into(),
                },
                ParameterSchema {
                    name: "model".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "AI model used".into(),
                },
                ParameterSchema {
                    name: "task_description".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent's task description".into(),
                },
            ],
        },
        ToolSchema {
            name: "send_message".into(),
            description: "Send a message from one agent to others.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "sender_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Sender agent name".into(),
                },
                ParameterSchema {
                    name: "to".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Recipient agent names (comma-separated)".into(),
                },
                ParameterSchema {
                    name: "cc".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "CC recipient agent names (comma-separated)".into(),
                },
                ParameterSchema {
                    name: "bcc".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "BCC recipient agent names (comma-separated)".into(),
                },
                ParameterSchema {
                    name: "subject".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Message subject".into(),
                },
                ParameterSchema {
                    name: "body_md".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Message body in markdown".into(),
                },
                ParameterSchema {
                    name: "importance".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Message importance level".into(),
                },
            ],
        },
        ToolSchema {
            name: "check_inbox".into(),
            description: "Check an agent's inbox for new messages.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name".into(),
                },
                ParameterSchema {
                    name: "limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Maximum messages to return".into(),
                },
            ],
        },
        ToolSchema {
            name: "reply_message".into(),
            description: "Reply to an existing message in a thread.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "sender_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Sender agent name".into(),
                },
                ParameterSchema {
                    name: "message_id".into(),
                    param_type: "integer".into(),
                    required: true,
                    description: "Message ID to reply to".into(),
                },
                ParameterSchema {
                    name: "body_md".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Reply body in markdown".into(),
                },
            ],
        },
        ToolSchema {
            name: "list_projects".into(),
            description: "List all projects.".into(),
            parameters: vec![],
        },
        ToolSchema {
            name: "list_agents".into(),
            description: "List all agents in a project.".into(),
            parameters: vec![ParameterSchema {
                name: "project_slug".into(),
                param_type: "string".into(),
                required: true,
                description: "Project slug".into(),
            }],
        },
        ToolSchema {
            name: "get_message".into(),
            description: "Get a specific message by ID.".into(),
            parameters: vec![ParameterSchema {
                name: "message_id".into(),
                param_type: "integer".into(),
                required: true,
                description: "Message ID".into(),
            }],
        },
        ToolSchema {
            name: "search_messages".into(),
            description: "Full-text search messages.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "query".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Search query".into(),
                },
                ParameterSchema {
                    name: "limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Maximum results".into(),
                },
            ],
        },
        ToolSchema {
            name: "reserve_file".into(),
            description: "Reserve a file path for exclusive editing.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name".into(),
                },
                ParameterSchema {
                    name: "path_pattern".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "File path or glob pattern".into(),
                },
                ParameterSchema {
                    name: "reason".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Reason for reservation".into(),
                },
                ParameterSchema {
                    name: "ttl_minutes".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Time-to-live in minutes".into(),
                },
            ],
        },
        ToolSchema {
            name: "release_reservation".into(),
            description: "Release a file reservation by ID.".into(),
            parameters: vec![ParameterSchema {
                name: "reservation_id".into(),
                param_type: "integer".into(),
                required: true,
                description: "Reservation ID".into(),
            }],
        },
        ToolSchema {
            name: "list_file_reservations".into(),
            description: "List active file reservations in a project.".into(),
            parameters: vec![ParameterSchema {
                name: "project_slug".into(),
                param_type: "string".into(),
                required: true,
                description: "Project slug".into(),
            }],
        },
        ToolSchema {
            name: "force_release_reservation".into(),
            description: "Force release a file reservation (emergency override).".into(),
            parameters: vec![ParameterSchema {
                name: "reservation_id".into(),
                param_type: "integer".into(),
                required: true,
                description: "Reservation ID".into(),
            }],
        },
        ToolSchema {
            name: "renew_file_reservation".into(),
            description: "Extend the TTL of a file reservation.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "reservation_id".into(),
                    param_type: "integer".into(),
                    required: true,
                    description: "Reservation ID".into(),
                },
                ParameterSchema {
                    name: "ttl_seconds".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "New TTL in seconds".into(),
                },
            ],
        },
        ToolSchema {
            name: "request_contact".into(),
            description: "Request to add another agent as a contact.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "from_project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "From project slug".into(),
                },
                ParameterSchema {
                    name: "from_agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "From agent name".into(),
                },
                ParameterSchema {
                    name: "to_project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "To project slug".into(),
                },
                ParameterSchema {
                    name: "to_agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "To agent name".into(),
                },
                ParameterSchema {
                    name: "reason".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Reason for contact request".into(),
                },
            ],
        },
        ToolSchema {
            name: "respond_contact".into(),
            description: "Accept or reject a contact request.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "link_id".into(),
                    param_type: "integer".into(),
                    required: true,
                    description: "Agent link ID".into(),
                },
                ParameterSchema {
                    name: "accept".into(),
                    param_type: "boolean".into(),
                    required: true,
                    description: "Accept or reject".into(),
                },
            ],
        },
        ToolSchema {
            name: "list_contacts".into(),
            description: "List all contacts for an agent.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name".into(),
                },
            ],
        },
        ToolSchema {
            name: "acquire_build_slot".into(),
            description: "Acquire an exclusive build slot for CI/CD isolation.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name".into(),
                },
                ParameterSchema {
                    name: "slot_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Slot name".into(),
                },
                ParameterSchema {
                    name: "ttl_seconds".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "TTL in seconds".into(),
                },
            ],
        },
        ToolSchema {
            name: "release_build_slot".into(),
            description: "Release a held build slot.".into(),
            parameters: vec![ParameterSchema {
                name: "slot_id".into(),
                param_type: "integer".into(),
                required: true,
                description: "Slot ID".into(),
            }],
        },
        ToolSchema {
            name: "list_macros".into(),
            description: "List all available macros in a project.".into(),
            parameters: vec![ParameterSchema {
                name: "project_slug".into(),
                param_type: "string".into(),
                required: true,
                description: "Project slug".into(),
            }],
        },
        ToolSchema {
            name: "register_macro".into(),
            description: "Register a new macro definition.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Macro name".into(),
                },
                ParameterSchema {
                    name: "description".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Macro description".into(),
                },
                ParameterSchema {
                    name: "steps".into(),
                    param_type: "array".into(),
                    required: true,
                    description: "Macro steps as JSON array".into(),
                },
            ],
        },
        ToolSchema {
            name: "invoke_macro".into(),
            description: "Execute a pre-defined macro and get its steps.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Macro name".into(),
                },
            ],
        },
        ToolSchema {
            name: "ensure_product".into(),
            description: "Create or get a product for multi-repo coordination.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "product_uid".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Product UID".into(),
                },
                ParameterSchema {
                    name: "name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Product name".into(),
                },
            ],
        },
        ToolSchema {
            name: "link_project_to_product".into(),
            description: "Link a project to a product for unified messaging.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "product_uid".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Product UID".into(),
                },
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
            ],
        },
        ToolSchema {
            name: "list_products".into(),
            description: "List all products and their linked projects.".into(),
            parameters: vec![],
        },
        ToolSchema {
            name: "product_inbox".into(),
            description: "Get aggregated inbox across all projects in a product.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "product_uid".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Product UID".into(),
                },
                ParameterSchema {
                    name: "limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Max messages per project".into(),
                },
            ],
        },
        ToolSchema {
            name: "export_mailbox".into(),
            description: "Export a project's mailbox to HTML, JSON, or Markdown format.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "format".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Export format: html, json, or markdown".into(),
                },
            ],
        },
        ToolSchema {
            name: "get_project_info".into(),
            description: "Get detailed information about a project.".into(),
            parameters: vec![ParameterSchema {
                name: "project_slug".into(),
                param_type: "string".into(),
                required: true,
                description: "Project slug".into(),
            }],
        },
        ToolSchema {
            name: "get_agent_profile".into(),
            description: "Get detailed profile information for an agent.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name".into(),
                },
            ],
        },
        ToolSchema {
            name: "list_threads".into(),
            description: "List all conversation threads in a project.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Maximum threads".into(),
                },
            ],
        },
        ToolSchema {
            name: "summarize_thread".into(),
            description: "Summarize one or more conversation threads. Accepts single thread_id (string) or multiple (array). Partial failures returned in errors array.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "thread_id".into(),
                    param_type: "string|array".into(),
                    required: true,
                    description: "Thread ID (string) or array of thread IDs".into(),
                },
            ],
        },
        ToolSchema {
            name: "mark_message_read".into(),
            description: "Mark a message as read by a recipient.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name marking as read".into(),
                },
                ParameterSchema {
                    name: "message_id".into(),
                    param_type: "integer".into(),
                    required: true,
                    description: "Message ID".into(),
                },
            ],
        },
        ToolSchema {
            name: "acknowledge_message".into(),
            description: "Acknowledge a message (sets both read and acknowledged).".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name acknowledging".into(),
                },
                ParameterSchema {
                    name: "message_id".into(),
                    param_type: "integer".into(),
                    required: true,
                    description: "Message ID".into(),
                },
            ],
        },
        ToolSchema {
            name: "set_contact_policy".into(),
            description: "Set an agent's contact policy (open, auto, contacts_only, block_all)."
                .into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name".into(),
                },
                ParameterSchema {
                    name: "contact_policy".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Policy: open, auto, contacts_only, block_all".into(),
                },
            ],
        },
        ToolSchema {
            name: "renew_build_slot".into(),
            description: "Extend the TTL of a build slot.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "slot_id".into(),
                    param_type: "integer".into(),
                    required: true,
                    description: "Slot ID".into(),
                },
                ParameterSchema {
                    name: "ttl_seconds".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "New TTL in seconds".into(),
                },
            ],
        },
        ToolSchema {
            name: "list_outbox".into(),
            description: "List messages in an agent's outbox (sent messages).".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name".into(),
                },
                ParameterSchema {
                    name: "limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Maximum messages to return".into(),
                },
            ],
        },
        ToolSchema {
            name: "file_reservation_paths".into(),
            description: "Reserve multiple file paths with conflict detection.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name".into(),
                },
                ParameterSchema {
                    name: "paths".into(),
                    param_type: "array".into(),
                    required: true,
                    description: "File paths to reserve".into(),
                },
                ParameterSchema {
                    name: "exclusive".into(),
                    param_type: "boolean".into(),
                    required: true,
                    description: "Whether reservation is exclusive".into(),
                },
                ParameterSchema {
                    name: "reason".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Reason for reservation".into(),
                },
                ParameterSchema {
                    name: "ttl_seconds".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "TTL in seconds".into(),
                },
            ],
        },
        ToolSchema {
            name: "install_precommit_guard".into(),
            description: "Install pre-commit guard for file reservation checks.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "target_repo_path".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Target repository path".into(),
                },
            ],
        },
        ToolSchema {
            name: "uninstall_precommit_guard".into(),
            description: "Uninstall pre-commit guard.".into(),
            parameters: vec![ParameterSchema {
                name: "target_repo_path".into(),
                param_type: "string".into(),
                required: true,
                description: "Target repository path".into(),
            }],
        },
        ToolSchema {
            name: "add_attachment".into(),
            description: "Add an attachment to a message.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "message_id".into(),
                    param_type: "integer".into(),
                    required: true,
                    description: "Message ID".into(),
                },
                ParameterSchema {
                    name: "filename".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Filename".into(),
                },
                ParameterSchema {
                    name: "content_base64".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Base64 encoded content".into(),
                },
            ],
        },
        ToolSchema {
            name: "get_attachment".into(),
            description: "Get an attachment from a message.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "attachment_id".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Attachment ID".into(),
                },
                ParameterSchema {
                    name: "filename".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Filename".into(),
                },
            ],
        },
        ToolSchema {
            name: "list_tool_metrics".into(),
            description: "List recent tool usage metrics.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_id".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Optional project ID filter".into(),
                },
                ParameterSchema {
                    name: "limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Maximum results".into(),
                },
            ],
        },
        ToolSchema {
            name: "get_tool_stats".into(),
            description: "Get aggregated tool usage statistics.".into(),
            parameters: vec![ParameterSchema {
                name: "project_id".into(),
                param_type: "integer".into(),
                required: false,
                description: "Optional project ID filter".into(),
            }],
        },
        ToolSchema {
            name: "list_activity".into(),
            description: "List recent activity for a project.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_id".into(),
                    param_type: "integer".into(),
                    required: true,
                    description: "Project ID".into(),
                },
                ParameterSchema {
                    name: "limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Maximum results".into(),
                },
            ],
        },
        ToolSchema {
            name: "list_pending_reviews".into(),
            description:
                "List messages requiring acknowledgment that haven't been fully acknowledged."
                    .into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Filter by project slug".into(),
                },
                ParameterSchema {
                    name: "sender_name".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Filter by sender agent name (requires project_slug)".into(),
                },
                ParameterSchema {
                    name: "limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Maximum results (default 5, max 50)".into(),
                },
            ],
        },
        // Product-level cross-project tools
        ToolSchema {
            name: "search_messages_product".into(),
            description: "Search messages across all projects linked to a product.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "product_uid".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Product UID".into(),
                },
                ParameterSchema {
                    name: "query".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Search query".into(),
                },
                ParameterSchema {
                    name: "limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Maximum results per project".into(),
                },
            ],
        },
        ToolSchema {
            name: "summarize_thread_product".into(),
            description: "Summarize a thread across all projects in a product.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "product_uid".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Product UID".into(),
                },
                ParameterSchema {
                    name: "thread_id".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Thread ID to summarize".into(),
                },
            ],
        },
        ToolSchema {
            name: "whois".into(),
            description: "Get information about an agent including their program, model, and task description.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name to look up".into(),
                },
            ],
        },
        // NTM compatibility aliases - these map to existing tools
        ToolSchema {
            name: "fetch_inbox".into(),
            description: "Check an agent's inbox for new messages. (Alias for check_inbox/list_inbox)".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent name".into(),
                },
                ParameterSchema {
                    name: "limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Maximum messages to return".into(),
                },
            ],
        },
        ToolSchema {
            name: "release_file_reservations".into(),
            description: "Release a file reservation by ID. (Alias for release_reservation)".into(),
            parameters: vec![ParameterSchema {
                name: "reservation_id".into(),
                param_type: "integer".into(),
                required: true,
                description: "Reservation ID".into(),
            }],
        },
        ToolSchema {
            name: "renew_file_reservations".into(),
            description: "Extend the TTL of a file reservation. (Alias for renew_file_reservation)".into(),
            parameters: vec![
                ParameterSchema {
                    name: "reservation_id".into(),
                    param_type: "integer".into(),
                    required: true,
                    description: "Reservation ID".into(),
                },
                ParameterSchema {
                    name: "ttl_seconds".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "New TTL in seconds".into(),
                },
            ],
        },
        ToolSchema {
            name: "list_project_agents".into(),
            description: "List all agents in a project. (Alias for list_agents)".into(),
            parameters: vec![ParameterSchema {
                name: "project_slug".into(),
                param_type: "string".into(),
                required: true,
                description: "Project slug".into(),
            }],
        },
        ToolSchema {
            name: "create_agent_identity".into(),
            description: "Create a unique agent identity with auto-generated name.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "project_slug".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Project slug".into(),
                },
                ParameterSchema {
                    name: "hint".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Optional hint for name generation (e.g., 'blue' for BlueMountain)".into(),
                },
            ],
        },
        ToolSchema {
            name: "macro_start_session".into(),
            description: "Boot a project session: ensure project, register agent, optionally reserve files, fetch inbox.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "human_key".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Human-readable project key (creates if not exists)".into(),
                },
                ParameterSchema {
                    name: "program".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Agent program identifier".into(),
                },
                ParameterSchema {
                    name: "model".into(),
                    param_type: "string".into(),
                    required: true,
                    description: "Model being used".into(),
                },
                ParameterSchema {
                    name: "task_description".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Description of agent's task".into(),
                },
                ParameterSchema {
                    name: "agent_name".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Agent name (auto-generated if not provided)".into(),
                },
                ParameterSchema {
                    name: "file_reservation_paths".into(),
                    param_type: "array".into(),
                    required: false,
                    description: "Optional file paths to reserve".into(),
                },
                ParameterSchema {
                    name: "inbox_limit".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Number of inbox messages to fetch".into(),
                },
            ],
        },
    ]
}

/// The main MCP service for Agent Mail
// Simple macro for early return locally
macro_rules! guard_unwrap {
    ($val:expr, $ret:expr) => {
        if let Some(v) = $val { v } else { $ret }
    };
}

#[derive(Clone)]
pub struct AgentMailService {
    mm: Arc<ModelManager>,
    tool_router: ToolRouter<Self>,
    /// Whether worktrees/build slot tools are enabled
    worktrees_enabled: bool,
}

impl AgentMailService {
    /// Create a new AgentMailService with default configuration.
    ///
    /// Worktrees are disabled by default. Use `new_with_config()` to enable.
    pub async fn new() -> Result<Self> {
        // When using ::new(), we rely on env vars or defaults for AppConfig
        let config = AppConfig::load().unwrap_or_default();
        Self::new_with_config(config).await
    }

    /// Create a new AgentMailService with explicit worktrees configuration.
    ///
    /// When `worktrees_enabled` is true, build slot tools (acquire, release, renew)
    /// are registered and available. When false, they are hidden from the tool list
    /// and calls to them will return an error.
    pub async fn new_with_config(config: AppConfig) -> Result<Self> {
        let worktrees_enabled = config.mcp.worktrees_active();
        let app_config = Arc::new(config);
        let mm = Arc::new(ModelManager::new(app_config).await?);
        let tool_router = Self::tool_router();

        if worktrees_enabled {
            tracing::info!("MCP service starting with worktrees/build-slots ENABLED");
        } else {
            tracing::info!("MCP service starting with worktrees/build-slots DISABLED");
        }

        Ok(Self {
            mm,
            tool_router,
            worktrees_enabled,
        })
    }

    /// Create a new service with an existing ModelManager (for testing)
    pub fn new_with_mm(mm: Arc<ModelManager>, worktrees_enabled: bool) -> Self {
        let tool_router = Self::tool_router();

        if worktrees_enabled {
            tracing::info!("MCP service starting with worktrees/build-slots ENABLED");
        } else {
            tracing::info!("MCP service starting with worktrees/build-slots DISABLED");
        }

        Self {
            mm,
            tool_router,
            worktrees_enabled,
        }
    }

    /// Returns whether worktrees/build-slot tools are enabled
    pub fn worktrees_enabled(&self) -> bool {
        self.worktrees_enabled
    }

    fn ctx(&self) -> Ctx {
        Ctx::root_ctx()
    }

    pub async fn read_resource_impl(
        &self,
        request: ReadResourceRequestParam,
    ) -> Result<ReadResourceResult, McpError> {
        let uri_str = request.uri;
        let uri = url::Url::parse(&uri_str)
            .map_err(|e| McpError::invalid_params(format!("Invalid URI: {}", e), None))?;

        if uri.scheme() != "agent-mail" && uri.scheme() != "resource" {
            return Err(McpError::invalid_params(
                "URI scheme must be 'agent-mail' or 'resource'".to_string(),
                None,
            ));
        }

        // Parse query parameters
        let query: std::collections::HashMap<_, _> = uri.query_pairs().into_owned().collect();
        let project_slug_param = query.get("project");
        let limit = query
            .get("limit")
            .and_then(|l| l.parse::<i64>().ok())
            .unwrap_or(20);
        let include_bodies = query
            .get("include_bodies")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        // Handle path and host based on scheme
        let (project_slug, resource_type, resource_id) = if uri.scheme() == "agent-mail" {
            // agent-mail://{project_slug}/{resource_type}/{optional_id}
            let host = uri.host_str().ok_or(McpError::invalid_params(
                "URI missing host (project slug)".to_string(),
                None,
            ))?;
            let segments: Vec<&str> = uri
                .path_segments()
                .ok_or(McpError::invalid_params(
                    "Invalid URI path".to_string(),
                    None,
                ))?
                .collect();
            if segments.is_empty() {
                return Err(McpError::invalid_params(
                    "URI path missing resource type".to_string(),
                    None,
                ));
            }
            (host, segments[0], segments.get(1).cloned())
        } else {
            // resource://{resource_type}/{optional_id}?project={slug}
            let resource_type = uri.host_str().ok_or(McpError::invalid_params(
                "URI missing resource type".to_string(),
                None,
            ))?;
            let segments: Vec<&str> = uri
                .path_segments()
                .ok_or(McpError::invalid_params(
                    "Invalid URI path".to_string(),
                    None,
                ))?
                .collect();
            let resource_id = segments.first().cloned();

            // Project slug is optional for parsing, but might be required later
            let slug = project_slug_param.map(|s| s.as_str()).unwrap_or("");
            (slug, resource_type, resource_id)
        };

        let ctx = self.ctx();
        let mm = &self.mm;

        // Identity and Product don't need project resolution
        if resource_type == "identity" {
            let path = uri.path();
            if path.is_empty() {
                return Err(McpError::invalid_params(
                    "Missing identity path".to_string(),
                    None,
                ));
            }
            let data = serde_json::json!({
                "path": path,
                "type": "repository",
                "identity": format!("repo-{}", path.replace("/", "-").trim_start_matches('-')),
            });
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri_str,
                    mime_type: Some("application/json".to_string()),
                    text: serde_json::to_string_pretty(&data)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?,
                    meta: None,
                }],
            });
        }

        if resource_type == "product" {
            let product_uid = resource_id.ok_or(McpError::invalid_params(
                "Missing product UID".to_string(),
                None,
            ))?;
            let product = ProductBmc::get_by_uid(&ctx, mm, product_uid)
                .await
                .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri_str,
                    mime_type: Some("application/json".to_string()),
                    text: serde_json::to_string_pretty(&product)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?,
                    meta: None,
                }],
            });
        }

        // Resolve project ID for other resources
        let project = ProjectBmc::get_by_slug(&ctx, mm, project_slug)
            .await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;
        let project_id = project.id;

        // DTO for resource messages to support include_bodies
        #[derive(serde::Serialize)]
        struct ResourceMessage<'a> {
            id: i64,
            sender_name: &'a str,
            subject: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            body_md: Option<&'a str>,
            thread_id: Option<&'a String>,
            importance: &'a str,
            created_ts: chrono::NaiveDateTime,
        }

        let content = match resource_type {
            "agents" | "agent" => {
                if let Some(agent_name) = resource_id {
                    let agent = AgentBmc::get_by_name(&ctx, mm, project_id, agent_name)
                        .await
                        .map_err(|e| {
                            McpError::invalid_params(format!("Agent not found: {}", e), None)
                        })?;
                    serde_json::to_string_pretty(&agent)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?
                } else {
                    let agents = AgentBmc::list_all_for_project(&ctx, mm, project_id)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    serde_json::to_string_pretty(&agents)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?
                }
            }
            "file_reservations" => {
                let reservations =
                    FileReservationBmc::list_active_for_project(&ctx, mm, project_id)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&reservations)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "inbox" => {
                let agent_name = resource_id.ok_or(McpError::invalid_params(
                    "Missing agent name".to_string(),
                    None,
                ))?;
                let agent = AgentBmc::get_by_name(&ctx, mm, project_id, agent_name)
                    .await
                    .map_err(|e| {
                        McpError::invalid_params(format!("Agent not found: {}", e), None)
                    })?;

                let messages =
                    MessageBmc::list_inbox_for_agent(&ctx, mm, project_id, agent.id, limit)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                let resource_messages: Vec<_> = messages
                    .iter()
                    .map(|m| ResourceMessage {
                        id: m.id,
                        sender_name: &m.sender_name,
                        subject: &m.subject,
                        body_md: if include_bodies {
                            Some(&m.body_md)
                        } else {
                            None
                        },
                        thread_id: m.thread_id.as_ref(),
                        importance: &m.importance,
                        created_ts: m.created_ts,
                    })
                    .collect();

                serde_json::to_string_pretty(&resource_messages)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "outbox" => {
                let agent_name = resource_id.ok_or(McpError::invalid_params(
                    "Missing agent name".to_string(),
                    None,
                ))?;
                let agent = AgentBmc::get_by_name(&ctx, mm, project_id, agent_name)
                    .await
                    .map_err(|e| {
                        McpError::invalid_params(format!("Agent not found: {}", e), None)
                    })?;

                let messages =
                    MessageBmc::list_outbox_for_agent(&ctx, mm, project_id, agent.id, limit)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                let resource_messages: Vec<_> = messages
                    .iter()
                    .map(|m| ResourceMessage {
                        id: m.id,
                        sender_name: &m.sender_name,
                        subject: &m.subject,
                        body_md: if include_bodies {
                            Some(&m.body_md)
                        } else {
                            None
                        },
                        thread_id: m.thread_id.as_ref(),
                        importance: &m.importance,
                        created_ts: m.created_ts,
                    })
                    .collect();

                serde_json::to_string_pretty(&resource_messages)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "thread" => {
                let thread_id_str = resource_id.ok_or(McpError::invalid_params(
                    "Missing thread ID".to_string(),
                    None,
                ))?;
                let messages = MessageBmc::list_by_thread(&ctx, mm, project_id, thread_id_str)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                let resource_messages: Vec<_> = messages
                    .iter()
                    .map(|m| ResourceMessage {
                        id: m.id,
                        sender_name: &m.sender_name,
                        subject: &m.subject,
                        body_md: if include_bodies {
                            Some(&m.body_md)
                        } else {
                            None
                        },
                        thread_id: m.thread_id.as_ref(),
                        importance: &m.importance,
                        created_ts: m.created_ts,
                    })
                    .collect();

                serde_json::to_string_pretty(&resource_messages)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "threads" => {
                let threads = MessageBmc::list_threads(&ctx, mm, project_id, limit)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&threads)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "product" => {
                let product_uid = resource_id.ok_or(McpError::invalid_params(
                    "Missing product UID".to_string(),
                    None,
                ))?;
                let product = ProductBmc::get_by_uid(&ctx, mm, product_uid)
                    .await
                    .map_err(|e| {
                        McpError::invalid_params(format!("Product not found: {}", e), None)
                    })?;
                serde_json::to_string_pretty(&product)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            _ => {
                return Err(McpError::invalid_params(
                    format!("Unknown resource type: {}", resource_type),
                    None,
                ));
            }
        };

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::TextResourceContents {
                uri: uri_str,
                mime_type: Some("application/json".to_string()),
                text: content,
                meta: None,
            }],
        })
    }
    pub async fn record_tool_metric(
        &self,
        tool_name: &str,
        args: &Option<serde_json::Value>,
        duration: std::time::Duration,
        result: &Result<CallToolResult, McpError>,
    ) {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::project::ProjectBmc;
        use lib_core::model::tool_metric::{ToolMetricBmc, ToolMetricForCreate};

        let (status, error_code) = match result {
            Ok(_) => ("success".to_string(), None),
            Err(e) => ("error".to_string(), Some(format!("{:?}", e.code))),
        };

        // Extract context
        let (project_slug, agent_name) = self.extract_context(args);

        // Resolve IDs (best effort)
        let ctx = self.ctx();
        let mut project_id = None;
        let mut agent_id = None;

        if let Some(slug) = project_slug
            && let Ok(p) = ProjectBmc::get_by_slug(&ctx, &self.mm, &slug).await
        {
            project_id = Some(p.id);
            if let Some(name) = agent_name
                && let Ok(a) = AgentBmc::get_by_name(&ctx, &self.mm, p.id, &name).await
            {
                agent_id = Some(a.id);
            }
        }

        let metric = ToolMetricForCreate {
            project_id,
            agent_id,
            tool_name: tool_name.to_string(),
            args_json: args.as_ref().map(|v| v.to_string()),
            status,
            error_code,
            duration_ms: duration.as_millis() as i64,
        };

        if let Err(e) = ToolMetricBmc::create(&ctx, &self.mm, metric).await {
            tracing::error!("Failed to record tool metric: {}", e);
        }
    }

    pub fn extract_context(
        &self,
        args: &Option<serde_json::Value>,
    ) -> (Option<String>, Option<String>) {
        let val = guard_unwrap!(args.as_ref(), return (None, None));
        let obj = guard_unwrap!(val.as_object(), return (None, None));

        // Try to find project slug
        let project_slug = obj
            .get("project_slug")
            .or_else(|| obj.get("slug")) // For EnsureProjectParams
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Try to find agent name
        let agent_name = obj
            .get("agent_name")
            .or_else(|| obj.get("sender_name")) // For SendMessageParams
            .or_else(|| obj.get("name")) // For RegisterAgentParams
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        (project_slug, agent_name)
    }
    pub async fn list_resources_impl(
        &self,
        _request: Option<PaginatedRequestParam>,
    ) -> Result<ListResourcesResult, McpError> {
        // List project-rooted resources for all projects
        let ctx = self.ctx();
        let projects = ProjectBmc::list_all(&ctx, &self.mm)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut resources = Vec::new();

        for project in projects {
            let slug = &project.slug;

            // Agents list
            resources.push(Resource {
                raw: RawResource {
                    uri: format!("agent-mail://{}/agents", slug),
                    name: format!("Agents ({})", slug),
                    description: Some(format!("List of all agents in project '{}'", slug)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                    title: None,
                },
                annotations: None,
            });

            // Agents Inboxes & Outboxes
            let project_agents = AgentBmc::list_all_for_project(&ctx, &self.mm, project.id)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            for agent in project_agents {
                resources.push(Resource {
                    raw: RawResource {
                        uri: format!("agent-mail://{}/inbox/{}", slug, agent.name),
                        name: format!("Inbox: {} ({})", agent.name, slug),
                        description: Some(format!("Inbox for agent '{}'", agent.name)),
                        mime_type: Some("application/json".to_string()),
                        size: None,
                        icons: None,
                        meta: None,
                        title: None,
                    },
                    annotations: None,
                });
                resources.push(Resource {
                    raw: RawResource {
                        uri: format!("agent-mail://{}/outbox/{}", slug, agent.name),
                        name: format!("Outbox: {} ({})", agent.name, slug),
                        description: Some(format!("Outbox for agent '{}'", agent.name)),
                        mime_type: Some("application/json".to_string()),
                        size: None,
                        icons: None,
                        meta: None,
                        title: None,
                    },
                    annotations: None,
                });

                // resource:// versions
                resources.push(Resource {
                    raw: RawResource {
                        uri: format!("resource://inbox/{}?project={}", agent.name, slug),
                        name: format!("Inbox (resource://): {} ({})", agent.name, slug),
                        description: Some(format!("Inbox for agent '{}'", agent.name)),
                        mime_type: Some("application/json".to_string()),
                        size: None,
                        icons: None,
                        meta: None,
                        title: None,
                    },
                    annotations: None,
                });
                resources.push(Resource {
                    raw: RawResource {
                        uri: format!("resource://outbox/{}?project={}", agent.name, slug),
                        name: format!("Outbox (resource://): {} ({})", agent.name, slug),
                        description: Some(format!("Outbox for agent '{}'", agent.name)),
                        mime_type: Some("application/json".to_string()),
                        size: None,
                        icons: None,
                        meta: None,
                        title: None,
                    },
                    annotations: None,
                });
            }

            // Project Threads
            resources.push(Resource {
                raw: RawResource {
                    uri: format!("agent-mail://{}/threads", slug),
                    name: format!("Threads ({})", slug),
                    description: Some(format!(
                        "List of all conversation threads in project '{}'",
                        slug
                    )),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                    title: None,
                },
                annotations: None,
            });

            // File reservations
            resources.push(Resource {
                raw: RawResource {
                    uri: format!("agent-mail://{}/file_reservations", slug),
                    name: format!("File Reservations ({})", slug),
                    description: Some(format!("Active file reservations in project '{}'", slug)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                    title: None,
                },
                annotations: None,
            });

            // --- resource:// Scheme Versions ---
            resources.push(Resource {
                raw: RawResource {
                    uri: format!("resource://agents?project={}", slug),
                    name: format!("Agents (resource:// {})", slug),
                    description: Some(format!("List of all agents in project '{}'", slug)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                    title: None,
                },
                annotations: None,
            });

            resources.push(Resource {
                raw: RawResource {
                    uri: format!("resource://threads?project={}", slug),
                    name: format!("Threads (resource:// {})", slug),
                    description: Some(format!(
                        "List of all conversation threads in project '{}'",
                        slug
                    )),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                    title: None,
                },
                annotations: None,
            });
        }

        // --- Products ---
        let products = ProductBmc::list_all(&ctx, &self.mm)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        for product in products {
            resources.push(Resource {
                raw: RawResource {
                    uri: format!("resource://product/{}", product.product_uid),
                    name: format!("Product: {}", product.name),
                    description: Some(format!("Information about product '{}'", product.name)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                    title: None,
                },
                annotations: None,
            });
        }

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    /// Public impl method for testing search_messages_product
    pub async fn search_messages_product_impl(
        &self,
        params: Parameters<SearchMessagesProductParams>,
    ) -> Result<CallToolResult, McpError> {
        self.search_messages_product(params).await
    }

    /// Public impl method for testing summarize_thread_product
    pub async fn summarize_thread_product_impl(
        &self,
        params: Parameters<SummarizeThreadProductParams>,
    ) -> Result<CallToolResult, McpError> {
        self.summarize_thread_product(params).await
    }
}

#[allow(clippy::manual_async_fn)]
impl ServerHandler for AgentMailService {
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            let all_tools = self.tool_router.list_all();

            // Filter out build slot tools when worktrees is disabled
            let tools = if self.worktrees_enabled {
                all_tools
            } else {
                all_tools
                    .into_iter()
                    .filter(|tool| !BUILD_SLOT_TOOLS.contains(&&*tool.name))
                    .collect()
            };

            Ok(ListToolsResult {
                tools,
                next_cursor: None,
                meta: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            let start = std::time::Instant::now();
            let original_name = request.name.clone();
            let args = request.arguments.clone();

            let resolved_name: Option<&str> = match &*original_name {
                "fetch_inbox" | "check_inbox" => Some("list_inbox"),
                "release_file_reservations" => Some("release_reservation"),
                "renew_file_reservations" => Some("renew_file_reservation"),
                "list_project_agents" => Some("list_agents"),
                _ => None,
            };

            let request = if let Some(new_name) = resolved_name {
                tracing::debug!(
                    original = %original_name,
                    resolved = %new_name,
                    "Resolved tool alias"
                );
                CallToolRequestParam {
                    name: new_name.into(),
                    arguments: args.clone(),
                }
            } else {
                request
            };

            let tool_name = request.name.clone();

            if !self.worktrees_enabled && BUILD_SLOT_TOOLS.contains(&&*tool_name) {
                tracing::warn!(
                    tool = %tool_name,
                    "Attempted to call build slot tool but worktrees are disabled"
                );
                return Err(McpError::invalid_request(
                    format!(
                        "Tool '{}' is not available. Build slot tools require WORKTREES_ENABLED=true.",
                        tool_name
                    ),
                    None,
                ));
            }

            let tool_context =
                rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
            let result = self.tool_router.call(tool_context).await;

            let duration = start.elapsed();

            // Fire and forget metric recording (spawn generic task or just await since we are async)
            // Awaiting is safer to ensure it's recorded before response?
            // Better to spawn to avoid latency, but for now await is fine as DB write is fast.
            let args_val = args.map(serde_json::Value::Object);
            self.record_tool_metric(&tool_name, &args_val, duration, &result)
                .await;

            result
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async move { self.list_resources_impl(_request).await }
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        self.read_resource_impl(request)
    }
}

// ============================================================================
// Tool Implementations
// ============================================================================

#[tool_router]
impl AgentMailService {
    /// Ensure a project exists, creating it if necessary
    #[tool(
        description = "Create or get a project. Projects are workspaces that contain agents and their messages."
    )]
    async fn ensure_project(
        &self,
        params: Parameters<EnsureProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::project::ProjectBmc;
        use lib_core::utils::validation::validate_project_key;

        let ctx = self.ctx();
        let p = params.0;

        // Validate inputs
        validate_project_key(&p.slug).map_err(|e| {
            McpError::invalid_params(
                format!("{}", e),
                Some(serde_json::json!({ "details": e.context() })),
            )
        })?;

        // Check if project exists
        match ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.slug).await {
            Ok(project) => {
                let msg = format!(
                    "Project '{}' already exists (id: {}, human_key: {})",
                    project.slug, project.id, project.human_key
                );
                Ok(CallToolResult::success(vec![Content::text(msg)]))
            }
            Err(_) => {
                // Create new project
                let id = ProjectBmc::create(&ctx, &self.mm, &p.slug, &p.human_key)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                let msg = format!(
                    "Created project '{}' with id {} and human_key '{}'",
                    p.slug, id, p.human_key
                );
                Ok(CallToolResult::success(vec![Content::text(msg)]))
            }
        }
    }

    /// Register a new agent in a project
    #[tool(description = "Register an agent in a project. Agents can send and receive messages.")]
    async fn register_agent(
        &self,
        params: Parameters<RegisterAgentParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::{AgentBmc, AgentForCreate};

        use lib_core::utils::validation::{validate_agent_name, validate_project_key};

        let ctx = self.ctx();
        let p = params.0;

        // Validate inputs
        validate_project_key(&p.project_slug).map_err(|e| {
            McpError::invalid_params(
                format!("{}", e),
                Some(serde_json::json!({ "details": e.context() })),
            )
        })?;

        validate_agent_name(&p.name).map_err(|e| {
            McpError::invalid_params(
                format!("{}", e),
                Some(serde_json::json!({ "details": e.context() })),
            )
        })?;

        // Get project
        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        // Check if agent exists
        match AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.name).await {
            Ok(agent) => {
                let msg = format!(
                    "Agent '{}' already exists (id: {}, program: {})",
                    agent.name, agent.id, agent.program
                );
                Ok(CallToolResult::success(vec![Content::text(msg)]))
            }
            Err(_) => {
                let agent_c = AgentForCreate {
                    project_id: project.id,
                    name: p.name.clone(),
                    program: p.program,
                    model: p.model,
                    task_description: p.task_description,
                };

                let id = AgentBmc::create(&ctx, &self.mm, agent_c)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                // Auto-grant default capabilities for MCP tool usage
                AgentCapabilityBmc::grant_defaults(&ctx, &self.mm, id)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                let msg = format!(
                    "Registered agent '{}' with id {} (granted default capabilities)",
                    p.name, id
                );
                Ok(CallToolResult::success(vec![Content::text(msg)]))
            }
        }
    }

    /// Send a message to one or more agents
    #[tool(
        description = "Send a message from one agent to another. Creates a new thread or continues an existing one."
    )]
    async fn send_message(
        &self,
        params: Parameters<SendMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::message::{MessageBmc, MessageForCreate};

        let ctx = self.ctx();
        let p = params.0;

        // Get project and sender
        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let sender = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.sender_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Sender not found: {}", e), None))?;

        if !AgentCapabilityBmc::check(&ctx, &self.mm, sender.id, "send_message")
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
        {
            return Err(McpError::invalid_params(
                format!(
                    "Agent '{}' does not have 'send_message' capability",
                    p.sender_name
                ),
                None,
            ));
        }

        // Helper to resolve list of names to IDs
        async fn resolve_agents(
            ctx: &lib_core::Ctx,
            mm: &lib_core::ModelManager,
            project_id: i64,
            names_str: &str,
        ) -> Result<Vec<i64>, McpError> {
            use lib_core::model::agent::AgentBmc;
            let names: Vec<&str> = names_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            let mut ids = Vec::new();
            for name in names {
                let agent = AgentBmc::get_by_name(ctx, mm, project_id, name)
                    .await
                    .map_err(|e| {
                        McpError::invalid_params(format!("Agent '{}' not found: {}", name, e), None)
                    })?;
                ids.push(agent.id);
            }
            Ok(ids)
        }

        // Parse recipients
        let recipient_ids = resolve_agents(&ctx, &self.mm, project.id, &p.to).await?;

        let cc_ids = if let Some(cc) = &p.cc {
            let ids = resolve_agents(&ctx, &self.mm, project.id, cc).await?;
            if ids.is_empty() { None } else { Some(ids) }
        } else {
            None
        };

        let bcc_ids = if let Some(bcc) = &p.bcc {
            let ids = resolve_agents(&ctx, &self.mm, project.id, bcc).await?;
            if ids.is_empty() { None } else { Some(ids) }
        } else {
            None
        };

        // Create message
        let msg_c = MessageForCreate {
            project_id: project.id,
            sender_id: sender.id,
            recipient_ids,
            cc_ids,
            bcc_ids,
            subject: p.subject.clone(),
            body_md: p.body_md,
            thread_id: p.thread_id,
            importance: p.importance,
            ack_required: p.ack_required.unwrap_or(false),
        };

        let msg_id = MessageBmc::create(&ctx, &self.mm, msg_c)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!(
            "Message sent (id: {}) from '{}' to '{}' with subject '{}'",
            msg_id, p.sender_name, p.to, p.subject
        );
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// List messages in an agent's inbox
    #[tool(description = "Get messages for an agent's inbox.")]
    async fn list_inbox(
        &self,
        params: Parameters<ListInboxParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::message::MessageBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        if !AgentCapabilityBmc::check(&ctx, &self.mm, agent.id, "fetch_inbox")
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
        {
            return Err(McpError::invalid_params(
                format!(
                    "Agent '{}' does not have 'fetch_inbox' capability",
                    p.agent_name
                ),
                None,
            ));
        }

        let messages = MessageBmc::list_inbox_for_agent(
            &ctx,
            &self.mm,
            project.id,
            agent.id,
            p.limit.unwrap_or(50),
        )
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!(
            "Inbox for '{}' ({} messages):\n\n",
            p.agent_name,
            messages.len()
        );
        for m in &messages {
            output.push_str(&format!(
                "- [{}] {} (from: {}, thread: {:?}, {})\n",
                m.id, m.subject, m.sender_name, m.thread_id, m.importance
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Get a specific message by ID
    #[tool(description = "Retrieve a message by its ID, including full body content.")]
    async fn get_message(
        &self,
        params: Parameters<GetMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::message::MessageBmc;

        let ctx = self.ctx();
        let p = params.0;

        let message = MessageBmc::get(&ctx, &self.mm, p.message_id)
            .await
            .map_err(|e| McpError::invalid_params(format!("Message not found: {}", e), None))?;

        let output = format!(
            "Message ID: {}\nFrom: {}\nSubject: {}\nThread: {:?}\nImportance: {}\nCreated: {}\n\n---\n{}",
            message.id,
            message.sender_name,
            message.subject,
            message.thread_id,
            message.importance,
            message.created_ts,
            message.body_md
        );

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Look up information about an agent
    #[tool(
        description = "Get information about an agent including their program, model, and task description."
    )]
    async fn whois(&self, params: Parameters<WhoisParams>) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let output = format!(
            "Agent: {}\nID: {}\nProgram: {}\nModel: {}\nTask: {}\nContact Policy: {}\nAttachments Policy: {}",
            agent.name,
            agent.id,
            agent.program,
            agent.model,
            agent.task_description,
            agent.contact_policy,
            agent.attachments_policy
        );

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Search messages using full-text search
    #[tool(description = "Search messages by content using full-text search.")]
    async fn search_messages(
        &self,
        params: Parameters<SearchMessagesParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::message::MessageBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let messages =
            MessageBmc::search(&ctx, &self.mm, project.id, &p.query, p.limit.unwrap_or(20))
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!(
            "Search results for '{}' ({} matches):\n\n",
            p.query,
            messages.len()
        );
        for m in &messages {
            output.push_str(&format!(
                "- [{}] {} (from: {}, thread: {:?})\n",
                m.id, m.subject, m.sender_name, m.thread_id
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Get all messages in a thread
    #[tool(description = "Retrieve all messages in a conversation thread.")]
    async fn get_thread(
        &self,
        params: Parameters<GetThreadParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::message::MessageBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let messages = MessageBmc::list_by_thread(&ctx, &self.mm, project.id, &p.thread_id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!(
            "Thread '{}' ({} messages):\n\n",
            p.thread_id,
            messages.len()
        );
        for m in &messages {
            output.push_str(&format!(
                "---\n[{}] From: {} | {}\nSubject: {}\n\n{}\n\n",
                m.id, m.sender_name, m.created_ts, m.subject, m.body_md
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Get review state of a task thread
    #[tool(
        description = "Get the current review state of a task thread based on message prefixes."
    )]
    async fn get_review_state(
        &self,
        params: Parameters<GetReviewStateParams>,
    ) -> Result<CallToolResult, McpError> {
        reviews::get_review_state_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Claim a pending review atomically
    #[tool(
        description = "Claim a pending review by sending a [REVIEWING] message. Prevents duplicate reviews."
    )]
    async fn claim_review(
        &self,
        params: Parameters<ClaimReviewParams>,
    ) -> Result<CallToolResult, McpError> {
        reviews::claim_review_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// List all projects
    #[tool(description = "List all available projects in the system.")]
    async fn list_projects(&self) -> Result<CallToolResult, McpError> {
        use lib_core::model::project::ProjectBmc;

        let ctx = self.ctx();

        let projects = ProjectBmc::list_all(&ctx, &self.mm)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Projects ({}):\n\n", projects.len());
        for p in &projects {
            output.push_str(&format!(
                "- {} (slug: {}, created: {})\n",
                p.human_key, p.slug, p.created_at
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// List all agents in a project
    #[tool(description = "List all agents registered in a project.")]
    async fn list_agents(
        &self,
        params: Parameters<ListAgentsParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agents = AgentBmc::list_all_for_project(&ctx, &self.mm, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Agents in '{}' ({}):\n\n", p.project_slug, agents.len());
        for a in &agents {
            output.push_str(&format!(
                "- {} (program: {}, model: {})\n  Task: {}\n",
                a.name, a.program, a.model, a.task_description
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Reserve a file path for an agent
    #[tool(description = "Reserve a file path pattern to prevent conflicts between agents.")]
    async fn reserve_file(
        &self,
        params: Parameters<FileReservationParams>,
    ) -> Result<CallToolResult, McpError> {
        files::reserve_file_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// List active file reservations
    #[tool(description = "List all active file reservations in a project.")]
    async fn list_reservations(
        &self,
        params: Parameters<ListReservationsParams>,
    ) -> Result<CallToolResult, McpError> {
        files::list_reservations_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Release a file reservation
    #[tool(description = "Release a file reservation by ID.")]
    async fn release_reservation(
        &self,
        params: Parameters<ReleaseReservationParams>,
    ) -> Result<CallToolResult, McpError> {
        files::release_reservation_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Force release a file reservation (emergency override)
    #[tool(
        description = "Force release a file reservation by ID. Use for emergencies when an agent has abandoned work."
    )]
    async fn force_release_reservation(
        &self,
        params: Parameters<ForceReleaseReservationParams>,
    ) -> Result<CallToolResult, McpError> {
        files::force_release_reservation_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Renew a file reservation TTL
    #[tool(
        description = "Extend the TTL of a file reservation. Keeps the lock active for more work."
    )]
    async fn renew_file_reservation(
        &self,
        params: Parameters<RenewFileReservationParams>,
    ) -> Result<CallToolResult, McpError> {
        files::renew_file_reservation_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Reply to a message
    #[tool(description = "Reply to an existing message in a thread.")]
    async fn reply_message(
        &self,
        params: Parameters<ReplyMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::message::{MessageBmc, MessageForCreate};

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let sender = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.sender_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Sender not found: {}", e), None))?;

        if !AgentCapabilityBmc::check(&ctx, &self.mm, sender.id, "send_message")
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
        {
            return Err(McpError::invalid_params(
                format!(
                    "Agent '{}' does not have 'send_message' capability",
                    p.sender_name
                ),
                None,
            ));
        }

        let original_msg = MessageBmc::get(&ctx, &self.mm, p.message_id)
            .await
            .map_err(|e| McpError::invalid_params(format!("Message not found: {}", e), None))?;

        let subject = if original_msg.subject.starts_with("Re: ") {
            original_msg.subject.clone()
        } else {
            format!("Re: {}", original_msg.subject)
        };

        let msg_c = MessageForCreate {
            project_id: project.id,
            sender_id: sender.id,
            recipient_ids: vec![original_msg.sender_id],
            cc_ids: None,
            bcc_ids: None,
            subject: subject.clone(),
            body_md: p.body_md,
            thread_id: original_msg.thread_id.clone(),
            importance: p.importance,
            ack_required: false, // Replies don't require ack by default
        };

        let msg_id = MessageBmc::create(&ctx, &self.mm, msg_c)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Reply sent (id: {}) with subject '{}'", msg_id, subject);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Mark a message as read
    #[tool(description = "Mark a message as read by a specific agent.")]
    async fn mark_message_read(
        &self,
        params: Parameters<MarkMessageReadParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::message::MessageBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        MessageBmc::mark_read(&ctx, &self.mm, p.message_id, agent.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!(
            "Message {} marked as read by '{}'",
            p.message_id, p.agent_name
        );
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Acknowledge a message
    #[tool(description = "Acknowledge receipt of a message requiring acknowledgment.")]
    async fn acknowledge_message(
        &self,
        params: Parameters<AcknowledgeMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::message::MessageBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        if !AgentCapabilityBmc::check(&ctx, &self.mm, agent.id, "acknowledge_message")
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
        {
            return Err(McpError::invalid_params(
                format!(
                    "Agent '{}' does not have 'acknowledge_message' capability",
                    p.agent_name
                ),
                None,
            ));
        }

        MessageBmc::acknowledge(&ctx, &self.mm, p.message_id, agent.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!(
            "Message {} acknowledged by '{}'",
            p.message_id, p.agent_name
        );
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Generate agent identity names
    #[tool(description = "Generate memorable agent names with collision detection.")]
    async fn create_agent_identity(
        &self,
        params: Parameters<CreateAgentIdentityParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;

        use std::collections::HashSet;

        const ADJECTIVES: &[&str] = &[
            "Blue", "Green", "Red", "Golden", "Silver", "Crystal", "Dark", "Bright", "Swift",
            "Calm", "Bold", "Wise", "Noble", "Grand", "Mystic", "Ancient",
        ];
        const NOUNS: &[&str] = &[
            "Mountain", "Castle", "River", "Forest", "Valley", "Harbor", "Tower", "Bridge",
            "Falcon", "Phoenix", "Dragon", "Wolf", "Eagle", "Lion", "Hawk", "Owl",
        ];

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let existing_agents = AgentBmc::list_all_for_project(&ctx, &self.mm, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let existing_names: HashSet<String> =
            existing_agents.iter().map(|a| a.name.clone()).collect();

        let mut alternatives = Vec::new();
        let mut rng_seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as usize;

        let mut next_rand = || {
            rng_seed = rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
            rng_seed
        };

        for _ in 0..10 {
            let adj_idx = next_rand() % ADJECTIVES.len();
            let noun_idx = next_rand() % NOUNS.len();
            let name = format!("{}{}", ADJECTIVES[adj_idx], NOUNS[noun_idx]);

            if !existing_names.contains(&name) && !alternatives.contains(&name) {
                alternatives.push(name);
                if alternatives.len() >= 5 {
                    break;
                }
            }
        }

        let suggested = alternatives
            .first()
            .cloned()
            .unwrap_or_else(|| "Agent1".to_string());
        let output = format!(
            "Suggested: {}\nAlternatives: {}",
            suggested,
            alternatives.join(", ")
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Update agent profile
    #[tool(description = "Update an agent's profile settings.")]
    async fn update_agent_profile(
        &self,
        params: Parameters<UpdateAgentProfileParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::{AgentBmc, AgentProfileUpdate};

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let update = AgentProfileUpdate {
            task_description: p.task_description,
            attachments_policy: p.attachments_policy,
            contact_policy: p.contact_policy,
        };

        AgentBmc::update_profile(&ctx, &self.mm, agent.id, update)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Updated profile for agent '{}'", p.agent_name);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Get project info
    #[tool(description = "Get detailed information about a project.")]
    async fn get_project_info(
        &self,
        params: Parameters<GetProjectInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agents = AgentBmc::list_all_for_project(&ctx, &self.mm, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let message_count = ProjectBmc::count_messages(&ctx, &self.mm, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let output = format!(
            "Project: {} ({})\nID: {}\nAgents: {}\nMessages: {}\nCreated: {}",
            project.human_key,
            project.slug,
            project.id,
            agents.len(),
            message_count,
            project.created_at
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Get agent profile
    #[tool(description = "Get detailed profile information for an agent.")]
    async fn get_agent_profile(
        &self,
        params: Parameters<GetAgentProfileParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::file_reservation::FileReservationBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let sent_count = AgentBmc::count_messages_sent(&ctx, &self.mm, agent.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let received_count = AgentBmc::count_messages_received(&ctx, &self.mm, agent.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let reservations = FileReservationBmc::list_active_for_project(&ctx, &self.mm, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let active_reservations = reservations
            .iter()
            .filter(|r| r.agent_id == agent.id)
            .count();

        let output = format!(
            "Agent: {}\nID: {}\nProgram: {}\nModel: {}\nTask: {}\nContact Policy: {}\nAttachments Policy: {}\nMessages Sent: {}\nMessages Received: {}\nActive Reservations: {}\nInception: {}\nLast Active: {}",
            agent.name,
            agent.id,
            agent.program,
            agent.model,
            agent.task_description,
            agent.contact_policy,
            agent.attachments_policy,
            sent_count,
            received_count,
            active_reservations,
            agent.inception_ts,
            agent.last_active_ts
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// List threads
    #[tool(description = "List all conversation threads in a project.")]
    async fn list_threads(
        &self,
        params: Parameters<ListThreadsParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::message::MessageBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let threads = MessageBmc::list_threads(&ctx, &self.mm, project.id, p.limit.unwrap_or(50))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Threads in '{}' ({}):\n\n", p.project_slug, threads.len());
        for t in &threads {
            output.push_str(&format!(
                "- {} | {} ({} msgs, last: {})\n",
                t.thread_id, t.subject, t.message_count, t.last_message_ts
            ));
        }
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Request contact
    #[tool(description = "Request to add another agent as a contact.")]
    async fn request_contact(
        &self,
        params: Parameters<RequestContactParams>,
    ) -> Result<CallToolResult, McpError> {
        contacts::request_contact_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Respond to contact request
    #[tool(description = "Accept or reject a contact request.")]
    async fn respond_contact(
        &self,
        params: Parameters<RespondContactParams>,
    ) -> Result<CallToolResult, McpError> {
        contacts::respond_contact_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// List contacts
    #[tool(description = "List all contacts for an agent.")]
    async fn list_contacts(
        &self,
        params: Parameters<ListContactsParams>,
    ) -> Result<CallToolResult, McpError> {
        contacts::list_contacts_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Set contact policy
    #[tool(description = "Set an agent's contact acceptance policy (auto, manual, deny).")]
    async fn set_contact_policy(
        &self,
        params: Parameters<SetContactPolicyParams>,
    ) -> Result<CallToolResult, McpError> {
        contacts::set_contact_policy_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Acquire build slot
    #[tool(description = "Acquire an exclusive build slot for CI/CD isolation.")]
    async fn acquire_build_slot(
        &self,
        params: Parameters<AcquireBuildSlotParams>,
    ) -> Result<CallToolResult, McpError> {
        builds::acquire_build_slot_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Renew build slot
    #[tool(description = "Extend TTL on an active build slot.")]
    async fn renew_build_slot(
        &self,
        params: Parameters<RenewBuildSlotParams>,
    ) -> Result<CallToolResult, McpError> {
        builds::renew_build_slot_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Release build slot
    #[tool(description = "Release a held build slot.")]
    async fn release_build_slot(
        &self,
        params: Parameters<ReleaseBuildSlotParams>,
    ) -> Result<CallToolResult, McpError> {
        builds::release_build_slot_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Send overseer message
    #[tool(description = "Send a guidance message from the human overseer to an agent.")]
    async fn send_overseer_message(
        &self,
        params: Parameters<SendOverseerMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::overseer_message::{OverseerMessageBmc, OverseerMessageForCreate};

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let msg_c = OverseerMessageForCreate {
            project_id: project.id,
            sender_id: agent.id,
            subject: p.subject.clone(),
            body_md: p.body_md,
            importance: p.importance.unwrap_or_else(|| "normal".to_string()),
        };

        let message_id = OverseerMessageBmc::create(&ctx, &self.mm, msg_c)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!(
            "Overseer message sent (id: {}) to '{}'",
            message_id, p.agent_name
        );
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// List macros
    #[tool(description = "List all available macros in a project.")]
    async fn list_macros(
        &self,
        params: Parameters<ListMacrosParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::macro_def::MacroDefBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let macros = MacroDefBmc::list(&ctx, &self.mm, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Macros in '{}' ({}):\n\n", p.project_slug, macros.len());
        for m in &macros {
            output.push_str(&format!(
                "- {} ({} steps): {}\n",
                m.name,
                m.steps.len(),
                m.description
            ));
        }
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Register macro
    #[tool(description = "Register a new macro definition.")]
    async fn register_macro(
        &self,
        params: Parameters<RegisterMacroParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::macro_def::{MacroDefBmc, MacroDefForCreate};

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let macro_c = MacroDefForCreate {
            project_id: project.id,
            name: p.name.clone(),
            description: p.description,
            steps: p.steps,
        };

        let macro_id = MacroDefBmc::create(&ctx, &self.mm, macro_c)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Registered macro '{}' with id {}", p.name, macro_id);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Unregister macro
    #[tool(description = "Remove a macro definition.")]
    async fn unregister_macro(
        &self,
        params: Parameters<UnregisterMacroParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::macro_def::MacroDefBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let deleted = MacroDefBmc::delete(&ctx, &self.mm, project.id, &p.name)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = if deleted {
            format!("Unregistered macro '{}'", p.name)
        } else {
            format!("Macro '{}' not found", p.name)
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Invoke macro
    #[tool(description = "Execute a pre-defined macro and get its steps.")]
    async fn invoke_macro(
        &self,
        params: Parameters<InvokeMacroParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::macro_def::MacroDefBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let macro_def = MacroDefBmc::get_by_name(&ctx, &self.mm, project.id, &p.name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Macro not found: {}", e), None))?;

        let steps_json =
            serde_json::to_string_pretty(&macro_def.steps).unwrap_or_else(|_| "[]".to_string());
        let output = format!(
            "Macro '{}' ({} steps)\nDescription: {}\n\nSteps:\n{}",
            macro_def.name,
            macro_def.steps.len(),
            macro_def.description,
            steps_json
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// List built-in workflow macros
    #[tool(description = "List the 5 built-in workflow macros available in all projects.")]
    async fn list_builtin_workflows(
        &self,
        _params: Parameters<ListBuiltinWorkflowsParams>,
    ) -> Result<CallToolResult, McpError> {
        let workflows = vec![
            ("start_session", "Register agent and check inbox"),
            ("prepare_thread", "Create thread and reserve files"),
            ("file_reservation_cycle", "Reserve, work, release files"),
            ("contact_handshake", "Establish cross-project contact"),
            ("broadcast_message", "Send to multiple agents"),
        ];

        let mut output = String::from("Built-in Workflows:\n\n");
        for (name, desc) in workflows {
            output.push_str(&format!("- {}: {}\n", name, desc));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Quick standup workflow
    #[tool(description = "Broadcast standup request to all agents in a project.")]
    async fn quick_standup_workflow(
        &self,
        params: Parameters<QuickStandupWorkflowParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::message::{MessageBmc, MessageForCreate};

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let sender = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.sender_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Sender not found: {}", e), None))?;

        let agents = AgentBmc::list_all_for_project(&ctx, &self.mm, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let recipient_ids: Vec<i64> = agents.iter().map(|a| a.id).collect();

        let question = p
            .standup_question
            .unwrap_or_else(|| "What are you working on today?".to_string());

        let msg_c = MessageForCreate {
            project_id: project.id,
            sender_id: sender.id,
            recipient_ids,
            cc_ids: None,
            bcc_ids: None,
            subject: "Daily Standup".to_string(),
            body_md: question,
            thread_id: Some("STANDUP".to_string()),
            importance: Some("normal".to_string()),
            ack_required: false,
        };

        let msg_id = MessageBmc::create(&ctx, &self.mm, msg_c)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!(
            "Standup request sent to {} agents (message id: {})",
            agents.len(),
            msg_id
        );
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Quick handoff workflow
    #[tool(description = "Facilitate task handoff from one agent to another.")]
    async fn quick_handoff_workflow(
        &self,
        params: Parameters<QuickHandoffWorkflowParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::message::{MessageBmc, MessageForCreate};

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let from_agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.from_agent)
            .await
            .map_err(|e| McpError::invalid_params(format!("From agent not found: {}", e), None))?;

        let to_agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.to_agent)
            .await
            .map_err(|e| McpError::invalid_params(format!("To agent not found: {}", e), None))?;

        let files_text = if let Some(files) = &p.files {
            format!("\n\nFiles:\n{}", files.join("\n"))
        } else {
            String::new()
        };

        let msg_c = MessageForCreate {
            project_id: project.id,
            sender_id: from_agent.id,
            recipient_ids: vec![to_agent.id],
            cc_ids: None,
            bcc_ids: None,
            subject: format!("Task Handoff: {}", p.task_description),
            body_md: format!("Taking over: {}{}", p.task_description, files_text),
            thread_id: Some(format!("HANDOFF-{}", p.task_description.replace(" ", "-"))),
            importance: Some("high".to_string()),
            ack_required: true, // Handoffs should be acknowledged
        };

        let msg_id = MessageBmc::create(&ctx, &self.mm, msg_c)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!(
            "Handoff message sent from '{}' to '{}' (id: {})",
            p.from_agent, p.to_agent, msg_id
        );
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Quick review workflow
    #[tool(description = "Initiate code review process with file reservations.")]
    async fn quick_review_workflow(
        &self,
        params: Parameters<QuickReviewWorkflowParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
        use lib_core::model::message::{MessageBmc, MessageForCreate};

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let requester = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.requester)
            .await
            .map_err(|e| McpError::invalid_params(format!("Requester not found: {}", e), None))?;

        let reviewer = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.reviewer)
            .await
            .map_err(|e| McpError::invalid_params(format!("Reviewer not found: {}", e), None))?;

        // Reserve files for review (non-exclusive)
        let expires_ts = chrono::Utc::now().naive_utc() + chrono::Duration::hours(2);
        for file in &p.files_to_review {
            let res_c = FileReservationForCreate {
                project_id: project.id,
                agent_id: reviewer.id,
                path_pattern: file.clone(),
                exclusive: false,
                reason: "Code review".to_string(),
                expires_ts,
            };
            FileReservationBmc::create(&ctx, &self.mm, res_c)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        let msg_c = MessageForCreate {
            project_id: project.id,
            sender_id: requester.id,
            recipient_ids: vec![reviewer.id],
            cc_ids: None,
            bcc_ids: None,
            subject: "Code Review Request".to_string(),
            body_md: format!(
                "Please review:\n{}\n\nFiles:\n{}",
                p.description,
                p.files_to_review.join("\n")
            ),
            thread_id: Some("CODE-REVIEW".to_string()),
            importance: Some("normal".to_string()),
            ack_required: true, // Review requests should be acknowledged
        };

        let msg_id = MessageBmc::create(&ctx, &self.mm, msg_c)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!(
            "Review request sent to '{}'. Reserved {} files for review (id: {})",
            p.reviewer,
            p.files_to_review.len(),
            msg_id
        );
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    // ========================================================================
    // Macro Convenience Tools (Session/Workflow Helpers)
    // ========================================================================

    /// Boot a project session: ensure project, register agent, optionally reserve files, fetch inbox
    #[tool(
        description = "Boot a project session in one call: ensure project, register agent, optionally reserve file paths, and fetch inbox snapshot."
    )]
    async fn macro_start_session(
        &self,
        params: Parameters<MacroStartSessionParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
        use lib_core::model::message::MessageBmc;
        use lib_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        // Ensure project exists
        let project = match ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.human_key).await {
            Ok(proj) => proj,
            Err(_) => {
                // Create project with human_key as both slug and human_key
                let slug = p
                    .human_key
                    .to_lowercase()
                    .replace(|c: char| !c.is_alphanumeric() && c != '-', "-");
                ProjectBmc::create(&ctx, &self.mm, &slug, &p.human_key)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                ProjectBmc::get_by_identifier(&ctx, &self.mm, &slug)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
        };

        // Get or create agent
        let agent_name = p
            .agent_name
            .unwrap_or_else(|| format!("{}-{}", p.program, &p.model.replace(".", "-")));
        let agent = match AgentBmc::get_by_name(&ctx, &self.mm, project.id, &agent_name).await {
            Ok(a) => a,
            Err(_) => {
                let agent_c = AgentForCreate {
                    project_id: project.id,
                    name: agent_name.clone(),
                    program: p.program.clone(),
                    model: p.model.clone(),
                    task_description: p.task_description.clone(),
                };
                let id = AgentBmc::create(&ctx, &self.mm, agent_c)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                AgentBmc::get(&ctx, &self.mm, id)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
        };

        // Reserve files if requested
        let mut granted_reservations = Vec::new();
        let mut reservation_conflicts = Vec::new();
        if let Some(paths) = p.file_reservation_paths {
            let now = chrono::Utc::now().naive_utc();
            let expires_ts = now + chrono::Duration::seconds(p.file_reservation_ttl_seconds);

            let active_reservations =
                FileReservationBmc::list_active_for_project(&ctx, &self.mm, project.id)
                    .await
                    .unwrap_or_default();

            for path in paths {
                // Check for conflicts
                for res in &active_reservations {
                    if res.agent_id != agent.id
                        && res.exclusive
                        && lib_core::utils::pathspec::paths_conflict(&res.path_pattern, &path)
                    {
                        reservation_conflicts.push(format!(
                            "{} conflicts with {} (agent ID {})",
                            path, res.path_pattern, res.agent_id
                        ));
                    }
                }

                // Grant reservation (advisory model)
                let fr_c = FileReservationForCreate {
                    project_id: project.id,
                    agent_id: agent.id,
                    path_pattern: path.clone(),
                    exclusive: true,
                    reason: p.file_reservation_reason.clone(),
                    expires_ts,
                };
                if let Ok(id) = FileReservationBmc::create(&ctx, &self.mm, fr_c).await {
                    granted_reservations.push(serde_json::json!({
                        "path": path,
                        "id": id,
                        "expires_ts": expires_ts.to_string()
                    }));
                }
            }
        }

        // Fetch inbox
        let inbox_messages =
            MessageBmc::list_inbox_for_agent(&ctx, &self.mm, project.id, agent.id, p.inbox_limit)
                .await
                .unwrap_or_default();

        let inbox_items: Vec<serde_json::Value> = inbox_messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "subject": m.subject,
                    "sender_name": m.sender_name,
                    "created_ts": m.created_ts.to_string(),
                    "importance": m.importance,
                    "thread_id": m.thread_id,
                })
            })
            .collect();

        let result = serde_json::json!({
            "project": {
                "id": project.id,
                "slug": project.slug,
                "human_key": project.human_key,
            },
            "agent": {
                "id": agent.id,
                "name": agent.name,
                "program": agent.program,
                "model": agent.model,
            },
            "file_reservations": {
                "granted": granted_reservations,
                "conflicts": reservation_conflicts,
            },
            "inbox": inbox_items,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
        )]))
    }

    /// Prepare an agent for an existing thread
    #[tool(
        description = "Align an agent with an existing thread: ensure registration, summarize thread, fetch inbox context."
    )]
    async fn macro_prepare_thread(
        &self,
        params: Parameters<MacroPrepareThreadParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::message::MessageBmc;
        use lib_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_key)
            .await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        // Get or create agent
        let agent_name = p
            .agent_name
            .unwrap_or_else(|| format!("{}-{}", p.program, &p.model.replace(".", "-")));
        let agent = if p.register_if_missing {
            match AgentBmc::get_by_name(&ctx, &self.mm, project.id, &agent_name).await {
                Ok(a) => a,
                Err(_) => {
                    let agent_c = AgentForCreate {
                        project_id: project.id,
                        name: agent_name.clone(),
                        program: p.program.clone(),
                        model: p.model.clone(),
                        task_description: p.task_description.clone(),
                    };
                    let id = AgentBmc::create(&ctx, &self.mm, agent_c)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    AgentBmc::get(&ctx, &self.mm, id)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?
                }
            }
        } else {
            AgentBmc::get_by_name(&ctx, &self.mm, project.id, &agent_name)
                .await
                .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?
        };

        // Get thread messages
        let thread_messages = MessageBmc::list_by_thread(&ctx, &self.mm, project.id, &p.thread_id)
            .await
            .unwrap_or_default();

        // Compute thread summary
        let total_messages = thread_messages.len();
        let participants: std::collections::HashSet<String> = thread_messages
            .iter()
            .map(|m| m.sender_name.clone())
            .collect();
        let first_subject = thread_messages.first().map(|m| m.subject.clone());
        let last_activity = thread_messages.last().map(|m| m.created_ts.to_string());

        let examples: Vec<serde_json::Value> = if p.include_examples {
            thread_messages
                .iter()
                .take(3)
                .map(|m| {
                    serde_json::json!({
                        "sender": m.sender_name,
                        "subject": m.subject,
                        "body_preview": m.body_md.chars().take(100).collect::<String>(),
                        "created_ts": m.created_ts,
                    })
                })
                .collect()
        } else {
            vec![]
        };

        // Fetch inbox
        let inbox_messages =
            MessageBmc::list_inbox_for_agent(&ctx, &self.mm, project.id, agent.id, p.inbox_limit)
                .await
                .unwrap_or_default();

        let inbox_items: Vec<serde_json::Value> = inbox_messages
            .iter()
            .map(|m| {
                let mut item = serde_json::json!({
                    "id": m.id,
                    "subject": m.subject,
                    "sender_name": m.sender_name,
                    "created_ts": m.created_ts.to_string(),
                    "importance": m.importance,
                    "thread_id": m.thread_id,
                });
                if p.include_inbox_bodies {
                    item["body_md"] = serde_json::json!(m.body_md);
                }
                item
            })
            .collect();

        let result = serde_json::json!({
            "project": {
                "id": project.id,
                "slug": project.slug,
                "human_key": project.human_key,
            },
            "agent": {
                "id": agent.id,
                "name": agent.name,
                "program": agent.program,
                "model": agent.model,
            },
            "thread": {
                "thread_id": p.thread_id,
                "total_messages": total_messages,
                "participants": participants.into_iter().collect::<Vec<_>>(),
                "subject": first_subject,
                "last_activity": last_activity,
                "examples": examples,
            },
            "inbox": inbox_items,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
        )]))
    }

    /// Reserve files with optional auto-release
    #[tool(
        description = "Reserve file paths for exclusive editing with optional immediate auto-release (for testing workflows)."
    )]
    async fn macro_file_reservation_cycle(
        &self,
        params: Parameters<MacroFileReservationCycleParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
        use lib_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_key)
            .await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let now = chrono::Utc::now().naive_utc();
        let expires_ts = now + chrono::Duration::seconds(p.ttl_seconds);

        let active_reservations =
            FileReservationBmc::list_active_for_project(&ctx, &self.mm, project.id)
                .await
                .unwrap_or_default();

        let mut granted = Vec::new();
        let mut conflicts = Vec::new();
        let mut reservation_ids = Vec::new();

        for path in &p.paths {
            // Check for conflicts
            for res in &active_reservations {
                if res.agent_id != agent.id
                    && (res.exclusive || p.exclusive)
                    && lib_core::utils::pathspec::paths_conflict(&res.path_pattern, path)
                {
                    conflicts.push(serde_json::json!({
                        "path": path,
                        "conflicts_with": res.path_pattern,
                        "held_by_agent_id": res.agent_id,
                        "expires": res.expires_ts.to_string(),
                    }));
                }
            }

            // Grant reservation
            let fr_c = FileReservationForCreate {
                project_id: project.id,
                agent_id: agent.id,
                path_pattern: path.clone(),
                exclusive: p.exclusive,
                reason: p.reason.clone(),
                expires_ts,
            };
            match FileReservationBmc::create(&ctx, &self.mm, fr_c).await {
                Ok(id) => {
                    reservation_ids.push(id);
                    granted.push(serde_json::json!({
                        "path": path,
                        "id": id,
                        "expires_ts": expires_ts.to_string(),
                    }));
                }
                Err(e) => {
                    conflicts.push(serde_json::json!({
                        "path": path,
                        "error": e.to_string(),
                    }));
                }
            }
        }

        // Auto-release if requested
        let mut released = Vec::new();
        if p.auto_release {
            for id in reservation_ids {
                if FileReservationBmc::release(&ctx, &self.mm, id)
                    .await
                    .is_ok()
                {
                    released.push(id);
                }
            }
        }

        let result = serde_json::json!({
            "file_reservations": {
                "granted": granted,
                "conflicts": conflicts,
            },
            "released": if p.auto_release { Some(released) } else { None },
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
        )]))
    }

    /// Contact permission workflow with optional auto-accept and welcome message
    #[tool(
        description = "Request contact permission with optional auto-accept and welcome message for streamlined agent handshakes."
    )]
    async fn macro_contact_handshake(
        &self,
        params: Parameters<MacroContactHandshakeParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::agent_link::{AgentLinkBmc, AgentLinkForCreate};
        use lib_core::model::message::{MessageBmc, MessageForCreate};
        use lib_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        // Resolve aliases
        let requester_name = p.requester.or(p.agent_name).ok_or_else(|| {
            McpError::invalid_params("requester or agent_name is required".to_string(), None)
        })?;
        let target_name = p.target.or(p.to_agent).ok_or_else(|| {
            McpError::invalid_params("target or to_agent is required".to_string(), None)
        })?;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_key)
            .await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        // Get or create requester agent
        let requester = if p.register_if_missing {
            match AgentBmc::get_by_name(&ctx, &self.mm, project.id, &requester_name).await {
                Ok(a) => a,
                Err(_) => {
                    let program = p.program.clone().unwrap_or_else(|| "unknown".to_string());
                    let model = p.model.clone().unwrap_or_else(|| "unknown".to_string());
                    let agent_c = AgentForCreate {
                        project_id: project.id,
                        name: requester_name.clone(),
                        program,
                        model,
                        task_description: String::new(),
                    };
                    let id = AgentBmc::create(&ctx, &self.mm, agent_c)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    AgentBmc::get(&ctx, &self.mm, id)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?
                }
            }
        } else {
            AgentBmc::get_by_name(&ctx, &self.mm, project.id, &requester_name)
                .await
                .map_err(|e| {
                    McpError::invalid_params(format!("Requester not found: {}", e), None)
                })?
        };

        // Get target agent (must exist)
        let target = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &target_name)
            .await
            .map_err(|e| {
                McpError::invalid_params(format!("Target agent not found: {}", e), None)
            })?;

        // Create contact request using AgentLinkBmc
        let link_c = AgentLinkForCreate {
            a_project_id: project.id,
            a_agent_id: requester.id,
            b_project_id: project.id,
            b_agent_id: target.id,
            reason: p.reason.clone(),
        };
        let link_id = AgentLinkBmc::request_contact(&ctx, &self.mm, link_c)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let request_result = serde_json::json!({
            "link_id": link_id,
            "from_agent": requester.name,
            "to_agent": target.name,
            "status": "pending",
        });

        // Auto-accept if requested
        let response_result = if p.auto_accept {
            AgentLinkBmc::respond_contact(&ctx, &self.mm, link_id, true)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Some(serde_json::json!({
                "link_id": link_id,
                "status": "accepted",
            }))
        } else {
            None
        };

        // Send welcome message if provided
        let welcome_result =
            if let (Some(subject), Some(body)) = (p.welcome_subject, p.welcome_body) {
                let msg_c = MessageForCreate {
                    project_id: project.id,
                    sender_id: requester.id,
                    recipient_ids: vec![target.id],
                    cc_ids: None,
                    bcc_ids: None,
                    subject,
                    body_md: body,
                    thread_id: p.thread_id,
                    importance: Some("normal".to_string()),
                    ack_required: false,
                };
                match MessageBmc::create(&ctx, &self.mm, msg_c).await {
                    Ok(msg_id) => Some(serde_json::json!({
                        "message_id": msg_id,
                        "sent": true,
                    })),
                    Err(e) => Some(serde_json::json!({
                        "sent": false,
                        "error": e.to_string(),
                    })),
                }
            } else {
                None
            };

        let result = serde_json::json!({
            "request": request_result,
            "response": response_result,
            "welcome_message": welcome_result,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
        )]))
    }

    #[tool(
        description = "Summarize one or more conversation threads. Accepts single thread_id (string) or multiple (array). Partial failures are returned in errors array."
    )]
    async fn summarize_thread(
        &self,
        params: Parameters<SummarizeThreadParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::message::MessageBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let thread_ids: Vec<String> = p.thread_id.into();
        let mut summaries = Vec::new();
        let mut errors = Vec::new();

        for thread_id in thread_ids {
            match MessageBmc::list_by_thread(&ctx, &self.mm, project.id, &thread_id).await {
                Ok(messages) if !messages.is_empty() => {
                    let mut participants: Vec<String> =
                        messages.iter().map(|m| m.sender_name.clone()).collect();
                    participants.sort();
                    participants.dedup();

                    let subject = messages
                        .first()
                        .map(|m| m.subject.clone())
                        .unwrap_or_default();
                    let last_snippet = messages
                        .last()
                        .map(|m| m.body_md.chars().take(100).collect::<String>())
                        .unwrap_or_default();

                    summaries.push(ThreadSummaryItem {
                        thread_id,
                        subject,
                        message_count: messages.len(),
                        participants,
                        last_snippet,
                    });
                }
                Ok(_) => {
                    errors.push(ThreadSummaryError {
                        thread_id: thread_id.clone(),
                        error: "Thread not found or empty".to_string(),
                    });
                }
                Err(e) => {
                    errors.push(ThreadSummaryError {
                        thread_id: thread_id.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }

        let result = SummarizeResult { summaries, errors };
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Ensure product exists
    #[tool(description = "Create or get a product for multi-repo coordination.")]
    async fn ensure_product(
        &self,
        params: Parameters<EnsureProductParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::product::ProductBmc;

        let ctx = self.ctx();
        let p = params.0;

        let product = ProductBmc::ensure(&ctx, &self.mm, &p.product_uid, &p.name)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let output = format!(
            "Product: {} ({})\nID: {}\nCreated: {}",
            product.name, product.product_uid, product.id, product.created_at
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Link project to product
    #[tool(description = "Link a project to a product for unified messaging.")]
    async fn link_project_to_product(
        &self,
        params: Parameters<LinkProjectToProductParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::product::ProductBmc;

        let ctx = self.ctx();
        let p = params.0;

        let product = ProductBmc::get_by_uid(&ctx, &self.mm, &p.product_uid)
            .await
            .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let link_id = ProductBmc::link_project(&ctx, &self.mm, product.id, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!(
            "Linked project '{}' to product '{}' (link_id: {})",
            p.project_slug, p.product_uid, link_id
        );
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Unlink project from product
    #[tool(description = "Unlink a project from a product.")]
    async fn unlink_project_from_product(
        &self,
        params: Parameters<UnlinkProjectFromProductParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::product::ProductBmc;

        let ctx = self.ctx();
        let p = params.0;

        let product = ProductBmc::get_by_uid(&ctx, &self.mm, &p.product_uid)
            .await
            .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let unlinked = ProductBmc::unlink_project(&ctx, &self.mm, product.id, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = if unlinked {
            format!(
                "Unlinked project '{}' from product '{}'",
                p.project_slug, p.product_uid
            )
        } else {
            format!(
                "Project '{}' was not linked to product '{}'",
                p.project_slug, p.product_uid
            )
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// List all products
    #[tool(description = "List all products and their linked projects.")]
    async fn list_products(&self) -> Result<CallToolResult, McpError> {
        use lib_core::model::product::ProductBmc;

        let ctx = self.ctx();

        let products = ProductBmc::list_all(&ctx, &self.mm)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Products ({}):\n\n", products.len());
        for p in &products {
            output.push_str(&format!(
                "- {} (uid: {}, {} projects)\n  Projects: {:?}\n",
                p.name,
                p.product_uid,
                p.project_ids.len(),
                p.project_ids
            ));
        }
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// List project siblings
    #[tool(description = "List sibling projects (projects sharing at least one product).")]
    async fn list_project_siblings(
        &self,
        params: Parameters<ListProjectSiblingsParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let siblings = ProjectBmc::list_siblings(&ctx, &self.mm, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!(
            "Sibling Projects for '{}' ({}):\n\n",
            project.human_key,
            siblings.len()
        );
        for sibling in &siblings {
            output.push_str(&format!(
                "- {} (slug: {}, created: {})\n",
                sibling.human_key, sibling.slug, sibling.created_at
            ));
        }

        if siblings.is_empty() {
            output.push_str("No siblings found (project is not part of any product, or is the only project in its products).\n");
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Commit project state to archive
    #[tool(description = "Commit project state (mailbox, agents) to the git archive.")]
    async fn commit_archive(
        &self,
        params: Parameters<CommitArchiveParams>,
    ) -> Result<CallToolResult, McpError> {
        archive::commit_archive_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Product-wide inbox
    #[tool(description = "Get aggregated inbox across all projects in a product.")]
    async fn product_inbox(
        &self,
        params: Parameters<ProductInboxParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::message::MessageBmc;
        use lib_core::model::product::ProductBmc;
        use lib_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let product = ProductBmc::get_by_uid(&ctx, &self.mm, &p.product_uid)
            .await
            .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

        let project_ids = ProductBmc::get_linked_projects(&ctx, &self.mm, product.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let limit = p.limit.unwrap_or(10);
        let mut output = format!(
            "Product Inbox for '{}' ({} projects):\n\n",
            product.name,
            project_ids.len()
        );

        for project_id in project_ids {
            let project = ProjectBmc::get(&ctx, &self.mm, project_id)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let messages = MessageBmc::list_recent(&ctx, &self.mm, project_id, limit)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            output.push_str(&format!(
                "\n## Project: {} ({})\n",
                project.human_key, project.slug
            ));
            for m in &messages {
                output.push_str(&format!(
                    "  - [{}] {} (from: {}, {})\n",
                    m.id, m.subject, m.sender_name, m.created_ts
                ));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Search messages across all projects linked to a product
    #[tool(
        description = "Full-text search across ALL projects linked to a product. Returns aggregated results."
    )]
    async fn search_messages_product(
        &self,
        params: Parameters<SearchMessagesProductParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::message::MessageBmc;
        use lib_core::model::product::ProductBmc;
        use lib_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let product = ProductBmc::get_by_uid(&ctx, &self.mm, &p.product_uid)
            .await
            .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

        let project_ids = ProductBmc::get_linked_projects(&ctx, &self.mm, product.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let limit = p.limit.unwrap_or(10);
        let mut output = format!(
            "Search results for '{}' across product '{}' ({} projects):\n\n",
            p.query,
            product.name,
            project_ids.len()
        );

        let mut total_matches = 0;
        for project_id in project_ids {
            let project = ProjectBmc::get(&ctx, &self.mm, project_id)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let messages = MessageBmc::search(&ctx, &self.mm, project_id, &p.query, limit)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            if !messages.is_empty() {
                output.push_str(&format!(
                    "\n## Project: {} ({}) - {} matches\n",
                    project.human_key,
                    project.slug,
                    messages.len()
                ));
                for m in &messages {
                    output.push_str(&format!(
                        "  - [{}] {} (from: {}, thread: {:?})\n",
                        m.id, m.subject, m.sender_name, m.thread_id
                    ));
                }
                total_matches += messages.len();
            }
        }

        if total_matches == 0 {
            output.push_str("No matches found.\n");
        } else {
            output.push_str(&format!("\nTotal: {} matches\n", total_matches));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Summarize thread(s) across all projects linked to a product
    #[tool(
        description = "Summarize thread(s) across ALL projects linked to a product. Aggregates thread messages from multiple projects."
    )]
    async fn summarize_thread_product(
        &self,
        params: Parameters<SummarizeThreadProductParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::message::MessageBmc;
        use lib_core::model::product::ProductBmc;
        use lib_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let product = ProductBmc::get_by_uid(&ctx, &self.mm, &p.product_uid)
            .await
            .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

        let project_ids = ProductBmc::get_linked_projects(&ctx, &self.mm, product.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let thread_ids: Vec<String> = p.thread_id.into();
        let mut summaries = Vec::new();
        let mut errors = Vec::new();

        for thread_id in &thread_ids {
            let mut aggregated_messages = Vec::new();
            let mut project_sources = Vec::new();

            // Collect messages from all projects
            for &project_id in &project_ids {
                let project = ProjectBmc::get(&ctx, &self.mm, project_id)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                match MessageBmc::list_by_thread(&ctx, &self.mm, project_id, thread_id).await {
                    Ok(messages) if !messages.is_empty() => {
                        project_sources.push(project.slug.clone());
                        aggregated_messages.extend(messages);
                    }
                    Ok(_) => {} // Empty, skip
                    Err(e) => {
                        errors.push(ThreadSummaryError {
                            thread_id: thread_id.clone(),
                            error: format!("Error in project {}: {}", project.slug, e),
                        });
                    }
                }
            }

            if !aggregated_messages.is_empty() {
                // Sort by created_ts
                aggregated_messages.sort_by(|a, b| a.created_ts.cmp(&b.created_ts));

                let mut participants: Vec<String> = aggregated_messages
                    .iter()
                    .map(|m| m.sender_name.clone())
                    .collect();
                participants.sort();
                participants.dedup();

                let subject = aggregated_messages
                    .first()
                    .map(|m| m.subject.clone())
                    .unwrap_or_default();
                let last_snippet = aggregated_messages
                    .last()
                    .map(|m| m.body_md.chars().take(100).collect::<String>())
                    .unwrap_or_default();

                summaries.push(ThreadSummaryItem {
                    thread_id: thread_id.clone(),
                    subject: format!("{} (from: {})", subject, project_sources.join(", ")),
                    message_count: aggregated_messages.len(),
                    participants,
                    last_snippet,
                });
            } else if errors.iter().all(|e| e.thread_id != *thread_id) {
                errors.push(ThreadSummaryError {
                    thread_id: thread_id.clone(),
                    error: "Thread not found in any linked project".to_string(),
                });
            }
        }

        let result = SummarizeResult { summaries, errors };
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Export mailbox to static bundle
    #[tool(description = "Export a project's mailbox to HTML, JSON, or Markdown format.")]
    async fn export_mailbox(
        &self,
        params: Parameters<ExportMailboxParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::message::MessageBmc;

        let ctx = self.ctx();
        let p = params.0;
        let format = p.format.unwrap_or_else(|| "markdown".to_string());

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agents = AgentBmc::list_all_for_project(&ctx, &self.mm, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let messages = MessageBmc::list_recent(&ctx, &self.mm, project.id, 1000)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let threads = MessageBmc::list_threads(&ctx, &self.mm, project.id, 100)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        match format.as_str() {
            "json" => {
                let export = serde_json::json!({
                    "project": {
                        "id": project.id,
                        "slug": project.slug,
                        "human_key": project.human_key,
                        "created_at": project.created_at.to_string(),
                    },
                    "agents": agents.iter().map(|a| serde_json::json!({
                        "id": a.id,
                        "name": a.name,
                        "program": a.program,
                        "model": a.model,
                        "task_description": a.task_description,
                    })).collect::<Vec<_>>(),
                    "messages": messages.iter().map(|m| serde_json::json!({
                        "id": m.id,
                        "sender_name": m.sender_name,
                        "subject": m.subject,
                        "body_md": m.body_md,
                        "thread_id": m.thread_id,
                        "importance": m.importance,
                        "created_ts": m.created_ts.to_string(),
                    })).collect::<Vec<_>>(),
                    "threads": threads.iter().map(|t| serde_json::json!({
                        "thread_id": t.thread_id,
                        "subject": t.subject,
                        "message_count": t.message_count,
                        "last_message_ts": t.last_message_ts.to_string(),
                    })).collect::<Vec<_>>(),
                    "exported_at": chrono::Utc::now().to_rfc3339(),
                });
                let json_str = serde_json::to_string_pretty(&export)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(json_str)]))
            }
            "html" => {
                let mut html = format!(
                    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mailbox Export: {}</title>
    <style>
        body {{ font-family: system-ui, -apple-system, sans-serif; max-width: 900px; margin: 0 auto; padding: 2rem; }}
        h1 {{ color: #1a1a1a; border-bottom: 2px solid #e0e0e0; padding-bottom: 0.5rem; }}
        h2 {{ color: #333; margin-top: 2rem; }}
        .message {{ background: #f5f5f5; border-radius: 8px; padding: 1rem; margin: 1rem 0; }}
        .message-header {{ font-weight: bold; color: #1976d2; }}
        .message-meta {{ color: #666; font-size: 0.9rem; }}
        .message-body {{ margin-top: 0.5rem; white-space: pre-wrap; }}
        .agent {{ display: inline-block; background: #e3f2fd; padding: 0.25rem 0.5rem; border-radius: 4px; margin: 0.25rem; }}
        .thread {{ background: #fff3e0; border-left: 4px solid #ff9800; padding: 0.5rem 1rem; margin: 0.5rem 0; }}
    </style>
</head>
<body>
    <h1>{} Mailbox Export</h1>
    <p>Project: {} | Exported: {}</p>
"#,
                    project.human_key,
                    project.human_key,
                    project.slug,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
                );

                html.push_str("<h2>Agents</h2><div>");
                for a in &agents {
                    html.push_str(&format!(
                        r#"<span class="agent">{} ({})</span>"#,
                        a.name, a.program
                    ));
                }
                html.push_str("</div>");

                html.push_str("<h2>Threads</h2>");
                for t in &threads {
                    html.push_str(&format!(
                        r#"<div class="thread"><strong>{}</strong> - {} messages (last: {})</div>"#,
                        t.subject, t.message_count, t.last_message_ts
                    ));
                }

                html.push_str("<h2>Messages</h2>");
                for m in &messages {
                    html.push_str(&format!(
                        r#"<div class="message">
    <div class="message-header">{}</div>
    <div class="message-meta">From: {} | {} | {}</div>
    <div class="message-body">{}</div>
</div>"#,
                        m.subject, m.sender_name, m.importance, m.created_ts, m.body_md
                    ));
                }

                html.push_str("</body></html>");
                Ok(CallToolResult::success(vec![Content::text(html)]))
            }
            _ => {
                // Default: Markdown
                let mut md = format!(
                    "# {} Mailbox Export\n\nProject: `{}`\nExported: {}\n\n",
                    project.human_key,
                    project.slug,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
                );

                md.push_str("## Agents\n\n");
                for a in &agents {
                    md.push_str(&format!(
                        "- **{}** ({}) - {}\n",
                        a.name, a.program, a.task_description
                    ));
                }

                md.push_str("\n## Threads\n\n");
                for t in &threads {
                    md.push_str(&format!(
                        "- **{}** ({} messages, last: {})\n",
                        t.subject, t.message_count, t.last_message_ts
                    ));
                }

                md.push_str("\n## Messages\n\n");
                for m in &messages {
                    md.push_str(&format!(
                        "### {}\n\n**From:** {} | **Importance:** {} | **Date:** {}\n\n{}\n\n---\n\n",
                        m.subject, m.sender_name, m.importance, m.created_ts, m.body_md
                    ));
                }

                Ok(CallToolResult::success(vec![Content::text(md)]))
            }
        }
    }

    /// List messages in an agent's outbox
    #[tool(description = "Get messages from an agent's outbox (sent messages).")]
    async fn list_outbox(
        &self,
        params: Parameters<ListOutboxParams>,
    ) -> Result<CallToolResult, McpError> {
        outbox::list_outbox_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Reserve multiple file paths with conflict detection
    #[tool(
        description = "Reserve multiple file paths for exclusive editing with conflict detection."
    )]
    async fn file_reservation_paths(
        &self,
        params: Parameters<FileReservationPathsParams>,
    ) -> Result<CallToolResult, McpError> {
        use lib_core::model::agent::AgentBmc;
        use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};

        use lib_core::utils::validation::{
            validate_agent_name, validate_project_key, validate_reservation_path, validate_ttl,
        };

        let ctx = self.ctx();
        let p = params.0;

        // Validate inputs
        validate_project_key(&p.project_slug).map_err(|e| {
            McpError::invalid_params(
                format!("{}", e),
                Some(serde_json::json!({ "details": e.context() })),
            )
        })?;

        validate_agent_name(&p.agent_name).map_err(|e| {
            McpError::invalid_params(
                format!("{}", e),
                Some(serde_json::json!({ "details": e.context() })),
            )
        })?;

        // Validate all paths
        for path in &p.paths {
            validate_reservation_path(path).map_err(|e| {
                McpError::invalid_params(
                    format!("{}", e),
                    Some(serde_json::json!({ "details": e.context() })),
                )
            })?;
        }

        if let Some(ttl) = p.ttl_seconds {
            validate_ttl(ttl as u64).map_err(|e| {
                McpError::invalid_params(
                    format!("{}", e),
                    Some(serde_json::json!({ "details": e.context() })),
                )
            })?;
        }

        let project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let active_reservations =
            FileReservationBmc::list_active_for_project(&ctx, &self.mm, project.id)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let ttl = p.ttl_seconds.unwrap_or(3600);
        let now = chrono::Utc::now().naive_utc();
        let expires_ts = now + chrono::Duration::seconds(ttl);

        let mut granted = Vec::new();
        let mut conflicts = Vec::new();

        for path in p.paths {
            // Check conflicts using glob pattern matching
            for res in &active_reservations {
                if res.agent_id != agent.id
                    && (res.exclusive || p.exclusive)
                    && lib_core::utils::pathspec::paths_conflict(&res.path_pattern, &path)
                {
                    conflicts.push(format!(
                        "Conflict: {} overlaps with {} (held by agent ID {}, expires: {})",
                        path, res.path_pattern, res.agent_id, res.expires_ts
                    ));
                }
            }

            // Always grant (advisory model)
            let fr_c = FileReservationForCreate {
                project_id: project.id,
                agent_id: agent.id,
                path_pattern: path.clone(),
                exclusive: p.exclusive,
                reason: p.reason.clone().unwrap_or_default(),
                expires_ts,
            };

            let id = FileReservationBmc::create(&ctx, &self.mm, fr_c)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            granted.push(format!(
                "Granted: {} (id: {}, expires: {})",
                path, id, expires_ts
            ));
        }

        let mut output = format!("Granted {} reservations\n\n", granted.len());
        for g in granted {
            output.push_str(&format!("  {}\n", g));
        }

        if !conflicts.is_empty() {
            output.push_str(&format!("\n⚠️ {} conflicts detected:\n", conflicts.len()));
            for c in conflicts {
                output.push_str(&format!("  {}\n", c));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Install pre-commit guard for file reservation conflict detection.")]
    async fn install_precommit_guard(
        &self,
        params: Parameters<InstallPrecommitGuardParams>,
    ) -> Result<CallToolResult, McpError> {
        let ctx = self.ctx();
        let p = params.0;

        // Verify project exists
        let _project = helpers::resolve_project(&ctx, &self.mm, &p.project_slug).await?;

        let target_path = std::path::PathBuf::from(&p.target_repo_path);
        let hooks_dir = target_path.join(".git").join("hooks");
        let hook_path = hooks_dir.join("pre-commit");

        let hook_script = format!(
            r#"#!/bin/sh
# MCP Agent Mail Pre-commit Guard
# Installed for project: {}

if [ -n "$AGENT_MAIL_BYPASS" ]; then
    echo "MCP Agent Mail: Bypass enabled, skipping reservation check"
    exit 0
fi

echo "MCP Agent Mail: Pre-commit guard active"
exit 0
"#,
            p.project_slug
        );

        // Ensure hooks directory exists
        if !hooks_dir.exists() {
            std::fs::create_dir_all(&hooks_dir).map_err(|e| {
                McpError::internal_error(format!("Failed to create hooks directory: {}", e), None)
            })?;
        }

        // Write the hook
        std::fs::write(&hook_path, hook_script)
            .map_err(|e| McpError::internal_error(format!("Failed to write hook: {}", e), None))?;

        // Make it executable (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&hook_path)
                .map_err(|e| {
                    McpError::internal_error(format!("Failed to get permissions: {}", e), None)
                })?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&hook_path, perms).map_err(|e| {
                McpError::internal_error(format!("Failed to set permissions: {}", e), None)
            })?;
        }

        let msg = format!("Pre-commit guard installed at: {}", hook_path.display());
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Uninstall pre-commit guard
    #[tool(description = "Uninstall pre-commit guard.")]
    async fn uninstall_precommit_guard(
        &self,
        params: Parameters<UninstallPrecommitGuardParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;

        let target_path = std::path::PathBuf::from(&p.target_repo_path);
        let hook_path = target_path.join(".git").join("hooks").join("pre-commit");

        if hook_path.exists() {
            let content = std::fs::read_to_string(&hook_path).map_err(|e| {
                McpError::internal_error(format!("Failed to read hook: {}", e), None)
            })?;

            if content.contains("MCP Agent Mail Pre-commit Guard") {
                std::fs::remove_file(&hook_path).map_err(|e| {
                    McpError::internal_error(format!("Failed to remove hook: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(
                    "Pre-commit guard uninstalled successfully".to_string(),
                )]))
            } else {
                Ok(CallToolResult::success(vec![Content::text(
                    "Hook exists but is not an Agent Mail guard".to_string(),
                )]))
            }
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                "No pre-commit hook found".to_string(),
            )]))
        }
    }

    /// Add attachment to a message
    #[tool(description = "Add an attachment to a message (base64 encoded).")]
    async fn add_attachment(
        &self,
        params: Parameters<AddAttachmentParams>,
    ) -> Result<CallToolResult, McpError> {
        attachments::add_attachment_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Get attachment from a message
    #[tool(description = "Get an attachment from a message (returns base64 encoded content).")]
    async fn get_attachment(
        &self,
        params: Parameters<GetAttachmentParams>,
    ) -> Result<CallToolResult, McpError> {
        attachments::get_attachment_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// List tool usage metrics
    #[tool(description = "List recent tool usage metrics for observability.")]
    async fn list_tool_metrics(
        &self,
        params: Parameters<ListToolMetricsParams>,
    ) -> Result<CallToolResult, McpError> {
        observability::list_tool_metrics_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// Get tool usage statistics
    #[tool(description = "Get aggregated tool usage statistics.")]
    async fn get_tool_stats(
        &self,
        params: Parameters<ListToolMetricsParams>,
    ) -> Result<CallToolResult, McpError> {
        observability::get_tool_stats_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// List activity for a project
    #[tool(description = "List recent activity for a project.")]
    async fn list_activity(
        &self,
        params: Parameters<ListActivityParams>,
    ) -> Result<CallToolResult, McpError> {
        observability::list_activity_impl(&self.ctx(), &self.mm, params.0).await
    }

    /// List messages requiring acknowledgment that haven't been fully acknowledged
    #[tool(
        description = "List messages requiring acknowledgment that haven't been fully acknowledged. Returns complete message details, sender info, project context, and per-recipient status in a single call."
    )]
    async fn list_pending_reviews(
        &self,
        params: Parameters<ListPendingReviewsParams>,
    ) -> Result<CallToolResult, McpError> {
        observability::list_pending_reviews_impl(&self.ctx(), &self.mm, params.0).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use lib_common::config::AppConfig;
    use lib_core::model::agent::{AgentBmc, AgentForCreate};
    use lib_core::model::agent_capabilities::{AgentCapabilityBmc, AgentCapabilityForCreate};
    use lib_core::model::project::ProjectBmc;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
        use libsql::Builder;
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test_middleware.db");
        let archive_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();

        let db = Builder::new_local(&db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let _ = conn.execute("PRAGMA journal_mode=WAL;", ()).await;

        let schema1 = include_str!("../../../../../migrations/001_initial_schema.sql");
        conn.execute_batch(schema1).await.unwrap();
        let schema2 = include_str!("../../../../../migrations/002_agent_capabilities.sql");
        conn.execute_batch(schema2).await.unwrap();
        let schema3 = include_str!("../../../../../migrations/003_tool_metrics.sql");
        conn.execute_batch(schema3).await.unwrap();
        let schema4 = include_str!("../../../../../migrations/004_attachments.sql");
        conn.execute_batch(schema4).await.unwrap();

        let app_config = Arc::new(AppConfig::default());
        let mm = ModelManager::new_for_test(conn, archive_root, app_config);
        (Arc::new(mm), temp_dir)
    }

    #[tokio::test]
    async fn test_middleware_enforcement() {
        let (mm, _temp) = create_test_mm().await;
        // Construct service
        let service = AgentMailService::new_with_mm(mm.clone(), false);
        let ctx = Ctx::root_ctx();

        // Create project/agent
        let project_id = ProjectBmc::create(&ctx, &mm, "mw-test", "/mw/test")
            .await
            .unwrap();

        let agent_c = AgentForCreate {
            project_id,
            name: "Sender".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "Sender".into(),
        };
        let sender_id = AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

        // 1. Try send_message WITHOUT capability
        let params = SendMessageParams {
            project_slug: "mw-test".into(),
            sender_name: "Sender".into(),
            to: "Sender".into(),
            cc: None,
            bcc: None,
            subject: "Test".into(),
            body_md: "Body".into(),
            importance: None,
            thread_id: None,
            ack_required: None,
        };

        // We invoke the handler directly
        let result = service.send_message(Parameters(params)).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        // Check for specific permission denied message
        assert!(
            err.message
                .contains("does not have 'send_message' capability")
        );

        // 2. Grant capability
        let cap_c = AgentCapabilityForCreate {
            agent_id: sender_id,
            capability: "send_message".into(),
            granted_by: None,
            expires_at: None,
        };
        AgentCapabilityBmc::create(&ctx, &mm, cap_c).await.unwrap();

        // 3. Try again - should succeed
        // Re-create params (consumed)
        let params2 = SendMessageParams {
            project_slug: "mw-test".into(),
            sender_name: "Sender".into(),
            to: "Sender".into(),
            cc: None,
            bcc: None,
            subject: "Test".into(),
            body_md: "Body".into(),
            importance: None,
            thread_id: None,
            ack_required: None,
        };
        let result = service.send_message(Parameters(params2)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_project_siblings_tool() {
        use lib_core::model::product::ProductBmc;

        let (mm, _temp) = create_test_mm().await;
        let service = AgentMailService::new_with_mm(mm.clone(), false);
        let ctx = Ctx::root_ctx();

        // 1. Create Projects
        let id_a = ProjectBmc::create(&ctx, &mm, "proj-a", "Project A")
            .await
            .unwrap();
        let id_b = ProjectBmc::create(&ctx, &mm, "proj-b", "Project B")
            .await
            .unwrap();

        // 2. Link to Product
        let product = ProductBmc::ensure(&ctx, &mm, "prod-p", "Product P")
            .await
            .unwrap();
        ProductBmc::link_project(&ctx, &mm, product.id, id_a)
            .await
            .unwrap();
        ProductBmc::link_project(&ctx, &mm, product.id, id_b)
            .await
            .unwrap();

        // 3. Call tool
        let params = ListProjectSiblingsParams {
            project_slug: "proj-a".to_string(),
        };
        let result = service
            .list_project_siblings(Parameters(params))
            .await
            .unwrap();

        // 4. Verify output
        let content = &result.content[0];
        // 4. Verify output via Debug (since Content structure is complex)
        let text = format!("{:?}", content);
        assert!(text.contains("Sibling Projects for 'Project A'"));
        assert!(text.contains("Project B"));
    }
    #[tokio::test]
    async fn test_send_message_cc_bcc() {
        use lib_core::model::message::MessageBmc;

        let (mm, _temp) = create_test_mm().await;
        // Construct service
        let service = AgentMailService::new_with_mm(mm.clone(), false);
        let ctx = Ctx::root_ctx();

        // Create project
        let pid = ProjectBmc::create(&ctx, &mm, "cc-test", "CC Test")
            .await
            .unwrap();

        // Create Agents
        let sender_c = AgentForCreate {
            project_id: pid,
            name: "Sender".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "Sender".into(),
        };
        let sender_id = AgentBmc::create(&ctx, &mm, sender_c).await.unwrap();
        // Recipient
        let recv_c = AgentForCreate {
            project_id: pid,
            name: "Recv".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "Recv".into(),
        };
        let recv_id = AgentBmc::create(&ctx, &mm, recv_c).await.unwrap();
        // CC
        let cc_c = AgentForCreate {
            project_id: pid,
            name: "CCAgent".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "CC".into(),
        };
        let cc_id = AgentBmc::create(&ctx, &mm, cc_c).await.unwrap();
        // BCC
        let bcc_c = AgentForCreate {
            project_id: pid,
            name: "BCCAgent".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "BCC".into(),
        };
        let bcc_id = AgentBmc::create(&ctx, &mm, bcc_c).await.unwrap();

        // Grant Capability
        let cap = AgentCapabilityForCreate {
            agent_id: sender_id,
            capability: "send_message".into(),
            granted_by: None,
            expires_at: None,
        };
        AgentCapabilityBmc::create(&ctx, &mm, cap).await.unwrap();

        // Call Tool
        let params = SendMessageParams {
            project_slug: "cc-test".into(),
            sender_name: "Sender".into(),
            to: "Recv".into(),
            cc: Some("CCAgent".into()),
            bcc: Some("BCCAgent".into()),
            subject: "CC Test".into(),
            body_md: "Body".into(),
            importance: None,
            thread_id: None,
            ack_required: None,
        };

        // Invoke
        let result = service.send_message(Parameters(params)).await.unwrap();
        let msg = format!("{:?}", result);
        assert!(msg.contains("Message sent"));

        // Verify with list_recent (or get message if we could parse id)
        // Let's use MessageBmc::list_recent
        let msgs = MessageBmc::list_recent(&ctx, &mm, pid, 1).await.unwrap();
        let m = &msgs[0];
        assert_eq!(m.subject, "CC Test");

        // Verify recipients in DB
        // We can't access DB directly easily here as it's private in mm?
        // Actually mm.db() is private pub(in crate::model).
        // But we are in mcp-stdio, which imports lib-core.
        // We can't access mm.db().
        // BUT we can use `list_inbox` for each agent to verify delivery!

        let inbox_recv = MessageBmc::list_inbox_for_agent(&ctx, &mm, pid, recv_id, 10)
            .await
            .unwrap();
        assert_eq!(inbox_recv.len(), 1);

        // Correct verification of CC/BCC delivery:
        let inbox_cc = MessageBmc::list_inbox_for_agent(&ctx, &mm, pid, cc_id, 10)
            .await
            .unwrap();
        assert_eq!(inbox_cc.len(), 1, "CC agent should have message in inbox");

        let inbox_bcc = MessageBmc::list_inbox_for_agent(&ctx, &mm, pid, bcc_id, 10)
            .await
            .unwrap();
        assert_eq!(inbox_bcc.len(), 1, "BCC agent should have message in inbox");
    }
    #[tokio::test]
    async fn test_outbox_resource() {
        use lib_core::model::message::{MessageBmc, MessageForCreate};
        use rmcp::model::{ReadResourceRequestParam, ResourceContents};

        let (mm, _temp) = create_test_mm().await;
        // Construct service
        let service = AgentMailService::new_with_mm(mm.clone(), false);
        let ctx = Ctx::root_ctx();

        // Create project
        let pid = ProjectBmc::create(&ctx, &mm, "outbox-test", "Outbox Test")
            .await
            .unwrap();

        // Create Agents
        let sender_c = AgentForCreate {
            project_id: pid,
            name: "Sender".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "Sender".into(),
        };
        let sender_id = AgentBmc::create(&ctx, &mm, sender_c).await.unwrap();

        let recv_c = AgentForCreate {
            project_id: pid,
            name: "Recv".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "Recv".into(),
        };
        let recv_id = AgentBmc::create(&ctx, &mm, recv_c).await.unwrap();

        // Send a message
        let msg_c = MessageForCreate {
            project_id: pid,
            sender_id,
            recipient_ids: vec![recv_id],
            cc_ids: None,
            bcc_ids: None,
            subject: "Outbox Check".into(),
            body_md: "Body".into(),
            thread_id: None,
            importance: None,
            ack_required: false,
        };
        MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

        // Call read_resource
        let uri = "agent-mail://outbox-test/outbox/Sender".to_string();
        let params = ReadResourceRequestParam { uri };

        // Use refactored impl to avoid constructing context
        let result = service.read_resource_impl(params).await.unwrap();

        let content = &result.contents[0];
        if let ResourceContents::TextResourceContents { text, .. } = content {
            assert!(text.contains("Outbox Check"));
            assert!(text.contains("Sender"));
        } else {
            panic!("Expected TextResourceContents");
        }
    }
    #[tokio::test]
    async fn test_record_tool_metric() {
        use lib_core::model::agent::{AgentBmc, AgentForCreate};
        use lib_core::model::project::ProjectBmc;
        use lib_core::model::tool_metric::ToolMetricBmc;
        use serde_json::json;

        let (mm, _temp) = create_test_mm().await;
        // Construct service
        let service = AgentMailService::new_with_mm(mm.clone(), false);
        let ctx = Ctx::root_ctx();

        // 1. Create Project and Agent
        let project_id = ProjectBmc::create(&ctx, &mm, "metric-test", "Metric Test")
            .await
            .unwrap();
        let agent_c = AgentForCreate {
            project_id,
            name: "MetricAgent".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "test".into(),
        };
        let agent_id = AgentBmc::create(&ctx, &mm, agent_c).await.unwrap();

        // 2. Simulate tool call arguments with context
        let args = Some(json!({
            "project_slug": "metric-test",
            "agent_name": "MetricAgent",
            "other": "stuff"
        }));

        let duration = std::time::Duration::from_millis(123);
        // We pass a dummy Ok result. Result type is Result<CallToolResult, McpError>.
        // CallToolResult is rmcp struct.
        use rmcp::model::CallToolResult;
        let result = Ok(CallToolResult {
            content: vec![],
            is_error: None,
            meta: None,
            structured_content: None,
        });

        // 3. Call record_tool_metric
        service
            .record_tool_metric("test_tool", &args, duration, &result)
            .await;

        // 4. Verify DB
        let metrics = ToolMetricBmc::list_recent(&ctx, &mm, Some(project_id), 10)
            .await
            .unwrap();
        assert_eq!(metrics.len(), 1);
        let m = &metrics[0];

        assert_eq!(m.tool_name, "test_tool");
        assert_eq!(m.duration_ms, 123);
        assert_eq!(m.status, "success");
        assert_eq!(m.project_id, Some(project_id));
        assert_eq!(m.agent_id, Some(agent_id));
    }

    // ==========================================================================
    // Conditional Build Slot Tool Registration (PORT-1.4)
    // ==========================================================================

    #[test]
    fn test_get_tool_schemas_worktrees_enabled_includes_build_slots() {
        let schemas = get_tool_schemas(true);
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();

        assert!(
            names.contains(&"acquire_build_slot"),
            "acquire_build_slot should be present when worktrees enabled"
        );
        assert!(
            names.contains(&"release_build_slot"),
            "release_build_slot should be present when worktrees enabled"
        );
        assert!(
            names.contains(&"renew_build_slot"),
            "renew_build_slot should be present when worktrees enabled"
        );
    }

    #[test]
    fn test_get_tool_schemas_worktrees_disabled_excludes_build_slots() {
        let schemas = get_tool_schemas(false);
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();

        assert!(
            !names.contains(&"acquire_build_slot"),
            "acquire_build_slot should NOT be present when worktrees disabled"
        );
        assert!(
            !names.contains(&"release_build_slot"),
            "release_build_slot should NOT be present when worktrees disabled"
        );
        assert!(
            !names.contains(&"renew_build_slot"),
            "renew_build_slot should NOT be present when worktrees disabled"
        );
    }

    #[test]
    fn test_get_tool_schemas_worktrees_disabled_includes_other_tools() {
        let schemas = get_tool_schemas(false);
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();

        // Core tools should still be present
        assert!(
            names.contains(&"ensure_project"),
            "ensure_project should be present"
        );
        assert!(
            names.contains(&"register_agent"),
            "register_agent should be present"
        );
        assert!(
            names.contains(&"send_message"),
            "send_message should be present"
        );
        assert!(
            names.contains(&"check_inbox"),
            "check_inbox should be present"
        );
    }

    #[test]
    fn test_build_slot_tools_constant() {
        // Ensure the constant matches expected tool names
        assert_eq!(BUILD_SLOT_TOOLS.len(), 3);
        assert!(BUILD_SLOT_TOOLS.contains(&"acquire_build_slot"));
        assert!(BUILD_SLOT_TOOLS.contains(&"release_build_slot"));
        assert!(BUILD_SLOT_TOOLS.contains(&"renew_build_slot"));
    }

    #[tokio::test]
    async fn test_service_worktrees_enabled_flag() {
        let (mm, _temp) = create_test_mm().await;

        // Test with worktrees enabled
        let service_enabled = AgentMailService::new_with_mm(mm.clone(), true);
        assert!(
            service_enabled.worktrees_enabled(),
            "Service should report worktrees enabled"
        );

        // Test with worktrees disabled
        let service_disabled = AgentMailService::new_with_mm(mm.clone(), false);
        assert!(
            !service_disabled.worktrees_enabled(),
            "Service should report worktrees disabled"
        );
    }
}
