//! Unit tests for lib-server/tools.rs handlers
//!
//! These tests verify the HTTP handlers in tools.rs by calling them directly
//! with test AppState containing an isolated ModelManager.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
};
use http_body_util::BodyExt;
use metrics_exporter_prometheus::PrometheusBuilder;
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ModelManager;
use mouchak_mail_server::auth::{AuthConfig, AuthMode};
use mouchak_mail_server::ratelimit::RateLimitConfig;
use mouchak_mail_server::{AppState, tools};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;
use tower::ServiceExt;

/// Create a test AppState with isolated database
async fn create_test_state() -> (AppState, TempDir) {
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
    let schema5 = include_str!("../../../../migrations/005_attachments_agent.sql");
    conn.execute_batch(schema5).await.unwrap();
    let schema6 = include_str!("../../../../migrations/006_query_indexes.sql");
    conn.execute_batch(schema6).await.unwrap();

    let app_config = Arc::new(AppConfig::default());
    let mm = ModelManager::new_for_test(conn, archive_root, app_config);

    // Create metrics handle (use a test-only builder to avoid conflicts)
    let metrics_handle = PrometheusBuilder::new()
        .install_recorder()
        .unwrap_or_else(|_| {
            // If recorder already installed, use a dummy handle
            PrometheusBuilder::new().build_recorder().handle()
        });

    let auth_config = AuthConfig {
        mode: AuthMode::None,
        bearer_token: None,
        jwks_url: None,
        jwt_audience: None,
        jwt_issuer: None,
        allow_localhost: true,
    };

    let state = AppState {
        mm,
        metrics_handle,
        start_time: Instant::now(),
        auth_config,
        jwks_client: None,
        ratelimit_config: RateLimitConfig::new(),
    };

    (state, temp_dir)
}

/// Helper to send POST request and get response
async fn post_json(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();
    let body_json: Value = serde_json::from_str(&body_str).unwrap_or(json!({"raw": body_str}));

    (status, body_json)
}

/// Helper to send GET request and get response
async fn get_json(app: Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();
    let body_json: Value = serde_json::from_str(&body_str).unwrap_or(json!({"raw": body_str}));

    (status, body_json)
}

// =============================================================================
// Health Check Tests
// =============================================================================

mod health_tests {
    use super::*;

