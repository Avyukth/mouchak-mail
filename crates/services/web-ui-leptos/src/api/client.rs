//! HTTP client for MCP Agent Mail API.

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

/// API base URL - defaults to localhost for development.
pub const API_BASE_URL: &str = "http://127.0.0.1:8765";

/// API error type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<gloo_net::Error> for ApiError {
    fn from(e: gloo_net::Error) -> Self {
        ApiError {
            message: e.to_string(),
        }
    }
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Project response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub slug: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub human_key: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Agent response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub project_id: Option<i64>,
    #[serde(default)]
    pub project_slug: Option<String>,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub task_description: Option<String>,
    #[serde(default)]
    pub inception_ts: Option<String>,
    #[serde(default)]
    pub last_active_ts: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Inbox message response (from POST /api/inbox).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessage {
    pub id: i64,
    pub subject: String,
    pub sender_name: String,
    pub created_ts: String,
}

/// Full message response (from GET /api/messages/:id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub project_id: i64,
    pub sender_id: i64,
    pub sender_name: String,
    #[serde(default)]
    pub thread_id: Option<String>,
    pub subject: String,
    pub body_md: String,
    pub importance: String,
    #[serde(default)]
    pub ack_required: bool,
    pub created_ts: String,
    #[serde(default)]
    pub attachments: Vec<serde_json::Value>,
}

/// Check API health.
pub async fn check_health() -> Result<HealthResponse, ApiError> {
    let url = format!("{}/api/health", API_BASE_URL);
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Health check failed: {}", response.status()),
        })
    }
}

/// Get all projects.
pub async fn get_projects() -> Result<Vec<Project>, ApiError> {
    let url = format!("{}/api/projects", API_BASE_URL);
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get projects: {}", response.status()),
        })
    }
}

/// Create or ensure a project exists.
pub async fn ensure_project(human_key: &str) -> Result<Project, ApiError> {
    let url = format!("{}/api/project/ensure", API_BASE_URL);

    #[derive(Serialize)]
    struct CreateProjectPayload<'a> {
        human_key: &'a str,
    }

    let payload = CreateProjectPayload { human_key };

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)?
        .send()
        .await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to create project: {}", response.status()),
        })
    }
}

/// Get project by slug.
pub async fn get_project(slug: &str) -> Result<Project, ApiError> {
    let url = format!("{}/api/projects/{}", API_BASE_URL, slug);
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get project: {}", response.status()),
        })
    }
}

/// Get agents for a project.
pub async fn get_agents(project_slug: &str) -> Result<Vec<Agent>, ApiError> {
    let url = format!("{}/api/projects/{}/agents", API_BASE_URL, project_slug);
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get agents: {}", response.status()),
        })
    }
}

/// Structured error response from backend (RFC 7807 style).
#[derive(Debug, Clone, Deserialize)]
struct BackendError {
    /// Machine-readable error code (e.g., "NOT_FOUND", "CONFLICT").
    #[serde(default)]
    code: Option<String>,
    /// Human-readable error message.
    #[serde(default)]
    error: Option<String>,
}

impl BackendError {
    /// Returns a user-friendly error message.
    fn message(&self) -> String {
        self.error.clone().unwrap_or_else(|| {
            self.code
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string())
        })
    }
}

/// Register a new agent for a project.
pub async fn register_agent(
    project_slug: &str,
    name: &str,
    program: &str,
    model: &str,
    task_description: Option<&str>,
) -> Result<Agent, ApiError> {
    let url = format!("{}/api/agent/register", API_BASE_URL);

    #[derive(Serialize)]
    struct RegisterAgentPayload<'a> {
        project_slug: &'a str,
        name: &'a str,
        program: &'a str,
        model: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_description: Option<&'a str>,
    }

    let payload = RegisterAgentPayload {
        project_slug,
        name,
        program,
        model,
        task_description,
    };

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)?
        .send()
        .await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        // Try to parse structured error response from backend
        let status = response.status();
        let error_msg = match response.json::<BackendError>().await {
            Ok(err) => err.message(),
            Err(_) => format!("HTTP {}", status),
        };
        Err(ApiError {
            message: format!("Failed to register agent: {}", error_msg),
        })
    }
}

/// Get all agents.
pub async fn get_all_agents() -> Result<Vec<Agent>, ApiError> {
    let url = format!("{}/api/agents", API_BASE_URL);
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get agents: {}", response.status()),
        })
    }
}

/// Get inbox messages for a specific project and agent.
pub async fn get_inbox(
    project_slug: &str,
    agent_name: &str,
) -> Result<Vec<InboxMessage>, ApiError> {
    let url = format!("{}/api/inbox", API_BASE_URL);

    let payload = serde_json::json!({
        "project_slug": project_slug,
        "agent_name": agent_name,
        "limit": 50
    });

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .map_err(|e| ApiError {
            message: e.to_string(),
        })?
        .send()
        .await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get inbox: {}", response.status()),
        })
    }
}

