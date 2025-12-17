use anyhow::Result;
use lib_common::config::McpConfig;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};

pub mod docs;
pub mod tools;
pub use tools::{
    AgentMailService, InvokeMacroParams, ListMacrosParams, RegisterMacroParams,
    UnregisterMacroParams,
};

pub async fn run_stdio(_config: McpConfig) -> Result<()> {
    // Initializing logging to stderr is crucial for MCP stdio transport
    // This might be already handled by the caller (unified binary), but stdio mode specifically
    // requires logs to go to stderr. The unified binary setup_tracing usually does this if not json.
    // However, we should ensure we don't interfere if already set up.
    // But mcp-stdio logic specifically sets a filter.

    // For now, assuming tracing is set up by the caller.

    tracing::info!("Starting MCP Agent Mail server (stdio mode)...");

    // Initialize the service
    let service = AgentMailService::new().await?;

    // Run over stdio
    let transport = (stdin(), stdout());
    let server = service.serve(transport).await?;

    tracing::info!("MCP server initialized, waiting for requests...");

    // Wait for shutdown
    let quit_reason = server.waiting().await?;
    tracing::info!("Server shutting down: {:?}", quit_reason);

    Ok(())
}

pub async fn run_sse(config: McpConfig) -> Result<()> {
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager,
        tower::{StreamableHttpServerConfig, StreamableHttpService},
    };
    use std::net::SocketAddr;
    use std::sync::Arc;

    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    tracing::info!(
        "Starting MCP Agent Mail server (HTTP/SSE mode) on http://{}",
        addr
    );

    // Create session manager for stateful connections
    let session_manager = Arc::new(LocalSessionManager::default());

    // Configure the HTTP server
    let config = StreamableHttpServerConfig::default();

    // Create a service factory that creates a new AgentMailService for each connection
    let service_factory = || {
        // Note: AgentMailService::new() is async but the factory needs to be sync
        // We'll use a blocking approach here for simplicity, or handle via tokio::spawn if structure allows
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            AgentMailService::new()
                .await
                .map_err(|e| std::io::Error::other(e.to_string()))
        })
    };

    // Create the StreamableHttpService (tower-compatible)
    let mcp_service = StreamableHttpService::new(service_factory, session_manager, config);

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

pub fn get_tool_schemas() -> Vec<tools::ToolSchema> {
    tools::get_tool_schemas()
}
