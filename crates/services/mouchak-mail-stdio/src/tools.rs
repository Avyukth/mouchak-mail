//! MCP Tool implementations for Mouchak Mail
//!
//! This module defines all MCP tools that wrap the lib-core functionality.

use anyhow::Result;
use rmcp::{
    ErrorData as McpError,
    tool, tool_router,
    model::{
        CallToolResult, CallToolRequestParam, Content, ListToolsResult, PaginatedRequestParam,
        ListResourcesResult, ReadResourceResult, ReadResourceRequestParam, Resource, ResourceContents,
        RawResource,
    },
    handler::server::{ServerHandler, tool::ToolRouter, wrapper::Parameters},
    service::{RequestContext, RoleServer},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use mouchak_mail_core::{ctx::Ctx, model::{ModelManager, project::ProjectBmc, agent::AgentBmc, message::MessageBmc, file_reservation::FileReservationBmc, agent_capabilities::AgentCapabilityBmc}};

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

/// Get schema information for all tools
pub fn get_tool_schemas() -> Vec<ToolSchema> {
    vec![
        ToolSchema {
            name: "ensure_project".into(),
            description: "Ensure a project exists, creating it if necessary.".into(),
            parameters: vec![
                ParameterSchema { name: "slug".into(), param_type: "string".into(), required: true, description: "Project slug (URL-safe identifier)".into() },
                ParameterSchema { name: "human_key".into(), param_type: "string".into(), required: true, description: "Human-readable project name".into() },
            ],
        },
        ToolSchema {
            name: "register_agent".into(),
            description: "Register a new agent in a project.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "name".into(), param_type: "string".into(), required: true, description: "Agent name".into() },
                ParameterSchema { name: "program".into(), param_type: "string".into(), required: true, description: "Agent program name".into() },
                ParameterSchema { name: "model".into(), param_type: "string".into(), required: true, description: "AI model used".into() },
                ParameterSchema { name: "task_description".into(), param_type: "string".into(), required: true, description: "Agent's task description".into() },
            ],
        },
        ToolSchema {
            name: "send_message".into(),
            description: "Send a message from one agent to others.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "sender_name".into(), param_type: "string".into(), required: true, description: "Sender agent name".into() },
                ParameterSchema { name: "to".into(), param_type: "string".into(), required: true, description: "Recipient agent names (comma-separated)".into() },
                ParameterSchema { name: "cc".into(), param_type: "string".into(), required: false, description: "CC recipient agent names (comma-separated)".into() },
                ParameterSchema { name: "bcc".into(), param_type: "string".into(), required: false, description: "BCC recipient agent names (comma-separated)".into() },
                ParameterSchema { name: "subject".into(), param_type: "string".into(), required: true, description: "Message subject".into() },
                ParameterSchema { name: "body_md".into(), param_type: "string".into(), required: true, description: "Message body in markdown".into() },
                ParameterSchema { name: "importance".into(), param_type: "string".into(), required: false, description: "Message importance level".into() },
            ],
        },
        ToolSchema {
            name: "check_inbox".into(),
            description: "Check an agent's inbox for new messages.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "agent_name".into(), param_type: "string".into(), required: true, description: "Agent name".into() },
                ParameterSchema { name: "limit".into(), param_type: "integer".into(), required: false, description: "Maximum messages to return".into() },
            ],
        },
        ToolSchema {
            name: "reply_message".into(),
            description: "Reply to an existing message in a thread.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "sender_name".into(), param_type: "string".into(), required: true, description: "Sender agent name".into() },
                ParameterSchema { name: "message_id".into(), param_type: "integer".into(), required: true, description: "Message ID to reply to".into() },
                ParameterSchema { name: "body_md".into(), param_type: "string".into(), required: true, description: "Reply body in markdown".into() },
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
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
            ],
        },
        ToolSchema {
            name: "get_message".into(),
            description: "Get a specific message by ID.".into(),
            parameters: vec![
                ParameterSchema { name: "message_id".into(), param_type: "integer".into(), required: true, description: "Message ID".into() },
            ],
        },
        ToolSchema {
            name: "search_messages".into(),
            description: "Full-text search messages.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "query".into(), param_type: "string".into(), required: true, description: "Search query".into() },
                ParameterSchema { name: "limit".into(), param_type: "integer".into(), required: false, description: "Maximum results".into() },
            ],
        },
        ToolSchema {
            name: "reserve_file".into(),
            description: "Reserve a file path for exclusive editing.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "agent_name".into(), param_type: "string".into(), required: true, description: "Agent name".into() },
                ParameterSchema { name: "path_pattern".into(), param_type: "string".into(), required: true, description: "File path or glob pattern".into() },
                ParameterSchema { name: "reason".into(), param_type: "string".into(), required: false, description: "Reason for reservation".into() },
                ParameterSchema { name: "ttl_minutes".into(), param_type: "integer".into(), required: false, description: "Time-to-live in minutes".into() },
            ],
        },
        ToolSchema {
            name: "release_reservation".into(),
            description: "Release a file reservation by ID.".into(),
            parameters: vec![
                ParameterSchema { name: "reservation_id".into(), param_type: "integer".into(), required: true, description: "Reservation ID".into() },
            ],
        },
        ToolSchema {
            name: "list_file_reservations".into(),
            description: "List active file reservations in a project.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
            ],
        },
        ToolSchema {
            name: "force_release_reservation".into(),
            description: "Force release a file reservation (emergency override).".into(),
            parameters: vec![
                ParameterSchema { name: "reservation_id".into(), param_type: "integer".into(), required: true, description: "Reservation ID".into() },
            ],
        },
        ToolSchema {
            name: "renew_file_reservation".into(),
            description: "Extend the TTL of a file reservation.".into(),
            parameters: vec![
                ParameterSchema { name: "reservation_id".into(), param_type: "integer".into(), required: true, description: "Reservation ID".into() },
                ParameterSchema { name: "ttl_seconds".into(), param_type: "integer".into(), required: false, description: "New TTL in seconds".into() },
            ],
        },
        ToolSchema {
            name: "request_contact".into(),
            description: "Request to add another agent as a contact.".into(),
            parameters: vec![
                ParameterSchema { name: "from_project_slug".into(), param_type: "string".into(), required: true, description: "From project slug".into() },
                ParameterSchema { name: "from_agent_name".into(), param_type: "string".into(), required: true, description: "From agent name".into() },
                ParameterSchema { name: "to_project_slug".into(), param_type: "string".into(), required: true, description: "To project slug".into() },
                ParameterSchema { name: "to_agent_name".into(), param_type: "string".into(), required: true, description: "To agent name".into() },
                ParameterSchema { name: "reason".into(), param_type: "string".into(), required: true, description: "Reason for contact request".into() },
            ],
        },
        ToolSchema {
            name: "respond_contact".into(),
            description: "Accept or reject a contact request.".into(),
            parameters: vec![
                ParameterSchema { name: "link_id".into(), param_type: "integer".into(), required: true, description: "Agent link ID".into() },
                ParameterSchema { name: "accept".into(), param_type: "boolean".into(), required: true, description: "Accept or reject".into() },
            ],
        },
        ToolSchema {
            name: "list_contacts".into(),
            description: "List all contacts for an agent.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "agent_name".into(), param_type: "string".into(), required: true, description: "Agent name".into() },
            ],
        },
        ToolSchema {
            name: "acquire_build_slot".into(),
            description: "Acquire an exclusive build slot for CI/CD isolation.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "agent_name".into(), param_type: "string".into(), required: true, description: "Agent name".into() },
                ParameterSchema { name: "slot_name".into(), param_type: "string".into(), required: true, description: "Slot name".into() },
                ParameterSchema { name: "ttl_seconds".into(), param_type: "integer".into(), required: false, description: "TTL in seconds".into() },
            ],
        },
        ToolSchema {
            name: "release_build_slot".into(),
            description: "Release a held build slot.".into(),
            parameters: vec![
                ParameterSchema { name: "slot_id".into(), param_type: "integer".into(), required: true, description: "Slot ID".into() },
            ],
        },
        ToolSchema {
            name: "list_macros".into(),
            description: "List all available macros in a project.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
            ],
        },
        ToolSchema {
            name: "register_macro".into(),
            description: "Register a new macro definition.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "name".into(), param_type: "string".into(), required: true, description: "Macro name".into() },
                ParameterSchema { name: "description".into(), param_type: "string".into(), required: true, description: "Macro description".into() },
                ParameterSchema { name: "steps".into(), param_type: "array".into(), required: true, description: "Macro steps as JSON array".into() },
            ],
        },
        ToolSchema {
            name: "invoke_macro".into(),
            description: "Execute a pre-defined macro and get its steps.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "name".into(), param_type: "string".into(), required: true, description: "Macro name".into() },
            ],
        },
        ToolSchema {
            name: "ensure_product".into(),
            description: "Create or get a product for multi-repo coordination.".into(),
            parameters: vec![
                ParameterSchema { name: "product_uid".into(), param_type: "string".into(), required: true, description: "Product UID".into() },
                ParameterSchema { name: "name".into(), param_type: "string".into(), required: true, description: "Product name".into() },
            ],
        },
        ToolSchema {
            name: "link_project_to_product".into(),
            description: "Link a project to a product for unified messaging.".into(),
            parameters: vec![
                ParameterSchema { name: "product_uid".into(), param_type: "string".into(), required: true, description: "Product UID".into() },
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
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
                ParameterSchema { name: "product_uid".into(), param_type: "string".into(), required: true, description: "Product UID".into() },
                ParameterSchema { name: "limit".into(), param_type: "integer".into(), required: false, description: "Max messages per project".into() },
            ],
        },
        ToolSchema {
            name: "export_mailbox".into(),
            description: "Export a project's mailbox to HTML, JSON, or Markdown format.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "format".into(), param_type: "string".into(), required: false, description: "Export format: html, json, or markdown".into() },
            ],
        },
        ToolSchema {
            name: "get_project_info".into(),
            description: "Get detailed information about a project.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
            ],
        },
        ToolSchema {
            name: "get_agent_profile".into(),
            description: "Get detailed profile information for an agent.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "agent_name".into(), param_type: "string".into(), required: true, description: "Agent name".into() },
            ],
        },
        ToolSchema {
            name: "list_threads".into(),
            description: "List all conversation threads in a project.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "limit".into(), param_type: "integer".into(), required: false, description: "Maximum threads".into() },
            ],
        },
        ToolSchema {
            name: "summarize_thread".into(),
            description: "Get a summary of a conversation thread.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "thread_id".into(), param_type: "string".into(), required: true, description: "Thread ID".into() },
            ],
        },
        ToolSchema {
            name: "mark_message_read".into(),
            description: "Mark a message as read by a recipient.".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "agent_name".into(), param_type: "string".into(), required: true, description: "Agent name marking as read".into() },
                ParameterSchema { name: "message_id".into(), param_type: "integer".into(), required: true, description: "Message ID".into() },
            ],
        },
        ToolSchema {
            name: "acknowledge_message".into(),
            description: "Acknowledge a message (sets both read and acknowledged).".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "agent_name".into(), param_type: "string".into(), required: true, description: "Agent name acknowledging".into() },
                ParameterSchema { name: "message_id".into(), param_type: "integer".into(), required: true, description: "Message ID".into() },
            ],
        },
        ToolSchema {
            name: "set_contact_policy".into(),
            description: "Set an agent's contact policy (open, auto, contacts_only, block_all).".into(),
            parameters: vec![
                ParameterSchema { name: "project_slug".into(), param_type: "string".into(), required: true, description: "Project slug".into() },
                ParameterSchema { name: "agent_name".into(), param_type: "string".into(), required: true, description: "Agent name".into() },
                ParameterSchema { name: "contact_policy".into(), param_type: "string".into(), required: true, description: "Policy: open, auto, contacts_only, block_all".into() },
            ],
        },
        ToolSchema {
            name: "renew_build_slot".into(),
            description: "Extend the TTL of a build slot.".into(),
            parameters: vec![
                ParameterSchema { name: "slot_id".into(), param_type: "integer".into(), required: true, description: "Slot ID".into() },
                ParameterSchema { name: "ttl_seconds".into(), param_type: "integer".into(), required: false, description: "New TTL in seconds".into() },
            ],
        },
    ]
}

