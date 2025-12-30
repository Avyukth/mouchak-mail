//! MCP (Model Context Protocol) HTTP handler module
//!
//! This module provides the HTTP/SSE endpoint for MCP protocol at `/mcp`.
//! It integrates lib-mcp's MouchakMailService with Axum's routing system.

use axum::{
    Router,
    body::Body,
    http::{Request, Response},
    routing::any_service,
};
use mouchak_mail_core::ModelManager;
use mouchak_mail_mcp::tools::MouchakMailService;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager,
    tower::{StreamableHttpServerConfig, StreamableHttpService},
};
use std::sync::Arc;
use tower::ServiceExt;

use crate::AppState;

/// Create the MCP service for the /mcp route
///
/// This creates a tower-compatible service that handles MCP JSON-RPC 2.0 requests
/// over HTTP. By default, uses stateless mode (no session handshake required) for
/// compatibility with clients like NTM that send tools/call without initialize.
///
/// Set MOUCHAK_MCP_STATEFUL=true for SSE streaming (requires initialize handshake).
fn create_mcp_service(mm: ModelManager) -> StreamableHttpService<MouchakMailService> {
    // Create session manager for stateful connections
    let session_manager = Arc::new(LocalSessionManager::default());

    let stateful_mode = std::env::var("MOUCHAK_MCP_STATEFUL")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let config = StreamableHttpServerConfig {
        stateful_mode,
        ..Default::default()
    };

    // Check if worktrees/build-slot tools are enabled via environment
    let worktrees_enabled = std::env::var("WORKTREES_ENABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || std::env::var("GIT_IDENTITY_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

    // Wrap ModelManager in Arc for sharing across connections
    let mm = Arc::new(mm);

    // Create a service factory that creates a new MouchakMailService for each connection.
    // Uses the shared ModelManager to avoid migration conflicts.
    let service_factory = move || -> Result<MouchakMailService, std::io::Error> {
        Ok(MouchakMailService::new_with_mm(
            mm.clone(),
            worktrees_enabled,
        ))
    };

    // Create the StreamableHttpService (tower-compatible)
    StreamableHttpService::new(service_factory, session_manager, config)
}

/// Get the MCP route for integration into the main router
///
/// This returns an Axum Router that handles both GET (SSE stream) and POST (tool calls)
/// on the /mcp endpoint. Uses the ModelManager from AppState to share database connection.
pub fn mcp_routes(mm: ModelManager) -> Router<AppState> {
    let mcp_service = create_mcp_service(mm);

    // Wrap the MCP service to convert body types
    let wrapped_service = tower::service_fn(move |req: Request<Body>| {
        let svc = mcp_service.clone();
        async move {
            // Call the MCP service
            let response = svc.oneshot(req).await?;
            // Convert BoxBody to axum::body::Body
            let (parts, body) = response.into_parts();
            let body = Body::new(body);
            Ok::<_, std::convert::Infallible>(Response::from_parts(parts, body))
        }
    });

    Router::new().route("/mcp", any_service(wrapped_service))
}
