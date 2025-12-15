use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use lib_mcp::{run_stdio, run_sse, tools::get_tool_schemas};
use lib_common::config::McpConfig;

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
        #[arg(short, long, default_value = "stdio")]
        transport: String,
        #[arg(short, long, default_value = "3000")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Export JSON schemas for all tools
    Schema {
        #[arg(short, long, default_value = "json")]
        format: String,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// List all available tools
    Tools,
}

fn setup_logging() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive("mcp_stdio=info".parse()?))
        .init();
    Ok(())
}

async fn handle_serve(transport: String, port: u16) -> Result<()> {
    setup_logging()?;
    let config = McpConfig { transport: transport.clone(), port };
    if transport == "sse" {
        run_sse(config).await
    } else {
        run_stdio(config).await
    }
}

fn handle_schema(format: String, output: Option<String>) -> Result<()> {
    let schemas = get_tool_schemas();
    let content = if format == "markdown" || format == "md" {
        lib_mcp::docs::generate_markdown_docs(&schemas)
    } else {
        serde_json::to_string_pretty(&schemas)?
    };
    if let Some(path) = output {
        std::fs::write(&path, &content)?;
        eprintln!("Schema written to {}", path);
    } else {
        println!("{}", content);
    }
    Ok(())
}

fn handle_tools() {
    let schemas = get_tool_schemas();
    println!("MCP Agent Mail Tools ({} total)\n", schemas.len());
    println!("{:<30} DESCRIPTION", "TOOL");
    println!("{}", "-".repeat(80));
    for schema in schemas {
        println!("{:<30} {}", schema.name, schema.description);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cmd = cli.command.unwrap_or(Commands::Serve {
        transport: "stdio".to_string(),
        port: 3000,
        host: "127.0.0.1".to_string(),
    });

    match cmd {
        Commands::Serve { transport, port, .. } => handle_serve(transport, port).await,
        Commands::Schema { format, output } => handle_schema(format, output),
        Commands::Tools => { handle_tools(); Ok(()) }
    }
}
