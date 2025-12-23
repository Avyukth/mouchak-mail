//! Tool Parameter and Response Types
//!
//! This module contains all parameter and response types for MCP tools.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// Tool Parameter Types
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnsureProjectParams {
    /// The project slug (URL-safe identifier)
    pub slug: String,
    /// Human-readable project name/key
    pub human_key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterAgentParams {
    /// Project slug the agent belongs to
    pub project_slug: String,
    /// Agent's unique name within the project (alias: agent_name)
    #[serde(alias = "agent_name")]
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
    /// Whether recipients must acknowledge this message (default: false)
    #[serde(default)]
    pub ack_required: Option<bool>,
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
pub struct GetReviewStateParams {
    /// Project slug
    pub project_slug: String,
    /// Thread ID (e.g., TASK-abc123)
    pub thread_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimReviewParams {
    /// Project slug
    pub project_slug: String,
    /// Message ID of the [COMPLETION] message to review
    pub message_id: i64,
    /// Reviewer agent name
    pub reviewer_name: String,
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
pub struct ListBuiltinWorkflowsParams {
    // No parameters needed
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QuickStandupWorkflowParams {
    /// Project slug
    pub project_slug: String,
    /// Sender agent name
    pub sender_name: String,
    /// Optional custom standup question
    pub standup_question: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QuickHandoffWorkflowParams {
    /// Project slug
    pub project_slug: String,
    /// Agent handing off the task
    pub from_agent: String,
    /// Agent receiving the task
    pub to_agent: String,
    /// Task description
    pub task_description: String,
    /// Optional files being handed off
    pub files: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QuickReviewWorkflowParams {
    /// Project slug
    pub project_slug: String,
    /// Agent requesting review
    pub requester: String,
    /// Agent who will review
    pub reviewer: String,
    /// Files to review
    pub files_to_review: Vec<String>,
    /// Review description
    pub description: String,
}

// ============================================================================
// Macro Convenience Tools (Session/Workflow Helpers)
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MacroStartSessionParams {
    /// Human-readable project key (creates if not exists)
    pub human_key: String,
    /// Agent program identifier (e.g., "claude-code")
    pub program: String,
    /// Agent model identifier (e.g., "opus-4")
    pub model: String,
    /// Agent task description
    #[serde(default)]
    pub task_description: String,
    /// Optional agent name (auto-generated if not provided)
    pub agent_name: Option<String>,
    /// Optional file paths to reserve
    pub file_reservation_paths: Option<Vec<String>>,
    /// Reason for file reservations
    #[serde(default = "default_reservation_reason")]
    pub file_reservation_reason: String,
    /// TTL for file reservations in seconds
    #[serde(default = "default_reservation_ttl")]
    pub file_reservation_ttl_seconds: i64,
    /// Number of inbox messages to fetch
    #[serde(default = "default_inbox_limit")]
    pub inbox_limit: i64,
}

pub fn default_reservation_reason() -> String {
    "macro-session".to_string()
}

pub fn default_reservation_ttl() -> i64 {
    3600
}

pub fn default_inbox_limit() -> i64 {
    10
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MacroPrepareThreadParams {
    /// Project slug or human key
    pub project_key: String,
    /// Thread ID to prepare for
    pub thread_id: String,
    /// Agent program identifier
    pub program: String,
    /// Agent model identifier
    pub model: String,
    /// Optional agent name
    pub agent_name: Option<String>,
    /// Agent task description
    #[serde(default)]
    pub task_description: String,
    /// Register agent if missing
    #[serde(default = "default_true")]
    pub register_if_missing: bool,
    /// Include example messages in thread summary
    #[serde(default = "default_true")]
    pub include_examples: bool,
    /// Number of inbox messages to fetch
    #[serde(default = "default_inbox_limit")]
    pub inbox_limit: i64,
    /// Include message bodies in inbox
    #[serde(default)]
    pub include_inbox_bodies: bool,
}

pub fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MacroFileReservationCycleParams {
    /// Project slug or human key
    pub project_key: String,
    /// Agent name
    pub agent_name: String,
    /// File paths to reserve
    pub paths: Vec<String>,
    /// TTL in seconds
    #[serde(default = "default_reservation_ttl")]
    pub ttl_seconds: i64,
    /// Exclusive reservation
    #[serde(default = "default_true")]
    pub exclusive: bool,
    /// Reason for reservation
    #[serde(default = "default_file_reservation_reason")]
    pub reason: String,
    /// Auto-release immediately after granting (for testing)
    #[serde(default)]
    pub auto_release: bool,
}

pub fn default_file_reservation_reason() -> String {
    "macro-file_reservation".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MacroContactHandshakeParams {
    /// Project slug or human key
    pub project_key: String,
    /// Requester agent name
    pub requester: Option<String>,
    /// Target agent name
    pub target: Option<String>,
    /// Reason for contact request
    #[serde(default)]
    pub reason: String,
    /// TTL for contact permission in seconds (default: 7 days)
    #[serde(default = "default_contact_ttl")]
    pub ttl_seconds: i64,
    /// Auto-accept the contact request
    #[serde(default)]
    pub auto_accept: bool,
    /// Optional welcome message subject
    pub welcome_subject: Option<String>,
    /// Optional welcome message body
    pub welcome_body: Option<String>,
    /// Target project (for cross-project contacts)
    pub to_project: Option<String>,
    /// Alias for requester
    pub agent_name: Option<String>,
    /// Alias for target
    pub to_agent: Option<String>,
    /// Register agent if missing
    #[serde(default = "default_true")]
    pub register_if_missing: bool,
    /// Program for auto-registration
    pub program: Option<String>,
    /// Model for auto-registration
    pub model: Option<String>,
    /// Optional thread ID for welcome message
    pub thread_id: Option<String>,
}

pub fn default_contact_ttl() -> i64 {
    7 * 24 * 3600 // 7 days
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ThreadIdInput {
    Single(String),
    Multiple(Vec<String>),
}

impl From<ThreadIdInput> for Vec<String> {
    fn from(input: ThreadIdInput) -> Self {
        match input {
            ThreadIdInput::Single(s) => vec![s],
            ThreadIdInput::Multiple(v) => v,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SummarizeThreadParams {
    pub project_slug: String,
    pub thread_id: ThreadIdInput,
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
pub struct SearchMessagesProductParams {
    /// Product UID to search across
    pub product_uid: String,
    /// Search query (full-text search)
    pub query: String,
    /// Maximum results per project
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SummarizeThreadProductParams {
    /// Product UID
    pub product_uid: String,
    /// Thread ID(s) to summarize
    pub thread_id: ThreadIdInput,
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListOutboxParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name to list outbox for
    pub agent_name: String,
    /// Maximum number of messages to return
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileReservationPathsParams {
    /// Project slug
    pub project_slug: String,
    /// Agent name requesting reservations
    pub agent_name: String,
    /// File paths to reserve (array)
    pub paths: Vec<String>,
    /// Whether this is an exclusive reservation
    pub exclusive: bool,
    /// Reason for the reservation
    pub reason: Option<String>,
    /// TTL in seconds (default 3600)
    pub ttl_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InstallPrecommitGuardParams {
    /// Project slug
    pub project_slug: String,
    /// Target repository path
    pub target_repo_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UninstallPrecommitGuardParams {
    /// Target repository path
    pub target_repo_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddAttachmentParams {
    /// Project slug
    pub project_slug: String,
    /// Message ID to attach to
    pub message_id: i64,
    /// Filename
    pub filename: String,
    /// Base64 encoded content
    pub content_base64: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetAttachmentParams {
    /// Project slug
    pub project_slug: String,
    /// Attachment ID
    pub attachment_id: String,
    /// Filename
    pub filename: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListToolMetricsParams {
    /// Optional project ID filter
    pub project_id: Option<i64>,
    /// Maximum number of results
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListActivityParams {
    /// Project ID
    pub project_id: i64,
    /// Maximum number of results
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPendingReviewsParams {
    /// Filter by project slug (optional)
    pub project_slug: Option<String>,
    /// Filter by sender agent name (optional, requires project_slug)
    pub sender_name: Option<String>,
    /// Maximum results (default: 5, max: 50)
    pub limit: Option<i64>,
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ReviewStateResponse {
    pub thread_id: String,
    pub state: String,
    pub is_reviewed: bool,
    pub reviewer: Option<String>,
    pub last_update: String,
}

#[derive(Debug, Serialize)]
pub struct ClaimResult {
    pub success: bool,
    pub thread_id: String,
    pub claimed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThreadSummaryItem {
    pub thread_id: String,
    pub subject: String,
    pub message_count: usize,
    pub participants: Vec<String>,
    pub last_snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThreadSummaryError {
    pub thread_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SummarizeResult {
    pub summaries: Vec<ThreadSummaryItem>,
    pub errors: Vec<ThreadSummaryError>,
}
