//! Test fixtures for creating test data

use serde::Deserialize;
use uuid::Uuid;

/// Test fixtures for E2E tests
pub struct TestFixtures;

impl TestFixtures {
    /// Generate a unique project slug for testing
    pub fn unique_project_slug() -> String {
        format!("test-project-{}", Uuid::new_v4().to_string()[..8].to_string())
    }

    /// Generate a unique agent name for testing
    pub fn unique_agent_name() -> String {
        format!("test-agent-{}", Uuid::new_v4().to_string()[..8].to_string())
    }

    /// Create project payload
    pub fn project_payload(slug: &str) -> serde_json::Value {
        serde_json::json!({
            "project_slug": slug
        })
    }

    /// Create agent registration payload
    pub fn agent_payload(project_slug: &str, agent_name: &str) -> serde_json::Value {
        serde_json::json!({
            "project_slug": project_slug,
            "agent_name": agent_name,
            "program": "test-runner",
            "model": "test-model"
        })
    }

    /// Create message payload
    pub fn message_payload(
        project_slug: &str,
        sender: &str,
        recipients: &[&str],
        subject: &str,
        body: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "project_slug": project_slug,
            "sender_name": sender,
            "recipient_names": recipients,
            "subject": subject,
            "body_md": body
        })
    }
}

/// Response from ensure_project endpoint
#[derive(Debug, Deserialize)]
pub struct ProjectResponse {
    pub id: i64,
    pub slug: String,
}

/// Response from register_agent endpoint
#[derive(Debug, Deserialize)]
pub struct AgentResponse {
    pub id: i64,
    pub name: String,
    pub project_id: i64,
}

/// Response from send_message endpoint
#[derive(Debug, Deserialize)]
pub struct MessageResponse {
    pub id: i64,
    pub thread_id: String,
}