    fn create_app(state: AppState) -> Router {
        Router::new()
            .route("/api/health", get(tools::health_check))
            .route("/api/ready", get(tools::readiness_check))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_health_check_returns_ok() {
        let (state, _temp) = create_test_state().await;
        let app = create_app(state);

        let (status, body) = get_json(app, "/api/health").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert!(body["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_readiness_check_with_healthy_db() {
        let (state, _temp) = create_test_state().await;
        let app = create_app(state);

        let (status, body) = get_json(app, "/api/ready").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ready");
        assert!(body["checks"]["database"]["ok"].as_bool().unwrap());
    }
}

// =============================================================================
// Project Tests
// =============================================================================

mod project_tests {
    use super::*;

    fn create_app(state: AppState) -> Router {
        Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .route("/api/projects", get(tools::list_all_projects))
            .route("/api/project/info", post(tools::get_project_info))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_ensure_project_creates_new_project() {
        let (state, _temp) = create_test_state().await;
        let app = create_app(state);

        let (status, body) = post_json(
            app,
            "/api/project/ensure",
            json!({
                "human_key": "test-project"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["id"].as_i64().unwrap() > 0);
        assert!(body["slug"].is_string());
        assert_eq!(body["human_key"], "test-project");
    }

    #[tokio::test]
    async fn test_ensure_project_returns_existing_project() {
        let (state, _temp) = create_test_state().await;
        let app = create_app(state.clone());

        // Create first
        let (_, body1) = post_json(
            app,
            "/api/project/ensure",
            json!({
                "human_key": "existing-project"
            }),
        )
        .await;

        // Ensure again - should return same project
        let app2 = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state);

        let (status, body2) = post_json(
            app2,
            "/api/project/ensure",
            json!({
                "human_key": "existing-project"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body1["id"], body2["id"]);
    }

    #[tokio::test]
    async fn test_list_all_projects() {
        let (state, _temp) = create_test_state().await;

        // Create a project first
        let app1 = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        post_json(
            app1,
            "/api/project/ensure",
            json!({"human_key": "list-test-project"}),
        )
        .await;

        // List projects
        let app2 = Router::new()
            .route("/api/projects", get(tools::list_all_projects))
            .with_state(state);

        let (status, body) = get_json(app2, "/api/projects").await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.as_array().unwrap().len() >= 1);
    }

    #[tokio::test]
    async fn test_get_project_info() {
        let (state, _temp) = create_test_state().await;

        // Create project first
        let app1 = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app1,
            "/api/project/ensure",
            json!({"human_key": "info-test"}),
        )
        .await;

        // Get project info
        let app2 = Router::new()
            .route("/api/project/info", post(tools::get_project_info))
            .with_state(state);

        let (status, body) = post_json(
            app2,
            "/api/project/info",
            json!({"project_slug": proj["slug"]}),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["human_key"], "info-test");
        assert!(body["agent_count"].as_i64().is_some());
        assert!(body["message_count"].as_i64().is_some());
    }
}

// =============================================================================
// Agent Tests
// =============================================================================

mod agent_tests {
    use super::*;

    async fn setup_with_project(state: &AppState) -> String {
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, body) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "agent-test-proj"}),
        )
        .await;
        body["slug"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_register_agent() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_with_project(&state).await;

        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "TestAgent",
                "program": "claude-code",
                "model": "claude-opus-4",
                "task_description": "Integration test agent"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["id"].as_i64().unwrap() > 0);
        assert_eq!(body["name"], "TestAgent");
        assert_eq!(body["program"], "claude-code");
    }

    #[tokio::test]
    async fn test_whois_agent() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_with_project(&state).await;

        // Register agent
        let app1 = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app1,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "WhoisAgent",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        // Whois lookup
        let app2 = Router::new()
            .route("/api/agent/whois", post(tools::whois))
            .with_state(state);

        let (status, body) = post_json(
            app2,
            "/api/agent/whois",
            json!({
                "project_slug": project_slug,
                "agent_name": "WhoisAgent"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["name"], "WhoisAgent");
        assert!(body["project_slug"].is_string());
    }

    #[tokio::test]
    async fn test_create_agent_identity() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_with_project(&state).await;

        let app = Router::new()
            .route(
                "/api/agent/create_identity",
                post(tools::create_agent_identity),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/agent/create_identity",
            json!({
                "project_slug": project_slug,
                "hint": "Blue"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["suggested_name"].is_string());
        assert!(body["alternatives"].as_array().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_list_agents_for_project() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_with_project(&state).await;

        // Register an agent
        let app1 = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app1,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "ListTestAgent",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        // List agents
        let app2 = Router::new()
            .route(
                "/api/projects/{slug}/agents",
                get(tools::list_all_agents_for_project),
            )
            .with_state(state);

        let (status, body) =
            get_json(app2, &format!("/api/projects/{}/agents", project_slug)).await;

        assert_eq!(status, StatusCode::OK);
        let agents = body.as_array().unwrap();
        assert!(agents.len() >= 1);
        assert!(agents.iter().any(|a| a["name"] == "ListTestAgent"));
    }
}

// =============================================================================
// Message Tests
// =============================================================================

mod message_tests {
    use super::*;

    async fn setup_with_agents(state: &AppState) -> (String, String, String) {
        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "msg-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create sender agent
        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "SenderAgent",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        // Create recipient agent
        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "RecipientAgent",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        (
            project_slug,
            "SenderAgent".to_string(),
            "RecipientAgent".to_string(),
        )
    }

    #[tokio::test]
    async fn test_send_message() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, sender, recipient) = setup_with_agents(&state).await;

        let app = Router::new()
            .route("/api/message/send", post(tools::send_message))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/message/send",
            json!({
                "project_slug": project_slug,
                "sender_name": sender,
                "recipient_names": [recipient],
                "subject": "Test Subject",
                "body_md": "Test message body",
                "importance": "normal"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["id"].as_i64().unwrap() > 0);
        assert_eq!(body["subject"], "Test Subject");
        assert_eq!(body["sender_name"], sender);
    }

    #[tokio::test]
    async fn test_send_message_with_cc_bcc() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, sender, recipient) = setup_with_agents(&state).await;

        // Create a CC agent
        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "CcAgent",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        let app = Router::new()
            .route("/api/message/send", post(tools::send_message))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/message/send",
            json!({
                "project_slug": project_slug,
                "sender_name": sender,
                "recipient_names": [recipient],
                "cc_names": ["CcAgent"],
                "subject": "CC Test",
                "body_md": "Test with CC"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["id"].as_i64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_list_inbox() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, sender, recipient) = setup_with_agents(&state).await;

        // Send a message first
        let app = Router::new()
            .route("/api/message/send", post(tools::send_message))
            .with_state(state.clone());
        post_json(
            app,
            "/api/message/send",
            json!({
                "project_slug": project_slug,
                "sender_name": sender,
                "recipient_names": [recipient],
                "subject": "Inbox Test",
                "body_md": "Check inbox"
            }),
        )
        .await;

        // Check inbox
        let app = Router::new()
            .route("/api/inbox", post(tools::list_inbox))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/inbox",
            json!({
                "project_slug": project_slug,
                "agent_name": recipient,
                "limit": 10
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let messages = body.as_array().unwrap();
        assert!(messages.len() >= 1);
        assert!(messages.iter().any(|m| m["subject"] == "Inbox Test"));
    }

    #[tokio::test]
    async fn test_list_outbox() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, sender, recipient) = setup_with_agents(&state).await;

        // Send a message
        let app = Router::new()
            .route("/api/message/send", post(tools::send_message))
            .with_state(state.clone());
        post_json(
            app,
            "/api/message/send",
            json!({
                "project_slug": project_slug,
                "sender_name": sender,
                "recipient_names": [recipient],
                "subject": "Outbox Test",
                "body_md": "Check outbox"
            }),
        )
        .await;

        // Check outbox
        let app = Router::new()
            .route("/api/outbox", post(tools::list_outbox))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/outbox",
            json!({
                "project_slug": project_slug,
                "agent_name": sender,
                "limit": 10
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let messages = body.as_array().unwrap();
        assert!(messages.len() >= 1);
    }

    #[tokio::test]
    async fn test_search_messages() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, sender, recipient) = setup_with_agents(&state).await;

        // Send a message with unique content
        let app = Router::new()
            .route("/api/message/send", post(tools::send_message))
            .with_state(state.clone());
        post_json(
            app,
            "/api/message/send",
            json!({
                "project_slug": project_slug,
                "sender_name": sender,
                "recipient_names": [recipient],
                "subject": "Searchable Subject",
                "body_md": "UniqueSearchKeyword123"
            }),
        )
        .await;

        // Search for it
        let app = Router::new()
            .route("/api/messages/search", post(tools::search_messages))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/messages/search",
            json!({
                "project_slug": project_slug,
                "query": "UniqueSearchKeyword123"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["count"].as_i64().unwrap() >= 1);
    }
}

// =============================================================================
// File Reservation Tests
// =============================================================================

mod file_reservation_tests {
    use super::*;

    async fn setup_with_agent(state: &AppState) -> (String, String) {
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "res-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "ResAgent",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        (project_slug, "ResAgent".to_string())
    }

    #[tokio::test]
    async fn test_file_reservation_paths() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name) = setup_with_agent(&state).await;

        let app = Router::new()
            .route(
                "/api/file_reservations/paths",
                post(tools::file_reservation_paths),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/file_reservations/paths",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "paths": ["src/**/*.rs", "Cargo.toml"],
                "exclusive": true,
                "ttl_seconds": 3600
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["granted"].as_array().unwrap().len(), 2);
        assert!(body["conflicts"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_file_reservations() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name) = setup_with_agent(&state).await;

        // Create a reservation
        let app = Router::new()
            .route(
                "/api/file_reservations/paths",
                post(tools::file_reservation_paths),
            )
            .with_state(state.clone());
        post_json(
            app,
            "/api/file_reservations/paths",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "paths": ["test/**"],
                "exclusive": true
            }),
        )
        .await;

        // List reservations
        let app = Router::new()
            .route(
                "/api/file_reservations/list",
                post(tools::list_file_reservations),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/file_reservations/list",
            json!({
                "project_slug": project_slug
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.as_array().unwrap().len() >= 1);
    }

    #[tokio::test]
    async fn test_release_file_reservation() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name) = setup_with_agent(&state).await;

        // Create reservation
        let app = Router::new()
            .route(
                "/api/file_reservations/paths",
                post(tools::file_reservation_paths),
            )
            .with_state(state.clone());
        post_json(
            app,
            "/api/file_reservations/paths",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "paths": ["release-test/**"],
                "exclusive": true
            }),
        )
        .await;

        // Release it
        let app = Router::new()
            .route(
                "/api/file_reservations/release",
                post(tools::release_file_reservation),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/file_reservations/release",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "paths": ["release-test/**"]
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["released_count"], 1);
    }

    #[tokio::test]
    async fn test_list_all_locks() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name) = setup_with_agent(&state).await;

        // Create a lock
        let app = Router::new()
            .route(
                "/api/file_reservations/paths",
                post(tools::file_reservation_paths),
            )
            .with_state(state.clone());
        post_json(
            app,
            "/api/file_reservations/paths",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "paths": ["all-locks-test/**"],
                "exclusive": true
            }),
        )
        .await;

        // List all locks
        let app = Router::new()
            .route("/api/locks", get(tools::list_all_locks))
            .with_state(state);

        let (status, body) = get_json(app, "/api/locks").await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.as_array().unwrap().len() >= 1);
    }
}

// =============================================================================
// Thread Tests
// =============================================================================

mod thread_tests {
    use super::*;

    async fn setup_with_thread(state: &AppState) -> (String, String) {
        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "thread-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create agents
        for name in ["ThreadSender", "ThreadRecipient"] {
            let app = Router::new()
                .route("/api/agent/register", post(tools::register_agent))
                .with_state(state.clone());
            post_json(
                app,
                "/api/agent/register",
                json!({
                    "project_slug": project_slug,
                    "name": name,
                    "program": "test",
                    "model": "test"
                }),
            )
            .await;
        }

        // Send message with thread_id
        let app = Router::new()
            .route("/api/message/send", post(tools::send_message))
            .with_state(state.clone());
        post_json(
            app,
            "/api/message/send",
            json!({
                "project_slug": project_slug,
                "sender_name": "ThreadSender",
                "recipient_names": ["ThreadRecipient"],
                "subject": "Thread Test",
                "body_md": "Message in thread",
                "thread_id": "TEST-THREAD-001"
            }),
        )
        .await;

        (project_slug, "TEST-THREAD-001".to_string())
    }

    #[tokio::test]
    async fn test_get_thread() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, thread_id) = setup_with_thread(&state).await;

        let app = Router::new()
            .route("/api/thread", post(tools::get_thread))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/thread",
            json!({
                "project_slug": project_slug,
                "thread_id": thread_id
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.as_array().unwrap().len() >= 1);
    }

    #[tokio::test]
    async fn test_list_threads() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, _) = setup_with_thread(&state).await;

        let app = Router::new()
            .route("/api/threads", post(tools::list_threads))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/threads",
            json!({
                "project_slug": project_slug
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.as_array().unwrap().len() >= 1);
    }

    #[tokio::test]
    async fn test_summarize_thread() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, thread_id) = setup_with_thread(&state).await;

        let app = Router::new()
            .route("/api/thread/summarize", post(tools::summarize_thread))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/thread/summarize",
            json!({
                "project_slug": project_slug,
                "thread_id": thread_id,
                "no_llm": true
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["thread_id"], thread_id);
        assert!(body["message_count"].as_i64().unwrap() >= 1);
        assert!(body["summary"].is_string());
    }
}

// =============================================================================
// Build Slot Tests
// =============================================================================

mod build_slot_tests {
    use super::*;

    async fn setup_with_agent(state: &AppState) -> (String, String) {
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "build-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "BuildAgent",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        (project_slug, "BuildAgent".to_string())
    }

    #[tokio::test]
    async fn test_acquire_build_slot() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name) = setup_with_agent(&state).await;

        let app = Router::new()
            .route("/api/build_slots/acquire", post(tools::acquire_build_slot))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/build_slots/acquire",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "slot_name": "test-slot",
                "ttl_seconds": 600
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["slot_id"].as_i64().unwrap() > 0);
        assert_eq!(body["slot_name"], "test-slot");
    }

    #[tokio::test]
    async fn test_renew_build_slot() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name) = setup_with_agent(&state).await;

        // Acquire first
        let app = Router::new()
            .route("/api/build_slots/acquire", post(tools::acquire_build_slot))
            .with_state(state.clone());
        let (_, slot) = post_json(
            app,
            "/api/build_slots/acquire",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "slot_name": "renew-slot",
                "ttl_seconds": 300
            }),
        )
        .await;

        // Renew
        let app = Router::new()
            .route("/api/build_slots/renew", post(tools::renew_build_slot))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/build_slots/renew",
            json!({
                "slot_id": slot["slot_id"],
                "ttl_seconds": 600
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["renewed"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_release_build_slot() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name) = setup_with_agent(&state).await;

        // Acquire first
        let app = Router::new()
            .route("/api/build_slots/acquire", post(tools::acquire_build_slot))
            .with_state(state.clone());
        let (_, slot) = post_json(
            app,
            "/api/build_slots/acquire",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "slot_name": "release-slot"
            }),
        )
        .await;

        // Release
        let app = Router::new()
            .route("/api/build_slots/release", post(tools::release_build_slot))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/build_slots/release",
            json!({
                "slot_id": slot["slot_id"]
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["released"].as_bool().unwrap());
    }
}

// =============================================================================
// Contact Tests
// =============================================================================

mod contact_tests {
    use super::*;

    async fn setup_with_agents(state: &AppState) -> (String, String, String) {
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "contact-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        for name in ["ContactRequester", "ContactTarget"] {
            let app = Router::new()
                .route("/api/agent/register", post(tools::register_agent))
                .with_state(state.clone());
            post_json(
                app,
                "/api/agent/register",
                json!({
                    "project_slug": project_slug,
                    "name": name,
                    "program": "test",
                    "model": "test"
                }),
            )
            .await;
        }

        (
            project_slug,
            "ContactRequester".to_string(),
            "ContactTarget".to_string(),
        )
    }

    #[tokio::test]
    async fn test_request_contact() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, requester, target) = setup_with_agents(&state).await;

        let app = Router::new()
            .route("/api/contact/request", post(tools::request_contact))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/contact/request",
            json!({
                "from_project_slug": project_slug,
                "from_agent_name": requester,
                "to_project_slug": project_slug,
                "to_agent_name": target,
                "reason": "Test contact request"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["link_id"].as_i64().unwrap() > 0);
        assert_eq!(body["status"], "pending");
    }

    #[tokio::test]
    async fn test_respond_contact() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, requester, target) = setup_with_agents(&state).await;

        // Request first
        let app = Router::new()
            .route("/api/contact/request", post(tools::request_contact))
            .with_state(state.clone());
        let (_, req) = post_json(
            app,
            "/api/contact/request",
            json!({
                "from_project_slug": project_slug,
                "from_agent_name": requester,
                "to_project_slug": project_slug,
                "to_agent_name": target,
                "reason": "Accept test"
            }),
        )
        .await;

        // Accept
        let app = Router::new()
            .route("/api/contact/respond", post(tools::respond_contact))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/contact/respond",
            json!({
                "link_id": req["link_id"],
                "accept": true
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "accepted");
    }

    #[tokio::test]
    async fn test_list_contacts() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, requester, target) = setup_with_agents(&state).await;

        // Create and accept a contact
        let app = Router::new()
            .route("/api/contact/request", post(tools::request_contact))
            .with_state(state.clone());
        let (_, req) = post_json(
            app,
            "/api/contact/request",
            json!({
                "from_project_slug": project_slug,
                "from_agent_name": requester,
                "to_project_slug": project_slug,
                "to_agent_name": target,
                "reason": "List test"
            }),
        )
        .await;

        let app = Router::new()
            .route("/api/contact/respond", post(tools::respond_contact))
            .with_state(state.clone());
        post_json(
            app,
            "/api/contact/respond",
            json!({
                "link_id": req["link_id"],
                "accept": true
            }),
        )
        .await;

        // List contacts
        let app = Router::new()
            .route("/api/contacts", post(tools::list_contacts))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/contacts",
            json!({
                "project_slug": project_slug,
                "agent_name": requester
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.as_array().unwrap().len() >= 1);
    }
}

// =============================================================================
// Macro Tests
// =============================================================================

mod macro_tests {
    use super::*;

    async fn setup_project(state: &AppState) -> String {
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "macro-test-proj"}),
        )
        .await;
        proj["slug"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_register_macro() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_project(&state).await;

        let app = Router::new()
            .route("/api/macro/register", post(tools::register_macro))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/macro/register",
            json!({
                "project_slug": project_slug,
                "name": "test-macro",
                "description": "A test macro",
                "steps": [
                    {"action": "step1", "params": {}},
                    {"action": "step2", "params": {}}
                ]
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["macro_id"].as_i64().unwrap() > 0);
        assert_eq!(body["name"], "test-macro");
    }

    #[tokio::test]
    async fn test_list_macros() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_project(&state).await;

        // Register a macro first
        let app = Router::new()
            .route("/api/macro/register", post(tools::register_macro))
            .with_state(state.clone());
        post_json(
            app,
            "/api/macro/register",
            json!({
                "project_slug": project_slug,
                "name": "list-test-macro",
                "description": "For list test",
                "steps": [{"action": "test"}]
            }),
        )
        .await;

        // List macros
        let app = Router::new()
            .route("/api/macros", post(tools::list_macros))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/macros",
            json!({
                "project_slug": project_slug
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        // Should have built-in macros plus our test macro
        assert!(body.as_array().unwrap().len() >= 1);
    }

    #[tokio::test]
    async fn test_invoke_macro() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_project(&state).await;

        // Register a macro
        let app = Router::new()
            .route("/api/macro/register", post(tools::register_macro))
            .with_state(state.clone());
        post_json(
            app,
            "/api/macro/register",
            json!({
                "project_slug": project_slug,
                "name": "invoke-test",
                "description": "Invoke test",
                "steps": [{"action": "step1"}, {"action": "step2"}]
            }),
        )
        .await;

        // Invoke it
        let app = Router::new()
            .route("/api/macro/invoke", post(tools::invoke_macro))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/macro/invoke",
            json!({
                "project_slug": project_slug,
                "name": "invoke-test"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["name"], "invoke-test");
        assert_eq!(body["steps"].as_array().unwrap().len(), 2);
    }
}

// =============================================================================
// Agent Profile Tests
// =============================================================================

mod profile_tests {
    use super::*;

    async fn setup_with_agent(state: &AppState) -> (String, String) {
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "profile-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "ProfileAgent",
                "program": "test",
                "model": "test",
                "task_description": "Profile test"
            }),
        )
        .await;

        (project_slug, "ProfileAgent".to_string())
    }

    #[tokio::test]
    async fn test_get_agent_profile() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name) = setup_with_agent(&state).await;

        let app = Router::new()
            .route("/api/agent/profile", post(tools::get_agent_profile))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/agent/profile",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["name"], "ProfileAgent");
        assert!(body["message_count_sent"].as_i64().is_some());
        assert!(body["message_count_received"].as_i64().is_some());
    }

    #[tokio::test]
    async fn test_update_agent_profile() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name) = setup_with_agent(&state).await;

        let app = Router::new()
            .route(
                "/api/agent/profile/update",
                post(tools::update_agent_profile),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/agent/profile/update",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "task_description": "Updated task description",
                "contact_policy": "auto"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["updated"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_set_contact_policy() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name) = setup_with_agent(&state).await;

        let app = Router::new()
            .route("/api/agent/contact_policy", post(tools::set_contact_policy))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/agent/contact_policy",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "contact_policy": "manual"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["updated"].as_bool().unwrap());
        assert_eq!(body["contact_policy"], "manual");
    }
}

// =============================================================================
// Message Acknowledgment Tests
// =============================================================================

mod ack_tests {
    use super::*;

    async fn setup_with_message(state: &AppState) -> (String, String, i64) {
        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "ack-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create agents
        for name in ["AckSender", "AckRecipient"] {
            let app = Router::new()
                .route("/api/agent/register", post(tools::register_agent))
                .with_state(state.clone());
            post_json(
                app,
                "/api/agent/register",
                json!({
                    "project_slug": project_slug,
                    "name": name,
                    "program": "test",
                    "model": "test"
                }),
            )
            .await;
        }

        // Send message
        let app = Router::new()
            .route("/api/message/send", post(tools::send_message))
            .with_state(state.clone());
        let (_, msg) = post_json(
            app,
            "/api/message/send",
            json!({
                "project_slug": project_slug,
                "sender_name": "AckSender",
                "recipient_names": ["AckRecipient"],
                "subject": "Ack Test",
                "body_md": "Test ack",
                "ack_required": true
            }),
        )
        .await;

        (
            project_slug,
            "AckRecipient".to_string(),
            msg["id"].as_i64().unwrap(),
        )
    }

    #[tokio::test]
    async fn test_mark_message_read() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name, message_id) = setup_with_message(&state).await;

        let app = Router::new()
            .route("/api/message/read", post(tools::mark_message_read))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/message/read",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "message_id": message_id
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["marked"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_acknowledge_message() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, agent_name, message_id) = setup_with_message(&state).await;

        let app = Router::new()
            .route("/api/message/acknowledge", post(tools::acknowledge_message))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/message/acknowledge",
            json!({
                "project_slug": project_slug,
                "agent_name": agent_name,
                "message_id": message_id
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["acknowledged"].as_bool().unwrap());
    }
}

// =============================================================================
// Delete Operations Tests
// =============================================================================

mod delete_tests {
    use super::*;

    #[tokio::test]
    async fn test_delete_agent() {
        let (state, _temp) = create_test_state().await;

        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "delete-agent-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create agent
        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "ToDelete",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        // Delete agent
        let app = Router::new()
            .route(
                "/api/projects/{project_slug}/agents/{agent_name}",
                axum::routing::delete(tools::delete_agent),
            )
            .with_state(state);

        let request = Request::builder()
            .method("DELETE")
            .uri(format!("/api/projects/{}/agents/ToDelete", project_slug))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_delete_project() {
        let (state, _temp) = create_test_state().await;

        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "delete-proj-test"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Delete project
        let app = Router::new()
            .route(
                "/api/projects/{project_slug}",
                axum::routing::delete(tools::delete_project),
            )
            .with_state(state);

        let request = Request::builder()
            .method("DELETE")
            .uri(format!("/api/projects/{}", project_slug))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

// =============================================================================
// Message Extended Tests
// =============================================================================

mod message_extended_tests {
    use super::*;

    async fn setup_with_message(state: &AppState) -> (String, i64) {
        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "msg-ext-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create agents
        for name in ["ExtSender", "ExtRecipient"] {
            let app = Router::new()
                .route("/api/agent/register", post(tools::register_agent))
                .with_state(state.clone());
            post_json(
                app,
                "/api/agent/register",
                json!({
                    "project_slug": project_slug,
                    "name": name,
                    "program": "test",
                    "model": "test"
                }),
            )
            .await;
        }

        // Send message
        let app = Router::new()
            .route("/api/message/send", post(tools::send_message))
            .with_state(state.clone());
        let (_, msg) = post_json(
            app,
            "/api/message/send",
            json!({
                "project_slug": project_slug,
                "sender_name": "ExtSender",
                "recipient_names": ["ExtRecipient"],
                "subject": "Extended Test",
                "body_md": "Extended message body",
                "thread_id": "EXT-THREAD-001"
            }),
        )
        .await;

        (project_slug, msg["id"].as_i64().unwrap())
    }

    #[tokio::test]
    async fn test_get_message() {
        let (state, _temp) = create_test_state().await;
        let (_project_slug, message_id) = setup_with_message(&state).await;

        let app = Router::new()
            .route("/api/messages/{message_id}", get(tools::get_message))
            .with_state(state);

        let (status, body) = get_json(app, &format!("/api/messages/{}", message_id)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], message_id);
        assert_eq!(body["subject"], "Extended Test");
    }

    #[tokio::test]
    async fn test_reply_message() {
        let (state, _temp) = create_test_state().await;
        let (project_slug, message_id) = setup_with_message(&state).await;

        let app = Router::new()
            .route("/api/message/reply", post(tools::reply_message))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/message/reply",
            json!({
                "project_slug": project_slug,
                "sender_name": "ExtRecipient",
                "message_id": message_id,
                "body_md": "This is a reply"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["id"].as_i64().unwrap() > 0);
        assert_eq!(body["thread_id"], "EXT-THREAD-001");
    }
}

// =============================================================================
// File Reservation Extended Tests
// =============================================================================

mod file_reservation_extended_tests {
    use super::*;

    async fn setup_with_reservation(state: &AppState) -> (String, String, i64) {
        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "file-res-ext-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create agent
        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "FileResAgent",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        // Create reservation
        let app = Router::new()
            .route(
                "/api/file_reservations/paths",
                post(tools::file_reservation_paths),
            )
            .with_state(state.clone());
        let (_, res) = post_json(
            app,
            "/api/file_reservations/paths",
            json!({
                "project_slug": project_slug,
                "agent_name": "FileResAgent",
                "paths": ["ext-test/**"],
                "exclusive": true,
                "ttl_seconds": 3600
            }),
        )
        .await;

        let res_id = res["granted"].as_array().unwrap()[0]["id"]
            .as_i64()
            .unwrap();
        (project_slug, "FileResAgent".to_string(), res_id)
    }

    #[tokio::test]
    async fn test_renew_file_reservation() {
        let (state, _temp) = create_test_state().await;
        let (_project_slug, _agent_name, reservation_id) = setup_with_reservation(&state).await;

        let app = Router::new()
            .route(
                "/api/file_reservations/renew",
                post(tools::renew_file_reservation),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/file_reservations/renew",
            json!({
                "reservation_id": reservation_id,
                "ttl_seconds": 7200
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["renewed"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_force_release_reservation() {
        let (state, _temp) = create_test_state().await;
        let (_project_slug, _agent_name, reservation_id) = setup_with_reservation(&state).await;

        let app = Router::new()
            .route(
                "/api/file_reservations/force_release",
                post(tools::force_release_reservation),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/file_reservations/force_release",
            json!({
                "reservation_id": reservation_id
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["released"].as_bool().unwrap());
    }
}

// =============================================================================
// Quota Tests
// =============================================================================

mod quota_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_quota_status() {
        let (state, _temp) = create_test_state().await;

        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "quota-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create agent
        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "QuotaAgent",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        let app = Router::new()
            .route("/api/quota/status", post(tools::get_quota_status))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/quota/status",
            json!({
                "project_slug": project_slug,
                "agent_name": "QuotaAgent"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["quota_enabled"].is_boolean());
        assert!(body["attachments_limit_bytes"].as_i64().is_some());
        assert!(body["attachments_usage_bytes"].as_i64().is_some());
    }
}

// =============================================================================
// Overseer Tests
// =============================================================================

mod overseer_tests {
    use super::*;

    #[tokio::test]
    async fn test_send_overseer_message() {
        let (state, _temp) = create_test_state().await;

        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "overseer-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create agent
        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "OverseerTarget",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        let app = Router::new()
            .route("/api/overseer/send", post(tools::send_overseer_message))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/overseer/send",
            json!({
                "project_slug": project_slug,
                "agent_name": "OverseerTarget",
                "subject": "Human Guidance",
                "body_md": "Please review this code"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["sent"].as_bool().unwrap());
        assert!(body["message_id"].as_i64().unwrap() > 0);
    }
}

// =============================================================================
// Macro Extended Tests
// =============================================================================

mod macro_extended_tests {
    use super::*;

    async fn setup_project(state: &AppState) -> String {
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "macro-ext-proj"}),
        )
        .await;
        proj["slug"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_unregister_macro() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_project(&state).await;

        // Register macro first
        let app = Router::new()
            .route("/api/macros/register", post(tools::register_macro))
            .with_state(state.clone());
        post_json(
            app,
            "/api/macros/register",
            json!({
                "project_slug": project_slug,
                "name": "test-unregister-macro",
                "description": "Macro to unregister",
                "steps": [{"action": "test"}]
            }),
        )
        .await;

        // Unregister it
        let app = Router::new()
            .route("/api/macros/unregister", post(tools::unregister_macro))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/macros/unregister",
            json!({
                "project_slug": project_slug,
                "name": "test-unregister-macro"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["deleted"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_macro_start_session() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_project(&state).await;

        let app = Router::new()
            .route(
                "/api/macros/start_session",
                post(tools::macro_start_session),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/macros/start_session",
            json!({
                "project_slug": project_slug,
                "name": "MacroSessionAgent",
                "program": "claude-code",
                "model": "claude-opus-4",
                "patterns": ["macro-session/**"]
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["agent_id"].as_i64().unwrap() > 0);
        assert_eq!(body["agent_name"], "MacroSessionAgent");
    }

    #[tokio::test]
    async fn test_macro_file_reservation_cycle() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_project(&state).await;

        // Create agent
        let app = Router::new()
            .route("/api/agent/register", post(tools::register_agent))
            .with_state(state.clone());
        post_json(
            app,
            "/api/agent/register",
            json!({
                "project_slug": project_slug,
                "name": "FileCycleAgent",
                "program": "test",
                "model": "test"
            }),
        )
        .await;

        let app = Router::new()
            .route(
                "/api/macros/file_reservation_cycle",
                post(tools::macro_file_reservation_cycle),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/macros/file_reservation_cycle",
            json!({
                "project_slug": project_slug,
                "agent_name": "FileCycleAgent",
                "patterns": ["cycle-test/**"],
                "action": "reserve"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["action"], "reserve");
        assert!(body["affected_count"].as_i64().unwrap() >= 0);
    }

    #[tokio::test]
    async fn test_macro_contact_handshake() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_project(&state).await;

        for name in ["HandshakeRequester", "HandshakeTarget"] {
            let app = Router::new()
                .route("/api/agent/register", post(tools::register_agent))
                .with_state(state.clone());
            post_json(
                app,
                "/api/agent/register",
                json!({
                    "project_slug": project_slug,
                    "name": name,
                    "program": "test",
                    "model": "test"
                }),
            )
            .await;
        }

        let app = Router::new()
            .route(
                "/api/macros/contact_handshake",
                post(tools::macro_contact_handshake),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/macros/contact_handshake",
            json!({
                "project_slug": project_slug,
                "requester": "HandshakeRequester",
                "target": "HandshakeTarget"
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["contacts_created"], 1);
        assert!(body["link_ids"].as_array().is_some());
    }
}

// =============================================================================
// Thread Extended Tests
// =============================================================================

mod thread_extended_tests {
    use super::*;

    async fn setup_with_threads(state: &AppState) -> String {
        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "thread-ext-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create agents
        for name in ["ThreadExtSender", "ThreadExtRecipient"] {
            let app = Router::new()
                .route("/api/agent/register", post(tools::register_agent))
                .with_state(state.clone());
            post_json(
                app,
                "/api/agent/register",
                json!({
                    "project_slug": project_slug,
                    "name": name,
                    "program": "test",
                    "model": "test"
                }),
            )
            .await;
        }

        // Create messages in multiple threads
        for i in 1..=2 {
            let app = Router::new()
                .route("/api/message/send", post(tools::send_message))
                .with_state(state.clone());
            post_json(
                app,
                "/api/message/send",
                json!({
                    "project_slug": project_slug,
                    "sender_name": "ThreadExtSender",
                    "recipient_names": ["ThreadExtRecipient"],
                    "subject": format!("Thread {} Message", i),
                    "body_md": format!("Message body {}", i),
                    "thread_id": format!("MULTI-THREAD-{}", i)
                }),
            )
            .await;
        }

        project_slug
    }

    #[tokio::test]
    async fn test_summarize_threads() {
        let (state, _temp) = create_test_state().await;
        let project_slug = setup_with_threads(&state).await;

        let app = Router::new()
            .route("/api/threads/summarize", post(tools::summarize_threads))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/threads/summarize",
            json!({
                "project_slug": project_slug,
                "limit": 10
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.is_array());
    }
}

// =============================================================================
// Precommit Guard Tests
// =============================================================================

mod precommit_guard_tests {
    use super::*;

    #[tokio::test]
    async fn test_install_precommit_guard() {
        let (state, _temp) = create_test_state().await;

        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "guard-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create a temp directory for the repo
        let temp_repo = tempfile::tempdir().unwrap();
        let repo_path = temp_repo.path().to_string_lossy().to_string();
        std::fs::create_dir_all(temp_repo.path().join(".git/hooks")).unwrap();

        let app = Router::new()
            .route(
                "/api/setup/install_guard",
                post(tools::install_precommit_guard),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/setup/install_guard",
            json!({
                "project_slug": project_slug,
                "target_repo_path": repo_path
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["installed"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_uninstall_precommit_guard() {
        let (state, _temp) = create_test_state().await;

        let temp_repo = tempfile::tempdir().unwrap();
        let hooks_dir = temp_repo.path().join(".git/hooks");
        std::fs::create_dir_all(&hooks_dir).unwrap();
        std::fs::write(
            hooks_dir.join("pre-commit"),
            "#!/bin/bash\n# Mouchak Mail Pre-commit Guard\necho 'checking'",
        )
        .unwrap();

        let repo_path = temp_repo.path().to_string_lossy().to_string();

        let app = Router::new()
            .route(
                "/api/setup/uninstall_guard",
                post(tools::uninstall_precommit_guard),
            )
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/setup/uninstall_guard",
            json!({
                "target_repo_path": repo_path
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["uninstalled"].as_bool().unwrap());
    }
}

// =============================================================================
// Metrics Tests
// =============================================================================

mod metrics_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_tool_metrics() {
        let (state, _temp) = create_test_state().await;

        let app = Router::new()
            .route("/api/metrics/tools", get(tools::list_tool_metrics))
            .with_state(state);

        let (status, body) = get_json(app, "/api/metrics/tools").await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.is_array() || body.is_object());
    }

    #[tokio::test]
    async fn test_get_tool_stats() {
        let (state, _temp) = create_test_state().await;

        let app = Router::new()
            .route("/api/metrics/stats", get(tools::get_tool_stats))
            .with_state(state);

        let (status, body) = get_json(app, "/api/metrics/stats").await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.is_array() || body.is_object());
    }

    #[tokio::test]
    async fn test_list_activity() {
        let (state, _temp) = create_test_state().await;

        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "activity-test-proj"}),
        )
        .await;
        let project_id = proj["id"].as_i64().unwrap();

        let app = Router::new()
            .route("/api/activity", get(tools::list_activity))
            .with_state(state);

        let (status, body) =
            get_json(app, &format!("/api/activity?project_id={}", project_id)).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.is_array());
    }
}

// =============================================================================
// Pending Reviews Tests
// =============================================================================

mod pending_reviews_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_pending_reviews() {
        let (state, _temp) = create_test_state().await;

        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "pending-rev-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        // Create agents
        for name in ["ReviewSender", "ReviewRecipient"] {
            let app = Router::new()
                .route("/api/agent/register", post(tools::register_agent))
                .with_state(state.clone());
            post_json(
                app,
                "/api/agent/register",
                json!({
                    "project_slug": project_slug,
                    "name": name,
                    "program": "test",
                    "model": "test"
                }),
            )
            .await;
        }

        // Send message with ack_required
        let app = Router::new()
            .route("/api/message/send", post(tools::send_message))
            .with_state(state.clone());
        post_json(
            app,
            "/api/message/send",
            json!({
                "project_slug": project_slug,
                "sender_name": "ReviewSender",
                "recipient_names": ["ReviewRecipient"],
                "subject": "Needs Review",
                "body_md": "Please review this",
                "ack_required": true
            }),
        )
        .await;

        let app = Router::new()
            .route(
                "/api/messages/pending-reviews",
                get(tools::list_pending_reviews),
            )
            .with_state(state);

        let (status, body) = get_json(app, "/api/messages/pending-reviews").await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["pending_reviews"].is_array());
        assert!(body["total_count"].as_i64().is_some());
    }
}

// =============================================================================
// Project Siblings Tests
// =============================================================================

mod siblings_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_project_siblings() {
        let (state, _temp) = create_test_state().await;

        // Create project
        let app = Router::new()
            .route("/api/project/ensure", post(tools::ensure_project))
            .with_state(state.clone());
        let (_, proj) = post_json(
            app,
            "/api/project/ensure",
            json!({"human_key": "sibling-test-proj"}),
        )
        .await;
        let project_slug = proj["slug"].as_str().unwrap().to_string();

        let app = Router::new()
            .route("/api/project/siblings", post(tools::list_project_siblings))
            .with_state(state);

        let (status, body) = post_json(
            app,
            "/api/project/siblings",
            json!({
                "project_slug": project_slug
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.is_array());
    }
}
