//! MCP Agent Mail - stdio server for Claude Desktop integration
//!
//! This binary exposes the Agent Mail API as MCP tools over stdio,
//! allowing Claude Desktop to interact with the multi-agent messaging system.

use anyhow::Result;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod tools;

use tools::AgentMailService;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (stdout is reserved for MCP)
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive("mcp_stdio=info".parse()?))
        .init();

    tracing::info!("Starting MCP Agent Mail server...");

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