/// Get a single message by ID.
pub async fn get_message(id: &str) -> Result<Message, ApiError> {
    let url = format!("{}/api/messages/{}", API_BASE_URL, id);
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get message: {}", response.status()),
        })
    }
}

/// Send a message.
#[allow(clippy::too_many_arguments)]
pub async fn send_message(
    project_slug: &str,
    sender: &str,
    recipients: &[String],
    subject: &str,
    body: &str,
    thread_id: Option<&str>,
    importance: &str,
    _ack_required: bool,
) -> Result<Message, ApiError> {
    let url = format!("{}/api/message/send", API_BASE_URL);

    #[derive(Serialize)]
    struct SendMessagePayload<'a> {
        project_slug: &'a str,
        sender_name: &'a str,
        recipient_names: &'a [String],
        subject: &'a str,
        body_md: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        thread_id: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        importance: Option<&'a str>,
    }

    let payload = SendMessagePayload {
        project_slug,
        sender_name: sender,
        recipient_names: recipients,
        subject,
        body_md: body,
        thread_id,
        importance: Some(importance),
    };

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)?
        .send()
        .await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to send message: {}", response.status()),
        })
    }
}

/// Unified inbox message (from GET /api/unified-inbox).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedInboxMessage {
    pub id: i64,
    pub project_id: i64,
    pub project_slug: String,
    pub sender_id: i64,
    pub sender_name: String,
    pub subject: String,
    pub importance: String,
    pub created_ts: String,
    #[serde(default)]
    pub thread_id: Option<String>,
}

/// Get unified inbox (all messages across all projects).
pub async fn get_unified_inbox(
    importance: Option<&str>,
    limit: Option<i32>,
) -> Result<Vec<UnifiedInboxMessage>, ApiError> {
    let mut url = format!("{}/api/unified-inbox", API_BASE_URL);

    let mut params = Vec::new();
    if let Some(imp) = importance {
        params.push(format!("importance={}", imp));
    }
    if let Some(lim) = limit {
        params.push(format!("limit={}", lim));
    }
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get unified inbox: {}", response.status()),
        })
    }
}

/// Get messages in a thread.
pub async fn get_thread(project_slug: &str, thread_id: &str) -> Result<Vec<Message>, ApiError> {
    let url = format!(
        "{}/api/projects/{}/threads/{}",
        API_BASE_URL, project_slug, thread_id
    );
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get thread: {}", response.status()),
        })
    }
}

/// Search messages.
pub async fn search_messages(project_slug: &str, query: &str) -> Result<Vec<Message>, ApiError> {
    let url = format!(
        "{}/api/projects/{}/search?q={}",
        API_BASE_URL, project_slug, query
    );
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to search: {}", response.status()),
        })
    }
}

/// File reservation response from API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReservationResponse {
    pub id: i64,
    pub agent_name: String,
    pub path_pattern: String,
    pub exclusive: bool,
    pub reason: Option<String>,
    pub created_ts: String,
    pub expires_ts: Option<String>,
    #[serde(default)]
    pub expired: bool,
}

/// Get file reservations for a project.
pub async fn get_file_reservations(
    project_slug: &str,
) -> Result<Vec<FileReservationResponse>, ApiError> {
    let url = format!("{}/api/file_reservations/list", API_BASE_URL);

    #[derive(Serialize)]
    struct Payload<'a> {
        project_slug: &'a str,
    }

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .json(&Payload { project_slug })?
        .send()
        .await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get reservations: {}", response.status()),
        })
    }
}

/// Mark read response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkReadResponse {
    pub success: bool,
    #[serde(default)]
    pub message: Option<String>,
}

/// Mark a message as read/unread.
///
/// # Arguments
/// * `message_id` - The message ID to mark
/// * `project_slug` - Project context
/// * `agent_name` - Agent marking the message
/// * `is_read` - True to mark as read, false to mark as unread
pub async fn mark_read(
    message_id: i64,
    project_slug: &str,
    agent_name: &str,
    is_read: bool,
) -> Result<MarkReadResponse, ApiError> {
    let url = format!("{}/api/messages/{}/read", API_BASE_URL, message_id);

    #[derive(Serialize)]
    struct Payload<'a> {
        project_slug: &'a str,
        agent_name: &'a str,
        is_read: bool,
    }

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .json(&Payload {
            project_slug,
            agent_name,
            is_read,
        })?
        .send()
        .await?;

    if response.ok() {
        Ok(MarkReadResponse {
            success: true,
            message: Some("Message read status updated".to_string()),
        })
    } else {
        Err(ApiError {
            message: format!("Failed to update read status: {}", response.status()),
        })
    }
}
