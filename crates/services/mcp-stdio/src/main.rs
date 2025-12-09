//! MCP Agent Mail - stdio server for Claude Desktop integration
//!
//! This binary exposes the Agent Mail API as MCP tools over stdio,
//! allowing Claude Desktop to interact with the multi-agent messaging system.

use anyhow::Result;
use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod tools;

use tools::AgentMailService;

#[derive(Parser)]
#[command(name = "mcp-agent-mail")]
#[command(about = "MCP Agent Mail - Multi-agent messaging system")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the MCP server over stdio (default)
    Serve {
        /// Transport mode: stdio or sse
        #[arg(short, long, default_value = "stdio")]
        transport: String,
        /// Port for SSE server (default: 3000)
        #[arg(short, long, default_value = "3000")]
        port: u16,
        /// Host to bind SSE server (default: 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Export JSON schemas for all tools
    Schema {
        /// Output format: json or markdown
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// List all available tools
    Tools,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Serve {
        transport: "stdio".to_string(),
        port: 3000,
        host: "127.0.0.1".to_string(),
    }) {
        Commands::Serve { transport, port, host } => {
            match transport.as_str() {
                "sse" => run_sse_server(&host, port).await,
                _ => run_stdio_server().await,
            }
        }
        Commands::Schema { format, output } => export_schema(&format, output.as_deref()).await,
        Commands::Tools => list_tools().await,
    }
}

async fn run_stdio_server() -> Result<()> {
    // Initialize logging to stderr (stdout is reserved for MCP)
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive("mcp_stdio=info".parse()?))
        .init();

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

async fn run_sse_server(host: &str, port: u16) -> Result<()> {
    use rmcp::transport::streamable_http_server::{
        tower::{StreamableHttpService, StreamableHttpServerConfig},
        session::local::LocalSessionManager,
    };
    use std::net::SocketAddr;
    use std::sync::Arc;

    // Initialize logging to stdout (SSE mode)
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("mcp_stdio=info".parse()?))
        .init();

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    tracing::info!("Starting MCP Agent Mail server (HTTP/SSE mode) on http://{}", addr);

    // Create session manager for stateful connections
    let session_manager = Arc::new(LocalSessionManager::default());

    // Configure the HTTP server
    let config = StreamableHttpServerConfig::default();

    // Create a service factory that creates a new AgentMailService for each connection
    let service_factory = || {
        // Note: AgentMailService::new() is async but the factory needs to be sync
        // We'll use a blocking approach here for simplicity
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            AgentMailService::new().await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })
    };

    // Create the StreamableHttpService (tower-compatible)
    let mcp_service = StreamableHttpService::new(
        service_factory,
        session_manager,
        config,
    );

    tracing::info!("HTTP/SSE MCP endpoints:");
    tracing::info!("  - POST http://{}/mcp (for tool calls)", addr);
    tracing::info!("  - GET  http://{}/mcp (for SSE stream)", addr);

    // Create an Axum app with the MCP service
    let app = axum::Router::new()
        .route("/mcp", axum::routing::any_service(mcp_service));

    // Run the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn export_schema(format: &str, output: Option<&str>) -> Result<()> {
    let schemas = tools::get_tool_schemas();

    let content = match format {
        "markdown" | "md" => generate_markdown_docs(&schemas),
        _ => serde_json::to_string_pretty(&schemas)?,
    };

    if let Some(path) = output {
        std::fs::write(path, &content)?;
        eprintln!("Schema written to {}", path);
    } else {
        println!("{}", content);
    }

    Ok(())
}

async fn list_tools() -> Result<()> {
    let schemas = tools::get_tool_schemas();

    println!("MCP Agent Mail Tools ({} total)\n", schemas.len());
    println!("{:<30} {}", "TOOL", "DESCRIPTION");
    println!("{}", "-".repeat(80));

    for schema in &schemas {
        println!("{:<30} {}", schema.name, schema.description);
    }

    Ok(())
}

fn generate_markdown_docs(schemas: &[tools::ToolSchema]) -> String {
    let mut md = String::from("# MCP Agent Mail - Tool Reference\n\n");
    md.push_str(&format!("Total tools: {}\n\n", schemas.len()));
    md.push_str("## Table of Contents\n\n");

    for schema in schemas {
        md.push_str(&format!("- [{}](#{})\n", schema.name, schema.name.replace('_', "-")));
    }

    md.push_str("\n---\n\n");

    for schema in schemas {
        md.push_str(&format!("## {}\n\n", schema.name));
        md.push_str(&format!("{}\n\n", schema.description));

        if !schema.parameters.is_empty() {
            md.push_str("### Parameters\n\n");
            md.push_str("| Name | Type | Required | Description |\n");
            md.push_str("|------|------|----------|-------------|\n");

            for param in &schema.parameters {
                md.push_str(&format!(
                    "| `{}` | {} | {} | {} |\n",
                    param.name,
                    param.param_type,
                    if param.required { "Yes" } else { "No" },
                    param.description
                ));
            }
            md.push_str("\n");
        }

        md.push_str("### Example\n\n");
        md.push_str("```json\n");
        md.push_str(&format!("{{\n  \"tool\": \"{}\",\n  \"arguments\": {{\n", schema.name));
        for (i, param) in schema.parameters.iter().enumerate() {
            let example_value = match param.param_type.as_str() {
                "string" => "\"example\"".to_string(),
                "integer" | "number" => "1".to_string(),
                "boolean" => "true".to_string(),
                _ => "null".to_string(),
            };
            let comma = if i < schema.parameters.len() - 1 { "," } else { "" };
            md.push_str(&format!("    \"{}\": {}{}\n", param.name, example_value, comma));
        }
        md.push_str("  }\n}\n```\n\n");
        md.push_str("---\n\n");
    }

    md
}
