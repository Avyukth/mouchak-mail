use clap::{Args, Parser, Subcommand};
use lib_common::config::AppConfig;
use lib_mcp::{
    docs::generate_markdown_docs,
    tools::get_tool_schemas,
    run_stdio, run_sse,
};
use tracing::info;

#[derive(Parser)]
#[command(name = "mcp-agent-mail")]
#[command(about = "Unified Server/CLI for Agent Mail")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Log format: plain or json
    #[arg(long, default_value = "plain", global = true)]
    log_format: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a server (HTTP or MCP)
    Serve(ServeArgs),

    /// Check server health
    Health {
        #[arg(short, long, default_value = "http://localhost:8765")]
        url: String,
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

    /// Show version info
    Version,
}

#[derive(Args)]
struct ServeArgs {
    #[command(subcommand)]
    command: ServeCommands,
}

#[derive(Subcommand)]
enum ServeCommands {
    /// Start the HTTP API Server
    Http {
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Start the MCP Server (Stdio or SSE)
    Mcp {
        #[arg(long, default_value = "stdio")]
        transport: String,
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
}

fn setup_tracing(json_logs: bool) -> anyhow::Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt, Layer};

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug,axum=debug,mcp_agent_mail=debug"));

    let layer = if json_logs {
        fmt::layer().json().with_writer(std::io::stderr).boxed()
    } else {
        fmt::layer().pretty().with_writer(std::io::stderr).boxed()
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(layer)
        .try_init()?;
    Ok(())
}

fn load_config() -> AppConfig {
    AppConfig::load().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config file: {}. Using defaults.", e);
        AppConfig::default()
    })
}

async fn handle_serve_http(port: Option<u16>, mut config: AppConfig) -> anyhow::Result<()> {
    if let Some(p) = port {
        config.server.port = p;
    }
    info!("Starting HTTP Server...");
    lib_server::run(config.server).await?;
    Ok(())
}

async fn handle_serve_mcp(transport: String, port: u16, mut config: AppConfig) -> anyhow::Result<()> {
    config.mcp.transport = transport.clone();
    config.mcp.port = port;
    info!("Starting MCP Server ({})", transport);
    if transport == "sse" {
        run_sse(config.mcp).await?;
    } else {
        run_stdio(config.mcp).await?;
    }
    Ok(())
}

async fn handle_health(url: String) -> anyhow::Result<()> {
    info!("Checking health at {}", url);
    let resp = reqwest::get(format!("{}/health", url)).await?;
    if resp.status().is_success() {
        info!("Server is HEALTHY: {}", resp.text().await?);
    } else {
        tracing::error!("Server is UNHEALTHY: Status {}", resp.status());
        std::process::exit(1);
    }
    Ok(())
}

fn handle_schema(format: String, output: Option<String>) -> anyhow::Result<()> {
    let schemas = get_tool_schemas();
    let content = if format == "markdown" || format == "md" {
        generate_markdown_docs(&schemas)
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
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    setup_tracing(cli.log_format == "json")?;
    let config = load_config();

    match cli.command {
        Commands::Serve(args) => match args.command {
            ServeCommands::Http { port } => handle_serve_http(port, config).await?,
            ServeCommands::Mcp { transport, port } => handle_serve_mcp(transport, port, config).await?,
        },
        Commands::Health { url } => handle_health(url).await?,
        Commands::Schema { format, output } => handle_schema(format, output)?,
        Commands::Tools => handle_tools(),
        Commands::Version => println!("mcp-agent-mail v{}", env!("CARGO_PKG_VERSION")),
    }

    Ok(())
}
