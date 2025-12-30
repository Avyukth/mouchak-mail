use anyhow::Result;
use mouchak_mail_common::config::AppConfig;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};

pub mod docs;
pub mod tools;
pub use tools::{
    InvokeMacroParams, ListMacrosParams, MouchakMailService, RegisterMacroParams,
    UnregisterMacroParams,
};

// COVERAGE: Server startup functions tested via e2e/ and mcp-stdio crate
pub async fn run_stdio(config: AppConfig) -> Result<()> {
    // Initializing logging to stderr is crucial for MCP stdio transport
    // This might be already handled by the caller (unified binary), but stdio mode specifically
    // requires logs to go to stderr. The unified binary setup_tracing usually does this if not json.
    // However, we should ensure we don't interfere if already set up.
    // But mcp-stdio logic specifically sets a filter.

    // For now, assuming tracing is set up by the caller.

    tracing::info!("Starting Mouchak Mail server (stdio mode)...");

    // Initialize the service with worktrees config
    let service = MouchakMailService::new_with_config(config).await?;

    // Run over stdio
    let transport = (stdin(), stdout());
    let server = service.serve(transport).await?;

    tracing::info!("MCP server initialized, waiting for requests...");

    // Wait for shutdown
    let quit_reason = server.waiting().await?;
    tracing::info!("Server shutting down: {:?}", quit_reason);

    Ok(())
}

pub async fn run_sse(config: AppConfig) -> Result<()> {
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager,
        tower::{StreamableHttpServerConfig, StreamableHttpService},
    };
    use std::net::SocketAddr;
    use std::sync::Arc;

    let addr: SocketAddr = format!("0.0.0.0:{}", config.mcp.port).parse()?;
    tracing::info!(
        "Starting Mouchak Mail server (HTTP/SSE mode) on http://{}",
        addr
    );

    // Create session manager for stateful connections
    let session_manager = Arc::new(LocalSessionManager::default());

    let stateful_mode = std::env::var("MOUCHAK_MCP_STATEFUL")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let server_config = StreamableHttpServerConfig {
        stateful_mode,
        ..Default::default()
    };

    // Create a service factory that creates a new MouchakMailService for each connection
    let service_factory = move || {
        // Note: MouchakMailService::new() is async but the factory needs to be sync
        // We'll use a blocking approach here for simplicity, or handle via tokio::spawn if structure allows
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            MouchakMailService::new_with_config(config.clone())
                .await
                .map_err(|e| std::io::Error::other(e.to_string()))
        })
    };

    // Create the StreamableHttpService (tower-compatible)
    let mcp_service = StreamableHttpService::new(service_factory, session_manager, server_config);

    tracing::info!("HTTP/SSE MCP endpoints:");
    tracing::info!("  - POST http://{}/mcp (for tool calls)", addr);
    tracing::info!("  - GET  http://{}/mcp (for SSE stream)", addr);

    // Create an Axum app with the MCP service
    let app = axum::Router::new().route("/mcp", axum::routing::any_service(mcp_service));

    // Run the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

#[allow(clippy::expect_used)] // Signal handler setup is infallible in practice; panic is acceptable
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Signal received, starting graceful shutdown");
}

/// Get tool schemas, conditionally filtering build slot tools.
///
/// When `worktrees_enabled` is false, build slot tools (acquire, release, renew)
/// are excluded from the returned list.
pub fn get_tool_schemas(worktrees_enabled: bool) -> Vec<tools::ToolSchema> {
    tools::get_tool_schemas(worktrees_enabled)
}