/// The main MCP service for Mouchak Mail
// Simple macro for early return locally
macro_rules! guard_unwrap {
    ($val:expr, $ret:expr) => {
        if let Some(v) = $val { v } else { $ret }
    };
}

#[derive(Clone)]
pub struct MouchakMailService {
    mm: Arc<ModelManager>,
    tool_router: ToolRouter<Self>,
}

impl MouchakMailService {
    pub async fn new() -> Result<Self> {
        let mm = Arc::new(ModelManager::new().await?);
        let tool_router = Self::tool_router();
        Ok(Self { mm, tool_router })
    }

    #[cfg(test)]
    pub fn new_with_mm(mm: Arc<ModelManager>) -> Self {
        let tool_router = Self::tool_router();
        Self { mm, tool_router }
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

        if uri.scheme() != "mouchak-mail" && uri.scheme() != "resource" {
            return Err(McpError::invalid_params(
                "URI scheme must be 'mouchak-mail' or 'resource'".to_string(),
                None,
            ));
        }

        // URI format: mouchak-mail://{project_slug}/{resource_type}/{optional_id}
        let project_slug = uri.host_str()
            .ok_or(McpError::invalid_params("URI missing host (project slug)".to_string(), None))?;
        
        // Path starts with /, so segments are after host
        let segments: Vec<&str> = uri.path_segments()
            .ok_or(McpError::invalid_params("Invalid URI path".to_string(), None))?
            .collect();

        if segments.is_empty() {
             return Err(McpError::invalid_params("URI path missing resource type".to_string(), None));
        }

        let resource_type = segments[0];
        let resource_id = segments.get(1).cloned();

        let ctx = self.ctx();
        let mm = &self.mm;

        // Resolve project ID
        let project = ProjectBmc::get_by_slug(&ctx, mm, project_slug).await
            .map_err(|_| McpError::invalid_params(format!("Project not found: {}", project_slug), None))?;
        let project_id = project.id;

        let content = match resource_type {
            "agents" => {
                let agents = AgentBmc::list_all_for_project(&ctx, mm, project_id).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&agents)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            },
            "file_reservations" => {
                let reservations = FileReservationBmc::list_active_for_project(&ctx, mm, project_id).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&reservations)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            },
            "inbox" => {
                let agent_name = resource_id.ok_or(McpError::invalid_params("Missing agent name".to_string(), None))?;
                let agent = AgentBmc::get_by_name(&ctx, mm, project_id, agent_name).await
                    .map_err(|_| McpError::invalid_params(format!("Agent not found: {}", agent_name), None))?;
                
                // Default limit 20
                let messages = MessageBmc::list_inbox_for_agent(&ctx, mm, project_id, agent.id, 20).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&messages)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            },
            "outbox" => {
                let agent_name = resource_id.ok_or(McpError::invalid_params("Missing agent name".to_string(), None))?;
                let agent = AgentBmc::get_by_name(&ctx, mm, project_id, agent_name).await
                    .map_err(|_| McpError::invalid_params(format!("Agent not found: {}", agent_name), None))?;
                
                let messages = MessageBmc::list_outbox_for_agent(&ctx, mm, project_id, agent.id, 20).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&messages)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            },
            "thread" => {
                let thread_id_str = resource_id.ok_or(McpError::invalid_params("Missing thread ID".to_string(), None))?;
                let messages = MessageBmc::list_by_thread(&ctx, mm, project_id, thread_id_str).await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                
                serde_json::to_string_pretty(&messages)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            },
            _ => return Err(McpError::invalid_params(format!("Unknown resource type: {}", resource_type), None)),
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
        use mouchak_mail_core::model::tool_metric::{ToolMetricBmc, ToolMetricForCreate};
        use mouchak_mail_core::model::project::ProjectBmc;
        use mouchak_mail_core::model::agent::AgentBmc;

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
        
        if let Some(slug) = project_slug {
             if let Ok(p) = ProjectBmc::get_by_slug(&ctx, &self.mm, &slug).await {
                 project_id = Some(p.id);
                 if let Some(name) = agent_name {
                     if let Ok(a) = AgentBmc::get_by_name(&ctx, &self.mm, p.id, &name).await {
                         agent_id = Some(a.id);
                     }
                 }
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

    pub fn extract_context(&self, args: &Option<serde_json::Value>) -> (Option<String>, Option<String>) {
        let val = guard_unwrap!(args.as_ref(), return (None, None));
        let obj = guard_unwrap!(val.as_object(), return (None, None));

        // Try to find project slug
        let project_slug = obj.get("project_slug")
            .or_else(|| obj.get("slug")) // For EnsureProjectParams
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Try to find agent name
        let agent_name = obj.get("agent_name")
            .or_else(|| obj.get("sender_name")) // For SendMessageParams
            .or_else(|| obj.get("name")) // For RegisterAgentParams
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
            
        (project_slug, agent_name)
    }
}

#[allow(clippy::manual_async_fn)]
impl ServerHandler for MouchakMailService {
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            Ok(ListToolsResult {
                tools: self.tool_router.list_all(),
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
            let tool_name = request.name.clone();
            let args = request.arguments.clone();
            
            let tool_context = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
            let result = self.tool_router.call(tool_context).await;
            
            let duration = start.elapsed();
            
            // Fire and forget metric recording (spawn generic task or just await since we are async)
            // Awaiting is safer to ensure it's recorded before response? 
            // Better to spawn to avoid latency, but for now await is fine as DB write is fast.
            let args_val = args.map(serde_json::Value::Object);
            self.record_tool_metric(&tool_name, &args_val, duration, &result).await;
            
            result
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async move {
            // List project-rooted resources for all projects
            let ctx = self.ctx();
            let projects = ProjectBmc::list_all(&ctx, &self.mm).await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let mut resources = Vec::new();

            for project in projects {
                let slug = &project.slug;
                
                // Agents list
                resources.push(Resource {
                    raw: RawResource {
                        uri: format!("mouchak-mail://{}/agents", slug),
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

                // File reservations
                resources.push(Resource {
                    raw: RawResource {
                        uri: format!("mouchak-mail://{}/file_reservations", slug),
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
            }

            Ok(ListResourcesResult {
                resources,
                next_cursor: None,
                meta: None,
            })
        }
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
// Tool Parameter Types
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnsureProjectParams {
    /// The project slug (URL-safe identifier)
    #[serde(default)]
    pub slug: Option<String>,
    /// Human-readable project name/key. If not provided, derived from slug (last path component).
    #[serde(default)]
    pub human_key: Option<String>,
}

impl EnsureProjectParams {
    pub fn effective_slug(&self) -> String {
        self.slug
            .clone()
            .or_else(|| self.human_key.clone())
            .unwrap_or_default()
    }

    pub fn effective_human_key(&self) -> String {
        self.human_key
            .clone()
            .or_else(|| {
                self.slug.as_ref().map(|s| {
                    std::path::Path::new(s)
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or(s)
                        .to_string()
                })
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterAgentParams {
    /// Project slug the agent belongs to
    pub project_slug: String,
    /// Agent's unique name within the project
    pub name: String,
    /// Agent's program identifier (e.g., "claude-code", "antigravity")
    pub program: String,
    /// Model being used (e.g., "claude-3-opus", "gemini-2.0-pro")
    pub model: String,
    /// Description of the agent's task/responsibilities
    pub task_description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendMessageParams {
    /// Project slug
    pub project_slug: String,
    /// Sender agent name
    pub sender_name: String,
    /// Recipient agent names (comma-separated for multiple)
    pub to: String,
    /// CC recipient agent names (comma-separated for multiple)
    pub cc: Option<String>,
    /// BCC recipient agent names (comma-separated for multiple)
    pub bcc: Option<String>,
    /// Message subject
    pub subject: String,
    /// Message body in markdown
    pub body_md: String,
    /// Message importance (low, normal, high, urgent)
    pub importance: Option<String>,
    /// Thread ID to continue existing conversation
    pub thread_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListInboxParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name to list inbox for
    pub agent_name: String,
    /// Maximum number of messages to return
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMessageParams {
    /// Message ID to retrieve
    pub message_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListProjectSiblingsParams {
    /// Project slug to find siblings for
    pub project_slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CommitArchiveParams {
    /// Project slug to archive
    pub project_slug: String,
    /// Commit message
    pub message: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WhoisParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name to look up
    pub agent_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchMessagesParams {
    /// Project slug
    pub project_slug: String,
    /// Search query (full-text search)
    pub query: String,
    /// Maximum results
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetThreadParams {
    /// Project slug
    pub project_slug: String,
    /// Thread ID
    pub thread_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListAgentsParams {
    /// Project slug
    pub project_slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileReservationParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name requesting reservations
    pub agent_name: String,
    /// File path pattern to reserve
    pub path_pattern: String,
    /// Whether this is an exclusive reservation
    pub exclusive: Option<bool>,
    /// Reason for the reservation
    pub reason: Option<String>,
    /// TTL in seconds (default 3600)
    pub ttl_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListReservationsParams {
    /// Project slug
    pub project_slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReleaseReservationParams {
    /// Reservation ID to release
    pub reservation_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ForceReleaseReservationParams {
    /// Reservation ID to force release (for emergencies)
    pub reservation_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenewFileReservationParams {
    /// Reservation ID to renew
    pub reservation_id: i64,
    /// New TTL in seconds (default 3600)
    pub ttl_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReplyMessageParams {
    /// Project slug
    pub project_slug: String,
    /// Sender agent name
    pub sender_name: String,
    /// Message ID to reply to
    pub message_id: i64,
    /// Reply body in markdown
    pub body_md: String,
    /// Message importance (optional)
    pub importance: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MarkMessageReadParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name marking as read
    pub agent_name: String,
    /// Message ID to mark as read
    pub message_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AcknowledgeMessageParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name acknowledging
    pub agent_name: String,
    /// Message ID to acknowledge
    pub message_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateAgentIdentityParams {
    /// Project slug
    pub project_slug: String,
    /// Optional hint for name generation
    #[allow(dead_code)]
    pub hint: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateAgentProfileParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name to update
    pub agent_name: String,
    /// New task description (optional)
    pub task_description: Option<String>,
    /// New attachments policy (optional)
    pub attachments_policy: Option<String>,
    /// New contact policy (optional)
    pub contact_policy: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetProjectInfoParams {
    /// Project slug
    pub project_slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetAgentProfileParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name
    pub agent_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListThreadsParams {
    /// Project slug
    pub project_slug: String,
    /// Maximum threads to return
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RequestContactParams {
    /// From project slug
    pub from_project_slug: String,
    /// From agent name
    pub from_agent_name: String,
    /// To project slug
    pub to_project_slug: String,
    /// To agent name
    pub to_agent_name: String,
    /// Reason for contact request
    pub reason: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RespondContactParams {
    /// Agent link ID
    pub link_id: i64,
    /// Accept (true) or reject (false)
    pub accept: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListContactsParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name
    pub agent_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetContactPolicyParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name
    pub agent_name: String,
    /// Contact policy: auto, manual, or deny
    pub contact_policy: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AcquireBuildSlotParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name
    pub agent_name: String,
    /// Slot name
    pub slot_name: String,
    /// TTL in seconds (default 1800)
    pub ttl_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenewBuildSlotParams {
    /// Slot ID to renew
    pub slot_id: i64,
    /// TTL in seconds (default 1800)
    pub ttl_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReleaseBuildSlotParams {
    /// Slot ID to release
    pub slot_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendOverseerMessageParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name receiving the message
    pub agent_name: String,
    /// Message subject
    pub subject: String,
    /// Message body in markdown
    pub body_md: String,
    /// Message importance (optional)
    pub importance: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListMacrosParams {
    /// Project slug
    pub project_slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterMacroParams {
    /// Project slug
    pub project_slug: String,
    /// Macro name
    pub name: String,
    /// Macro description
    pub description: String,
    /// Macro steps as JSON array
    pub steps: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UnregisterMacroParams {
    /// Project slug
    pub project_slug: String,
    /// Macro name to remove
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InvokeMacroParams {
    /// Project slug
    pub project_slug: String,
    /// Macro name to invoke
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SummarizeThreadParams {
    /// Project slug
    pub project_slug: String,
    /// Thread ID to summarize
    pub thread_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnsureProductParams {
    /// Product UID (unique identifier)
    pub product_uid: String,
    /// Human-readable product name
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LinkProjectToProductParams {
    /// Product UID
    pub product_uid: String,
    /// Project slug to link
    pub project_slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UnlinkProjectFromProductParams {
    /// Product UID
    pub product_uid: String,
    /// Project slug to unlink
    pub project_slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProductInboxParams {
    /// Product UID
    pub product_uid: String,
    /// Maximum messages per project
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportMailboxParams {
    /// Project slug to export
    pub project_slug: String,
    /// Export format: html, json, or markdown
    pub format: Option<String>,
    /// Include attachments in export
    #[allow(dead_code)]
    pub include_attachments: Option<bool>,
}

// ============================================================================
// Tool Implementations
// ============================================================================

#[tool_router]
impl MouchakMailService {
    /// Ensure a project exists, creating it if necessary
    #[tool(description = "Create or get a project. Projects are workspaces that contain agents and their messages.")]
    async fn ensure_project(
        &self,
        params: Parameters<EnsureProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;
        let slug = p.effective_slug();
        let human_key = p.effective_human_key();

        match ProjectBmc::get_by_identifier(&ctx, &self.mm, &slug).await {
            Ok(project) => {
                let msg = format!(
                    "Project '{}' already exists (id: {}, human_key: {})",
                    project.slug, project.id, project.human_key
                );
                Ok(CallToolResult::success(vec![Content::text(msg)]))
            }
            Err(_) => {
                let id = ProjectBmc::create(&ctx, &self.mm, &slug, &human_key).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                let msg = format!(
                    "Created project '{}' with id {} and human_key '{}'",
                    slug, id, human_key
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
        use mouchak_mail_core::model::agent::{AgentBmc, AgentForCreate};
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        // Get project
        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

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

                let id = AgentBmc::create(&ctx, &self.mm, agent_c).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                // Auto-grant default capabilities for MCP tool usage
                AgentCapabilityBmc::grant_defaults(&ctx, &self.mm, id).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                let msg = format!("Registered agent '{}' with id {} (granted default capabilities)", p.name, id);
                Ok(CallToolResult::success(vec![Content::text(msg)]))
            }
        }
    }

    /// Send a message to one or more agents
    #[tool(description = "Send a message from one agent to another. Creates a new thread or continues an existing one.")]
    async fn send_message(
        &self,
        params: Parameters<SendMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        // Get project and sender
        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let sender = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.sender_name).await
            .map_err(|e| McpError::invalid_params(format!("Sender not found: {}", e), None))?;

        if !AgentCapabilityBmc::check(&ctx, &self.mm, sender.id, "send_message").await
            .map_err(|e| McpError::internal_error(e.to_string(), None))? {
            return Err(McpError::invalid_params(format!("Agent '{}' does not have 'send_message' capability", p.sender_name), None));
        }

        // Helper to resolve list of names to IDs
        async fn resolve_agents(ctx: &mouchak_mail_core::Ctx, mm: &mouchak_mail_core::ModelManager, project_id: i64, names_str: &str) -> Result<Vec<i64>, McpError> {
             use mouchak_mail_core::model::agent::AgentBmc;
             let names: Vec<&str> = names_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
             let mut ids = Vec::new();
             for name in names {
                 let agent = AgentBmc::get_by_name(ctx, mm, project_id, name).await
                     .map_err(|e| McpError::invalid_params(format!("Agent '{}' not found: {}", name, e), None))?;
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
        };

        let msg_id = MessageBmc::create(&ctx, &self.mm, msg_c).await
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
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::message::MessageBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        if !AgentCapabilityBmc::check(&ctx, &self.mm, agent.id, "fetch_inbox").await
            .map_err(|e| McpError::internal_error(e.to_string(), None))? {
            return Err(McpError::invalid_params(format!("Agent '{}' does not have 'fetch_inbox' capability", p.agent_name), None));
        }

        let messages = MessageBmc::list_inbox_for_agent(&ctx, &self.mm, project.id, agent.id, p.limit.unwrap_or(50)).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Inbox for '{}' ({} messages):\n\n", p.agent_name, messages.len());
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
        use mouchak_mail_core::model::message::MessageBmc;

        let ctx = self.ctx();
        let p = params.0;

        let message = MessageBmc::get(&ctx, &self.mm, p.message_id).await
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
    #[tool(description = "Get information about an agent including their program, model, and task description.")]
    async fn whois(
        &self,
        params: Parameters<WhoisParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
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
        use mouchak_mail_core::model::message::MessageBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let messages = MessageBmc::search(&ctx, &self.mm, project.id, &p.query, p.limit.unwrap_or(20)).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Search results for '{}' ({} matches):\n\n", p.query, messages.len());
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
        use mouchak_mail_core::model::message::MessageBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let messages = MessageBmc::list_by_thread(&ctx, &self.mm, project.id, &p.thread_id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Thread '{}' ({} messages):\n\n", p.thread_id, messages.len());
        for m in &messages {
            output.push_str(&format!(
                "---\n[{}] From: {} | {}\nSubject: {}\n\n{}\n\n",
                m.id, m.sender_name, m.created_ts, m.subject, m.body_md
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// List all projects
    #[tool(description = "List all available projects in the system.")]
    async fn list_projects(&self) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();

        let projects = ProjectBmc::list_all(&ctx, &self.mm).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Projects ({}):\n\n", projects.len());
        for p in &projects {
            output.push_str(&format!("- {} (slug: {}, created: {})\n", p.human_key, p.slug, p.created_at));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// List all agents in a project
    #[tool(description = "List all agents registered in a project.")]
    async fn list_agents(
        &self,
        params: Parameters<ListAgentsParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agents = AgentBmc::list_all_for_project(&ctx, &self.mm, project.id).await
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
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        if !AgentCapabilityBmc::check(&ctx, &self.mm, agent.id, "file_reservation_paths").await
            .map_err(|e| McpError::internal_error(e.to_string(), None))? {
            return Err(McpError::invalid_params(format!("Agent '{}' does not have 'file_reservation_paths' capability", p.agent_name), None));
        }

        let ttl = p.ttl_seconds.unwrap_or(3600);
        let expires_ts = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl);

        let res_c = FileReservationForCreate {
            project_id: project.id,
            agent_id: agent.id,
            path_pattern: p.path_pattern.clone(),
            exclusive: p.exclusive.unwrap_or(true),
            reason: p.reason.unwrap_or_else(|| "Reserved via MCP".to_string()),
            expires_ts,
        };

        let id = FileReservationBmc::create(&ctx, &self.mm, res_c).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!(
            "Reserved '{}' for agent '{}' (reservation id: {}, expires: {})",
            p.path_pattern, p.agent_name, id, expires_ts
        );
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// List active file reservations
    #[tool(description = "List all active file reservations in a project.")]
    async fn list_reservations(
        &self,
        params: Parameters<ListReservationsParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::file_reservation::FileReservationBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let reservations = FileReservationBmc::list_active_for_project(&ctx, &self.mm, project.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Active reservations in '{}' ({}):\n\n", p.project_slug, reservations.len());
        for r in &reservations {
            output.push_str(&format!(
                "- [{}] {} (agent_id: {}, exclusive: {}, expires: {})\n",
                r.id, r.path_pattern, r.agent_id, r.exclusive, r.expires_ts
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Release a file reservation
    #[tool(description = "Release a file reservation by ID.")]
    async fn release_reservation(
        &self,
        params: Parameters<ReleaseReservationParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::file_reservation::FileReservationBmc;

        let ctx = self.ctx();
        let p = params.0;

        FileReservationBmc::release(&ctx, &self.mm, p.reservation_id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Released reservation {}", p.reservation_id);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Force release a file reservation (emergency override)
    #[tool(description = "Force release a file reservation by ID. Use for emergencies when an agent has abandoned work.")]
    async fn force_release_reservation(
        &self,
        params: Parameters<ForceReleaseReservationParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::file_reservation::FileReservationBmc;

        let ctx = self.ctx();
        let p = params.0;

        FileReservationBmc::force_release(&ctx, &self.mm, p.reservation_id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Force released reservation {}", p.reservation_id);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Renew a file reservation TTL
    #[tool(description = "Extend the TTL of a file reservation. Keeps the lock active for more work.")]
    async fn renew_file_reservation(
        &self,
        params: Parameters<RenewFileReservationParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::file_reservation::FileReservationBmc;

        let ctx = self.ctx();
        let p = params.0;

        let ttl = p.ttl_seconds.unwrap_or(3600);
        let new_expires = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl);

        FileReservationBmc::renew(&ctx, &self.mm, p.reservation_id, new_expires).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Renewed reservation {} until {}", p.reservation_id, new_expires);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Reply to a message
    #[tool(description = "Reply to an existing message in a thread.")]
    async fn reply_message(
        &self,
        params: Parameters<ReplyMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let sender = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.sender_name).await
            .map_err(|e| McpError::invalid_params(format!("Sender not found: {}", e), None))?;

        if !AgentCapabilityBmc::check(&ctx, &self.mm, sender.id, "send_message").await
            .map_err(|e| McpError::internal_error(e.to_string(), None))? {
            return Err(McpError::invalid_params(format!("Agent '{}' does not have 'send_message' capability", p.sender_name), None));
        }

        let original_msg = MessageBmc::get(&ctx, &self.mm, p.message_id).await
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
        };

        let msg_id = MessageBmc::create(&ctx, &self.mm, msg_c).await
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
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::message::MessageBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        MessageBmc::mark_read(&ctx, &self.mm, p.message_id, agent.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Message {} marked as read by '{}'", p.message_id, p.agent_name);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Acknowledge a message
    #[tool(description = "Acknowledge receipt of a message requiring acknowledgment.")]
    async fn acknowledge_message(
        &self,
        params: Parameters<AcknowledgeMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::message::MessageBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        if !AgentCapabilityBmc::check(&ctx, &self.mm, agent.id, "acknowledge_message").await
            .map_err(|e| McpError::internal_error(e.to_string(), None))? {
            return Err(McpError::invalid_params(format!("Agent '{}' does not have 'acknowledge_message' capability", p.agent_name), None));
        }

        MessageBmc::acknowledge(&ctx, &self.mm, p.message_id, agent.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Message {} acknowledged by '{}'", p.message_id, p.agent_name);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Generate agent identity names
    #[tool(description = "Generate memorable agent names with collision detection.")]
    async fn create_agent_identity(
        &self,
        params: Parameters<CreateAgentIdentityParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::project::ProjectBmc;
        use std::collections::HashSet;

        const ADJECTIVES: &[&str] = &[
            "Blue", "Green", "Red", "Golden", "Silver", "Crystal", "Dark", "Bright",
            "Swift", "Calm", "Bold", "Wise", "Noble", "Grand", "Mystic", "Ancient",
        ];
        const NOUNS: &[&str] = &[
            "Mountain", "Castle", "River", "Forest", "Valley", "Harbor", "Tower", "Bridge",
            "Falcon", "Phoenix", "Dragon", "Wolf", "Eagle", "Lion", "Hawk", "Owl",
        ];

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let existing_agents = AgentBmc::list_all_for_project(&ctx, &self.mm, project.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let existing_names: HashSet<String> = existing_agents.iter().map(|a| a.name.clone()).collect();

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

        let suggested = alternatives.first().cloned().unwrap_or_else(|| "Agent1".to_string());
        let output = format!("Suggested: {}\nAlternatives: {}", suggested, alternatives.join(", "));
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Update agent profile
    #[tool(description = "Update an agent's profile settings.")]
    async fn update_agent_profile(
        &self,
        params: Parameters<UpdateAgentProfileParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::{AgentBmc, AgentProfileUpdate};
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let update = AgentProfileUpdate {
            task_description: p.task_description,
            attachments_policy: p.attachments_policy,
            contact_policy: p.contact_policy,
        };

        AgentBmc::update_profile(&ctx, &self.mm, agent.id, update).await
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
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agents = AgentBmc::list_all_for_project(&ctx, &self.mm, project.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let message_count = ProjectBmc::count_messages(&ctx, &self.mm, project.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let output = format!(
            "Project: {} ({})\nID: {}\nAgents: {}\nMessages: {}\nCreated: {}",
            project.human_key, project.slug, project.id, agents.len(), message_count, project.created_at
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Get agent profile
    #[tool(description = "Get detailed profile information for an agent.")]
    async fn get_agent_profile(
        &self,
        params: Parameters<GetAgentProfileParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::file_reservation::FileReservationBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let sent_count = AgentBmc::count_messages_sent(&ctx, &self.mm, agent.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let received_count = AgentBmc::count_messages_received(&ctx, &self.mm, agent.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let reservations = FileReservationBmc::list_active_for_project(&ctx, &self.mm, project.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let active_reservations = reservations.iter().filter(|r| r.agent_id == agent.id).count();

        let output = format!(
            "Agent: {}\nID: {}\nProgram: {}\nModel: {}\nTask: {}\nContact Policy: {}\nAttachments Policy: {}\nMessages Sent: {}\nMessages Received: {}\nActive Reservations: {}\nInception: {}\nLast Active: {}",
            agent.name, agent.id, agent.program, agent.model, agent.task_description,
            agent.contact_policy, agent.attachments_policy,
            sent_count, received_count, active_reservations,
            agent.inception_ts, agent.last_active_ts
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// List threads
    #[tool(description = "List all conversation threads in a project.")]
    async fn list_threads(
        &self,
        params: Parameters<ListThreadsParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::message::MessageBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let threads = MessageBmc::list_threads(&ctx, &self.mm, project.id, p.limit.unwrap_or(50)).await
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
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::agent_link::{AgentLinkBmc, AgentLinkForCreate};
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let from_project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.from_project_slug).await
            .map_err(|e| McpError::invalid_params(format!("From project not found: {}", e), None))?;
        let from_agent = AgentBmc::get_by_name(&ctx, &self.mm, from_project.id, &p.from_agent_name).await
            .map_err(|e| McpError::invalid_params(format!("From agent not found: {}", e), None))?;

        let to_project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.to_project_slug).await
            .map_err(|e| McpError::invalid_params(format!("To project not found: {}", e), None))?;
        let to_agent = AgentBmc::get_by_name(&ctx, &self.mm, to_project.id, &p.to_agent_name).await
            .map_err(|e| McpError::invalid_params(format!("To agent not found: {}", e), None))?;

        let link_c = AgentLinkForCreate {
            a_project_id: from_project.id,
            a_agent_id: from_agent.id,
            b_project_id: to_project.id,
            b_agent_id: to_agent.id,
            reason: p.reason,
        };

        let link_id = AgentLinkBmc::request_contact(&ctx, &self.mm, link_c).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Contact request sent (link_id: {}, status: pending)", link_id);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Respond to contact request
    #[tool(description = "Accept or reject a contact request.")]
    async fn respond_contact(
        &self,
        params: Parameters<RespondContactParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent_link::AgentLinkBmc;

        let ctx = self.ctx();
        let p = params.0;

        AgentLinkBmc::respond_contact(&ctx, &self.mm, p.link_id, p.accept).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let status = if p.accept { "accepted" } else { "rejected" };
        let msg = format!("Contact request {} {}", p.link_id, status);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// List contacts
    #[tool(description = "List all contacts for an agent.")]
    async fn list_contacts(
        &self,
        params: Parameters<ListContactsParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::agent_link::AgentLinkBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let links = AgentLinkBmc::list_contacts(&ctx, &self.mm, project.id, agent.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Contacts for '{}' ({}):\n\n", p.agent_name, links.len());
        for link in &links {
            let (other_project_id, other_agent_id) = if link.a_agent_id == agent.id {
                (link.b_project_id, link.b_agent_id)
            } else {
                (link.a_project_id, link.a_agent_id)
            };
            output.push_str(&format!(
                "- [{}] project:{} agent:{} (status: {}, reason: {})\n",
                link.id, other_project_id, other_agent_id, link.status, link.reason
            ));
        }
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Set contact policy
    #[tool(description = "Set an agent's contact acceptance policy (auto, manual, deny).")]
    async fn set_contact_policy(
        &self,
        params: Parameters<SetContactPolicyParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::{AgentBmc, AgentProfileUpdate};
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let update = AgentProfileUpdate {
            task_description: None,
            attachments_policy: None,
            contact_policy: Some(p.contact_policy.clone()),
        };

        AgentBmc::update_profile(&ctx, &self.mm, agent.id, update).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Contact policy for '{}' set to '{}'", p.agent_name, p.contact_policy);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Acquire build slot
    #[tool(description = "Acquire an exclusive build slot for CI/CD isolation.")]
    async fn acquire_build_slot(
        &self,
        params: Parameters<AcquireBuildSlotParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::build_slot::{BuildSlotBmc, BuildSlotForCreate};
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let ttl = p.ttl_seconds.unwrap_or(1800);
        let slot_c = BuildSlotForCreate {
            project_id: project.id,
            agent_id: agent.id,
            slot_name: p.slot_name.clone(),
            ttl_seconds: ttl,
        };

        let slot_id = BuildSlotBmc::acquire(&ctx, &self.mm, slot_c).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let expires = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl);
        let msg = format!("Acquired build slot '{}' (id: {}, expires: {})", p.slot_name, slot_id, expires);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Renew build slot
    #[tool(description = "Extend TTL on an active build slot.")]
    async fn renew_build_slot(
        &self,
        params: Parameters<RenewBuildSlotParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::build_slot::BuildSlotBmc;

        let ctx = self.ctx();
        let p = params.0;

        let ttl = p.ttl_seconds.unwrap_or(1800);
        let new_expires = BuildSlotBmc::renew(&ctx, &self.mm, p.slot_id, ttl).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Renewed build slot {} (new expires: {})", p.slot_id, new_expires);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Release build slot
    #[tool(description = "Release a held build slot.")]
    async fn release_build_slot(
        &self,
        params: Parameters<ReleaseBuildSlotParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::build_slot::BuildSlotBmc;

        let ctx = self.ctx();
        let p = params.0;

        BuildSlotBmc::release(&ctx, &self.mm, p.slot_id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Released build slot {}", p.slot_id);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Send overseer message
    #[tool(description = "Send a guidance message from the human overseer to an agent.")]
    async fn send_overseer_message(
        &self,
        params: Parameters<SendOverseerMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::overseer_message::{OverseerMessageBmc, OverseerMessageForCreate};
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agent = AgentBmc::get_by_name(&ctx, &self.mm, project.id, &p.agent_name).await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

        let msg_c = OverseerMessageForCreate {
            project_id: project.id,
            sender_id: agent.id,
            subject: p.subject.clone(),
            body_md: p.body_md,
            importance: p.importance.unwrap_or_else(|| "normal".to_string()),
        };

        let message_id = OverseerMessageBmc::create(&ctx, &self.mm, msg_c).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Overseer message sent (id: {}) to '{}'", message_id, p.agent_name);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// List macros
    #[tool(description = "List all available macros in a project.")]
    async fn list_macros(
        &self,
        params: Parameters<ListMacrosParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::macro_def::MacroDefBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let macros = MacroDefBmc::list(&ctx, &self.mm, project.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Macros in '{}' ({}):\n\n", p.project_slug, macros.len());
        for m in &macros {
            output.push_str(&format!("- {} ({} steps): {}\n", m.name, m.steps.len(), m.description));
        }
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Register macro
    #[tool(description = "Register a new macro definition.")]
    async fn register_macro(
        &self,
        params: Parameters<RegisterMacroParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::macro_def::{MacroDefBmc, MacroDefForCreate};
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let macro_c = MacroDefForCreate {
            project_id: project.id,
            name: p.name.clone(),
            description: p.description,
            steps: p.steps,
        };

        let macro_id = MacroDefBmc::create(&ctx, &self.mm, macro_c).await
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
        use mouchak_mail_core::model::macro_def::MacroDefBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let deleted = MacroDefBmc::delete(&ctx, &self.mm, project.id, &p.name).await
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
        use mouchak_mail_core::model::macro_def::MacroDefBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let macro_def = MacroDefBmc::get_by_name(&ctx, &self.mm, project.id, &p.name).await
            .map_err(|e| McpError::invalid_params(format!("Macro not found: {}", e), None))?;

        let steps_json = serde_json::to_string_pretty(&macro_def.steps)
            .unwrap_or_else(|_| "[]".to_string());
        let output = format!(
            "Macro '{}' ({} steps)\nDescription: {}\n\nSteps:\n{}",
            macro_def.name, macro_def.steps.len(), macro_def.description, steps_json
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Summarize thread
    #[tool(description = "Get a summary of a conversation thread.")]
    async fn summarize_thread(
        &self,
        params: Parameters<SummarizeThreadParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::message::MessageBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let messages = MessageBmc::list_by_thread(&ctx, &self.mm, project.id, &p.thread_id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut participants: Vec<String> = messages.iter().map(|m| m.sender_name.clone()).collect();
        participants.sort();
        participants.dedup();

        let subject = messages.first().map(|m| m.subject.clone()).unwrap_or_default();
        let last_snippet = messages.last()
            .map(|m| m.body_md.chars().take(100).collect::<String>())
            .unwrap_or_default();

        let output = format!(
            "Thread: {}\nSubject: {}\nMessages: {}\nParticipants: {}\nLatest: {}...",
            p.thread_id, subject, messages.len(), participants.join(", "), last_snippet
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Ensure product exists
    #[tool(description = "Create or get a product for multi-repo coordination.")]
    async fn ensure_product(
        &self,
        params: Parameters<EnsureProductParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::product::ProductBmc;

        let ctx = self.ctx();
        let p = params.0;

        let product = ProductBmc::ensure(&ctx, &self.mm, &p.product_uid, &p.name).await
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
        use mouchak_mail_core::model::product::ProductBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let product = ProductBmc::get_by_uid(&ctx, &self.mm, &p.product_uid).await
            .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let link_id = ProductBmc::link_project(&ctx, &self.mm, product.id, project.id).await
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
        use mouchak_mail_core::model::product::ProductBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let product = ProductBmc::get_by_uid(&ctx, &self.mm, &p.product_uid).await
            .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let unlinked = ProductBmc::unlink_project(&ctx, &self.mm, product.id, project.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = if unlinked {
            format!("Unlinked project '{}' from product '{}'", p.project_slug, p.product_uid)
        } else {
            format!("Project '{}' was not linked to product '{}'", p.project_slug, p.product_uid)
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// List all products
    #[tool(description = "List all products and their linked projects.")]
    async fn list_products(&self) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::product::ProductBmc;

        let ctx = self.ctx();

        let products = ProductBmc::list_all(&ctx, &self.mm).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Products ({}):\n\n", products.len());
        for p in &products {
            output.push_str(&format!(
                "- {} (uid: {}, {} projects)\n  Projects: {:?}\n",
                p.name, p.product_uid, p.project_ids.len(), p.project_ids
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
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let siblings = ProjectBmc::list_siblings(&ctx, &self.mm, project.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut output = format!("Sibling Projects for '{}' ({}):\n\n", project.human_key, siblings.len());
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
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let oid = ProjectBmc::sync_to_archive(&ctx, &self.mm, project.id, &p.message).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let msg = format!("Archived project '{}' to git. Commit ID: {}", project.slug, oid);
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    /// Product-wide inbox
    #[tool(description = "Get aggregated inbox across all projects in a product.")]
    async fn product_inbox(
        &self,
        params: Parameters<ProductInboxParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::message::MessageBmc;
        use mouchak_mail_core::model::product::ProductBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;

        let product = ProductBmc::get_by_uid(&ctx, &self.mm, &p.product_uid).await
            .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;

        let project_ids = ProductBmc::get_linked_projects(&ctx, &self.mm, product.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let limit = p.limit.unwrap_or(10);
        let mut output = format!("Product Inbox for '{}' ({} projects):\n\n", product.name, project_ids.len());

        for project_id in project_ids {
            let project = ProjectBmc::get(&ctx, &self.mm, project_id).await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let messages = MessageBmc::list_recent(&ctx, &self.mm, project_id, limit).await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            output.push_str(&format!("\n## Project: {} ({})\n", project.human_key, project.slug));
            for m in &messages {
                output.push_str(&format!(
                    "  - [{}] {} (from: {}, {})\n",
                    m.id, m.subject, m.sender_name, m.created_ts
                ));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    /// Export mailbox to static bundle
    #[tool(description = "Export a project's mailbox to HTML, JSON, or Markdown format.")]
    async fn export_mailbox(
        &self,
        params: Parameters<ExportMailboxParams>,
    ) -> Result<CallToolResult, McpError> {
        use mouchak_mail_core::model::agent::AgentBmc;
        use mouchak_mail_core::model::message::MessageBmc;
        use mouchak_mail_core::model::project::ProjectBmc;

        let ctx = self.ctx();
        let p = params.0;
        let format = p.format.unwrap_or_else(|| "markdown".to_string());

        let project = ProjectBmc::get_by_identifier(&ctx, &self.mm, &p.project_slug).await
            .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

        let agents = AgentBmc::list_all_for_project(&ctx, &self.mm, project.id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let messages = MessageBmc::list_recent(&ctx, &self.mm, project.id, 1000).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let threads = MessageBmc::list_threads(&ctx, &self.mm, project.id, 100).await
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
                let mut html = format!(r#"<!DOCTYPE html>
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
"#, project.human_key, project.human_key, project.slug, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));

                html.push_str("<h2>Agents</h2><div>");
                for a in &agents {
                    html.push_str(&format!(r#"<span class="agent">{} ({})</span>"#, a.name, a.program));
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
                    project.human_key, project.slug, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
                );

                md.push_str("## Agents\n\n");
                for a in &agents {
                    md.push_str(&format!("- **{}** ({}) - {}\n", a.name, a.program, a.task_description));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use mouchak_mail_common::config::AppConfig;
    use mouchak_mail_core::model::agent::{AgentForCreate, AgentBmc};
    use mouchak_mail_core::model::project::ProjectBmc;
    use mouchak_mail_core::model::agent_capabilities::{AgentCapabilityBmc, AgentCapabilityForCreate};
    use tempfile::TempDir;
    use std::sync::Arc;

    async fn create_test_mm() -> (Arc<ModelManager>, TempDir) {
        use libsql::Builder;
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test_middleware.db");
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
        let schema4 = include_str!("../../../../migrations/004_add_recipient_type.sql");
        conn.execute_batch(schema4).await.unwrap();

        let app_config = Arc::new(AppConfig::default());
        let mm = ModelManager::new_for_test(conn, archive_root, app_config);
        (Arc::new(mm), temp_dir)
    }

    #[tokio::test]
    async fn test_middleware_enforcement() {
        let (mm, _temp) = create_test_mm().await;
        // Construct service
        let service = MouchakMailService::new_with_mm(mm.clone());
        let ctx = Ctx::root_ctx();

        // Create project/agent
        let project_id = ProjectBmc::create(&ctx, &mm, "mw-test", "/mw/test").await.unwrap();
        
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
        };
        
        // We invoke the handler directly
        let result = service.send_message(Parameters(params)).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        // Check for specific permission denied message
        assert!(err.message.contains("does not have 'send_message' capability"));

        // 2. Grant capability
        let cap_c = AgentCapabilityForCreate {
            agent_id: sender_id,
            capability: "send_message".into(),
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
        };
        let result = service.send_message(Parameters(params2)).await;
        assert!(result.is_ok());
    }


    #[tokio::test]
    async fn test_list_project_siblings_tool() {
        use mouchak_mail_core::model::product::ProductBmc;
        
        let (mm, _temp) = create_test_mm().await;
        let service = MouchakMailService::new_with_mm(mm.clone());
        let ctx = Ctx::root_ctx();

        // 1. Create Projects
        let id_a = ProjectBmc::create(&ctx, &mm, "proj-a", "Project A").await.unwrap();
        let id_b = ProjectBmc::create(&ctx, &mm, "proj-b", "Project B").await.unwrap();

        // 2. Link to Product
        let product = ProductBmc::ensure(&ctx, &mm, "prod-p", "Product P").await.unwrap();
        ProductBmc::link_project(&ctx, &mm, product.id, id_a).await.unwrap();
        ProductBmc::link_project(&ctx, &mm, product.id, id_b).await.unwrap();

        // 3. Call tool
        let params = ListProjectSiblingsParams {
            project_slug: "proj-a".to_string(),
        };
        let result = service.list_project_siblings(Parameters(params)).await.unwrap();
        
        // 4. Verify output
        let content = &result.content[0];
        // 4. Verify output via Debug (since Content structure is complex)
        let text = format!("{:?}", content);
        assert!(text.contains("Sibling Projects for 'Project A'"));
        assert!(text.contains("Project B"));
    }
    #[tokio::test]
    async fn test_send_message_cc_bcc() {
        use mouchak_mail_core::model::message::MessageBmc;
    
        let (mm, _temp) = create_test_mm().await;
        // Construct service
        let service = MouchakMailService::new_with_mm(mm.clone());
        let ctx = Ctx::root_ctx();

        // Create project
        let pid = ProjectBmc::create(&ctx, &mm, "cc-test", "CC Test").await.unwrap();

        // Create Agents
        let sender_c = AgentForCreate { project_id: pid, name: "Sender".into(), program: "test".into(), model: "test".into(), task_description: "Sender".into() };
        let sender_id = AgentBmc::create(&ctx, &mm, sender_c).await.unwrap();
        // Recipient
        let recv_c = AgentForCreate { project_id: pid, name: "Recv".into(), program: "test".into(), model: "test".into(), task_description: "Recv".into() };
        let recv_id = AgentBmc::create(&ctx, &mm, recv_c).await.unwrap();
        // CC
        let cc_c = AgentForCreate { project_id: pid, name: "CCAgent".into(), program: "test".into(), model: "test".into(), task_description: "CC".into() };
        let cc_id = AgentBmc::create(&ctx, &mm, cc_c).await.unwrap();
        // BCC
        let bcc_c = AgentForCreate { project_id: pid, name: "BCCAgent".into(), program: "test".into(), model: "test".into(), task_description: "BCC".into() };
        let bcc_id = AgentBmc::create(&ctx, &mm, bcc_c).await.unwrap();

        // Grant Capability
        let cap = AgentCapabilityForCreate { agent_id: sender_id, capability: "send_message".into() };
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
        
        let inbox_recv = MessageBmc::list_inbox_for_agent(&ctx, &mm, pid, recv_id, 10).await.unwrap();
        assert_eq!(inbox_recv.len(), 1);
        
        // Correct verification of CC/BCC delivery:
        let inbox_cc = MessageBmc::list_inbox_for_agent(&ctx, &mm, pid, cc_id, 10).await.unwrap();
        assert_eq!(inbox_cc.len(), 1, "CC agent should have message in inbox");
        
        let inbox_bcc = MessageBmc::list_inbox_for_agent(&ctx, &mm, pid, bcc_id, 10).await.unwrap();
        assert_eq!(inbox_bcc.len(), 1, "BCC agent should have message in inbox");
    }
    #[tokio::test]
    async fn test_outbox_resource() {
        use rmcp::model::{ReadResourceRequestParam, ResourceContents};
        use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};

        let (mm, _temp) = create_test_mm().await;
        // Construct service
        let service = MouchakMailService::new_with_mm(mm.clone());
        let ctx = Ctx::root_ctx();

        // Create project
        let pid = ProjectBmc::create(&ctx, &mm, "outbox-test", "Outbox Test").await.unwrap();

        // Create Agents
        let sender_c = AgentForCreate { project_id: pid, name: "Sender".into(), program: "test".into(), model: "test".into(), task_description: "Sender".into() };
        let sender_id = AgentBmc::create(&ctx, &mm, sender_c).await.unwrap();
        
        let recv_c = AgentForCreate { project_id: pid, name: "Recv".into(), program: "test".into(), model: "test".into(), task_description: "Recv".into() };
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
        };
        MessageBmc::create(&ctx, &mm, msg_c).await.unwrap();

        // Call read_resource
        let uri = "mouchak-mail://outbox-test/outbox/Sender".to_string();
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
        use mouchak_mail_core::model::tool_metric::ToolMetricBmc;
        use mouchak_mail_core::model::project::ProjectBmc;
        use mouchak_mail_core::model::agent::{AgentBmc, AgentForCreate};
        use serde_json::json;

        let (mm, _temp) = create_test_mm().await;
        // Construct service
        let service = MouchakMailService::new_with_mm(mm.clone());
        let ctx = Ctx::root_ctx();

        // 1. Create Project and Agent
        let project_id = ProjectBmc::create(&ctx, &mm, "metric-test", "Metric Test").await.unwrap();
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
        let result = Ok(CallToolResult { content: vec![], is_error: None, meta: None, structured_content: None });

        // 3. Call record_tool_metric
        service.record_tool_metric("test_tool", &args, duration, &result).await;

        // 4. Verify DB
        let metrics = ToolMetricBmc::list_recent(&ctx, &mm, Some(project_id), 10).await.unwrap();
        assert_eq!(metrics.len(), 1);
        let m = &metrics[0];
        
        assert_eq!(m.tool_name, "test_tool");
        assert_eq!(m.duration_ms, 123);
        assert_eq!(m.status, "success");
        assert_eq!(m.project_id, Some(project_id));
        assert_eq!(m.agent_id, Some(agent_id));
    }
}
