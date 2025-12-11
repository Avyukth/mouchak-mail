//! HTTP client for MCP Agent Mail API.

use serde::{Deserialize, Serialize};
use gloo_net::http::Request;

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
    pub id: Option<String>,
    pub slug: String,
    pub name: Option<String>,
    pub human_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// Agent response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Option<String>,
    pub name: String,
    pub project_id: Option<String>,
    pub project_slug: Option<String>,
    pub program: Option<String>,
    pub model: Option<String>,
    pub task_description: Option<String>,
    pub last_active_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// Message response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub thread_id: Option<String>,
    pub sender: String,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub importance: Option<String>,
    pub ack_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// Check API health.
pub async fn check_health() -> Result<HealthResponse, ApiError> {
    let url = format!("{}/api/health", API_BASE_URL);
    let response = Request::get(&url)
        .send()
        .await?;
    
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
    let response = Request::get(&url)
        .send()
        .await?;
    
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
    let url = format!("{}/api/projects", API_BASE_URL);
    
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
    let response = Request::get(&url)
        .send()
        .await?;
    
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
    let response = Request::get(&url)
        .send()
        .await?;
    
    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get agents: {}", response.status()),
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
    let url = format!("{}/api/projects/{}/agents", API_BASE_URL, project_slug);
    
    #[derive(Serialize)]
    struct RegisterAgentPayload<'a> {
        name: &'a str,
        program: &'a str,
        model: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_description: Option<&'a str>,
    }
    
    let payload = RegisterAgentPayload {
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
        Err(ApiError {
            message: format!("Failed to register agent: {}", response.status()),
        })
    }
}

/// Get all agents.
pub async fn get_all_agents() -> Result<Vec<Agent>, ApiError> {
    let url = format!("{}/api/agents", API_BASE_URL);
    let response = Request::get(&url)
        .send()
        .await?;
    
    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get agents: {}", response.status()),
        })
    }
}

/// Get messages (inbox).
pub async fn get_messages(project_slug: Option<&str>, agent: Option<&str>) -> Result<Vec<Message>, ApiError> {
    let mut url = format!("{}/api/messages", API_BASE_URL);
    let mut params = Vec::new();
    
    if let Some(p) = project_slug {
        params.push(format!("project={}", p));
    }
    if let Some(a) = agent {
        params.push(format!("agent={}", a));
    }
    
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }
    
    let response = Request::get(&url)
        .send()
        .await?;
    
    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get messages: {}", response.status()),
        })
    }
}

/// Get inbox messages for a specific project and agent.
pub async fn get_inbox(project_slug: &str, agent: &str) -> Result<Vec<Message>, ApiError> {
    get_messages(Some(project_slug), Some(agent)).await
}

/// Get a single message by ID.
pub async fn get_message(id: &str) -> Result<Message, ApiError> {
    let url = format!("{}/api/messages/{}", API_BASE_URL, id);
    let response = Request::get(&url)
        .send()
        .await?;
    
    if response.ok() {
        Ok(response.json().await?)
    } else {
        Err(ApiError {
            message: format!("Failed to get message: {}", response.status()),
        })
    }
}
