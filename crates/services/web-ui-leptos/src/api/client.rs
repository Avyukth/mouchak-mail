//! HTTP client for MCP Agent Mail API.

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

/// Get the API base URL.
///
/// In WASM (browser), uses the current window origin so the API is on the same host.
/// This allows the frontend to work correctly when deployed behind a reverse proxy.
///
/// Build-time configuration via `API_BASE_URL` env var is also supported for development.
pub fn api_base_url() -> String {
    // Check for build-time env var first (for development overrides)
    if let Some(url) = option_env!("API_BASE_URL") {
        if !url.is_empty() {
            return url.to_string();
        }
    }

    // In WASM, use the current window origin
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(origin) = window.location().origin() {
                return origin;
            }
        }
    }

    // Fallback for non-WASM or if window is unavailable
    "http://127.0.0.1:8080".to_string()
}

/// Legacy constant for backwards compatibility - prefer api_base_url() function
#[deprecated(since = "0.2.0", note = "Use api_base_url() function instead")]
pub const API_BASE_URL: &str = "http://127.0.0.1:8080";

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
    #[serde(default)]
    pub recipients: Vec<String>,
}

/// Check API health.
pub async fn check_health() -> Result<HealthResponse, ApiError> {
    let url = format!("{}/api/health", api_base_url());
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
    let url = format!("{}/api/projects", api_base_url());
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
    let url = format!("{}/api/project/ensure", api_base_url());

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
    let url = format!("{}/api/projects/{}", api_base_url(), slug);
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
    let url = format!("{}/api/projects/{}/agents", api_base_url(), project_slug);
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
    let url = format!("{}/api/agent/register", api_base_url());

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
    let url = format!("{}/api/agents", api_base_url());
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
    let url = format!("{}/api/inbox", api_base_url());

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
    let url = format!("{}/api/messages/{}", api_base_url(), id);
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
    let url = format!("{}/api/message/send", api_base_url());

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
    let mut url = format!("{}/api/unified-inbox", api_base_url());

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
        api_base_url(),
        project_slug,
        thread_id
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
        api_base_url(),
        project_slug,
        query
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
    let url = format!("{}/api/file_reservations/list", api_base_url());

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
    let url = format!("{}/api/messages/{}/read", api_base_url(), message_id);

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

// -- Attachments API --

/// Attachment response from listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: i64,
    pub project_id: i64,
    /// Agent ID that uploaded the file (if any).
    #[serde(default)]
    pub agent_id: Option<i64>,
    pub filename: String,
    pub stored_path: String,
    pub media_type: String,
    pub size_bytes: i64,
    #[serde(default)]
    pub created_ts: Option<String>,
}

impl Attachment {
    /// Get file type category for icon display.
    pub fn file_type_category(&self) -> &'static str {
        let ext = self
            .filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" => "image",
            "pdf" => "pdf",
            "doc" | "docx" | "odt" => "document",
            "xls" | "xlsx" | "csv" | "ods" => "spreadsheet",
            "mp4" | "webm" | "mov" | "avi" => "video",
            "mp3" | "wav" | "ogg" | "flac" => "audio",
            "zip" | "tar" | "gz" | "7z" | "rar" => "archive",
            "json" | "xml" | "yaml" | "toml" => "code",
            "rs" | "js" | "ts" | "py" | "go" | "java" | "c" | "cpp" | "h" => "code",
            "md" | "txt" | "log" => "text",
            _ => "file",
        }
    }

    /// Get human-readable file size.
    pub fn human_size(&self) -> String {
        let size = self.size_bytes as f64;
        if size < 1024.0 {
            format!("{} B", self.size_bytes)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }

    /// Get lucide icon name for file type.
    pub fn icon_name(&self) -> &'static str {
        match self.file_type_category() {
            "image" => "image",
            "pdf" => "file-text",
            "document" => "file-text",
            "spreadsheet" => "table",
            "video" => "video",
            "audio" => "music",
            "archive" => "archive",
            "code" => "file-code",
            "text" => "file-text",
            _ => "file",
        }
    }
}

/// List attachments for a project, optionally filtered by agent.
pub async fn list_attachments(
    project_slug: &str,
    agent_name: Option<&str>,
) -> Result<Vec<Attachment>, ApiError> {
    let mut url = format!(
        "{}/api/attachments?project_slug={}",
        api_base_url(),
        urlencoding::encode(project_slug)
    );

    if let Some(agent) = agent_name {
        url.push_str(&format!("&agent_name={}", urlencoding::encode(agent)));
    }

    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to list attachments: {}", response.status()),
        })
    }
}

/// Get attachment download URL.
pub fn attachment_download_url(id: i64, project_slug: &str) -> String {
    format!(
        "{}/api/attachments/{}?project_slug={}",
        api_base_url(),
        id,
        urlencoding::encode(project_slug)
    )
}

// -- Archive Browser API --

/// Commit summary from archive browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
    pub files_changed: usize,
}

/// Commit details from archive browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetails {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
    #[serde(default)]
    pub parent_sha: Option<String>,
    #[serde(default)]
    pub files_added: Vec<String>,
    #[serde(default)]
    pub files_modified: Vec<String>,
    #[serde(default)]
    pub files_deleted: Vec<String>,
}

/// File entry in archive browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(default)]
    pub size: Option<i64>,
}

/// File content from archive browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub is_binary: bool,
    #[serde(default)]
    pub size: i64,
}

/// Activity summary from archive browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub total_commits: usize,
    pub authors: Vec<String>,
    #[serde(default)]
    pub files_changed: usize,
    #[serde(default)]
    pub additions: usize,
    #[serde(default)]
    pub deletions: usize,
}

/// Get archive commits.
pub async fn get_archive_commits(limit: Option<usize>) -> Result<Vec<CommitSummary>, ApiError> {
    let mut url = format!("{}/api/archive/commits", api_base_url());
    if let Some(lim) = limit {
        url.push_str(&format!("?limit={}", lim));
    }

    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get commits: {}", response.status()),
        })
    }
}

/// Get archive commit details.
pub async fn get_archive_commit(sha: &str) -> Result<CommitDetails, ApiError> {
    let url = format!("{}/api/archive/commits/{}", api_base_url(), sha);
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get commit: {}", response.status()),
        })
    }
}

/// List files at a specific commit.
pub async fn get_archive_files(sha: &str, path: Option<&str>) -> Result<Vec<FileEntry>, ApiError> {
    let mut url = format!("{}/api/archive/files/{}", api_base_url(), sha);
    if let Some(p) = path {
        url.push_str(&format!("?path={}", urlencoding::encode(p)));
    }

    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to list files: {}", response.status()),
        })
    }
}

/// Get file content at a specific commit.
pub async fn get_archive_file_content(sha: &str, path: &str) -> Result<FileContent, ApiError> {
    let url = format!(
        "{}/api/archive/file/{}?path={}",
        api_base_url(),
        sha,
        urlencoding::encode(path)
    );
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get file content: {}", response.status()),
        })
    }
}

/// Get archive activity summary.
pub async fn get_archive_activity() -> Result<ActivitySummary, ApiError> {
    let url = format!("{}/api/archive/activity", api_base_url());
    let response = Request::get(&url).send().await?;

    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get activity: {}", response.status()),
        })
    }
}
