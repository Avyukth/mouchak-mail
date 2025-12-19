use clap::{Args, Parser, Subcommand};
use lib_common::config::AppConfig;
use lib_mcp::{docs::generate_markdown_docs, run_sse, run_stdio, tools::get_tool_schemas};
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
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

    /// Install shell alias and configuration
    Install(InstallArgs),

    /// Manage the background service
    Service(ServiceArgs),

    /// Show version info
    Version,
}

#[derive(Args)]
struct InstallArgs {
    #[command(subcommand)]
    command: InstallCommands,
}

#[derive(Subcommand)]
enum InstallCommands {
    /// Add 'am' shell alias for quick server start
    Alias {
        /// Force overwrite existing alias
        #[arg(long)]
        force: bool,
    },
}

#[derive(Args)]
struct ServiceArgs {
    #[command(subcommand)]
    command: ServiceCommands,
}

#[derive(Subcommand)]
enum ServiceCommands {
    /// Stop the running server on the specified port
    Stop {
        /// Port to stop the server on
        #[arg(short, long, default_value = "8765")]
        port: u16,
    },
    /// Check if server is running
    Status {
        /// Port to check
        #[arg(short, long, default_value = "8765")]
        port: u16,
    },
    /// Restart the server (stop then start)
    Restart {
        /// Port to restart
        #[arg(short, long, default_value = "8765")]
        port: u16,
    },
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
        /// Serve embedded web UI (default: true if compiled with with-web-ui feature)
        #[arg(long, default_value = "true")]
        with_ui: bool,
        /// Disable web UI serving (overrides --with-ui)
        #[arg(long, conflicts_with = "with_ui")]
        no_ui: bool,
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
    use tracing_subscriber::{
        EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt,
    };

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,tower_http=debug,axum=debug,mcp_agent_mail=debug")
    });

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

// ============================================================================
// Port Validation (PORT-6.3)
// ============================================================================

/// Error returned when a port is unavailable.
#[derive(Debug)]
pub struct PortInUseError {
    pub port: u16,
    pub suggestion: String,
}

impl std::fmt::Display for PortInUseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Port {} is already in use.\n\n{}\n",
            self.port, self.suggestion
        )
    }
}

impl std::error::Error for PortInUseError {}

/// Validate that a port is available for binding.
///
/// Attempts to bind to the port briefly to check availability.
/// Returns Ok(()) if the port is available, or a helpful error if not.
///
/// # Arguments
/// * `port` - The port number to validate
///
/// # Returns
/// * `Ok(())` - Port is available
/// * `Err(PortInUseError)` - Port is in use, with helpful suggestions
pub fn validate_port(port: u16) -> Result<(), PortInUseError> {
    let addr = format!("127.0.0.1:{}", port);

    match TcpListener::bind(&addr) {
        Ok(_listener) => {
            // Port is available - listener will be dropped and port released
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            let alt_port = if port < 65535 { port + 1 } else { port - 1 };

            let suggestion = format!(
                "To find what's using port {}:\n\
                 \x20 lsof -i :{}\n\n\
                 To kill the process:\n\
                 \x20 lsof -ti :{} | xargs kill\n\n\
                 Or use an alternative port:\n\
                 \x20 mcp-agent-mail serve http --port {}",
                port, port, port, alt_port
            );

            Err(PortInUseError { port, suggestion })
        }
        Err(e) => {
            // Other errors (permission denied, etc.)
            let suggestion = format!(
                "Failed to bind to port {}: {}\n\n\
                 Try running with a different port:\n\
                 \x20 mcp-agent-mail serve http --port 8766",
                port, e
            );

            Err(PortInUseError { port, suggestion })
        }
    }
}

async fn handle_serve_http(
    port: Option<u16>,
    with_ui: bool,
    no_ui: bool,
    mut config: AppConfig,
) -> anyhow::Result<()> {
    if let Some(p) = port {
        config.server.port = p;
    }
    // --no-ui takes precedence, otherwise use --with-ui value
    config.server.serve_ui = !no_ui && with_ui;

    // Validate port availability before starting server
    if let Err(e) = validate_port(config.server.port) {
        eprintln!("\n{}", e);
        std::process::exit(1);
    }

    info!("Starting HTTP Server on port {}...", config.server.port);
    lib_server::run(config.server).await?;
    Ok(())
}

async fn handle_serve_mcp(
    transport: String,
    port: u16,
    mut config: AppConfig,
) -> anyhow::Result<()> {
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
    // Show all tools in documentation (worktrees_enabled=true)
    let schemas = get_tool_schemas(true);
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
    // Show all tools in documentation (worktrees_enabled=true)
    let schemas = get_tool_schemas(true);
    println!("MCP Agent Mail Tools ({} total)\n", schemas.len());
    println!("{:<30} DESCRIPTION", "TOOL");
    println!("{}", "-".repeat(80));
    for schema in schemas {
        println!("{:<30} {}", schema.name, schema.description);
    }
}

// ============================================================================
// Install Command Handlers (PORT-6.1)
// ============================================================================

/// Detect user's shell and return the appropriate rc file path.
fn detect_shell_rc() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let home_path = PathBuf::from(&home);

    // Check SHELL environment variable first
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.ends_with("zsh") {
            return Some(home_path.join(".zshrc"));
        } else if shell.ends_with("bash") {
            return Some(home_path.join(".bashrc"));
        } else if shell.ends_with("fish") {
            return Some(home_path.join(".config/fish/config.fish"));
        }
    }

    // Fallback: check which rc files exist
    let zshrc = home_path.join(".zshrc");
    if zshrc.exists() {
        return Some(zshrc);
    }

    let bashrc = home_path.join(".bashrc");
    if bashrc.exists() {
        return Some(bashrc);
    }

    // Default to .profile for POSIX shells
    Some(home_path.join(".profile"))
}

/// Check if the 'am' alias marker already exists in the rc file.
fn alias_marker_exists(rc_path: &PathBuf) -> bool {
    if let Ok(contents) = std::fs::read_to_string(rc_path) {
        contents.contains("# >>> MCP Agent Mail alias")
    } else {
        false
    }
}

/// Check if a different 'am' alias exists (not managed by us).
fn other_am_alias_exists(rc_path: &PathBuf) -> bool {
    if let Ok(contents) = std::fs::read_to_string(rc_path) {
        // Check for any 'alias am=' that isn't in our managed block
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("alias am=") && !contents.contains("# >>> MCP Agent Mail alias")
            {
                return true;
            }
        }
    }
    false
}

/// Generate the alias snippet for a given shell type.
fn generate_alias_snippet(rc_path: &Path) -> &'static str {
    let is_fish = rc_path
        .to_string_lossy()
        .contains(".config/fish/config.fish");

    if is_fish {
        // Fish shell uses different syntax
        r#"
# >>> MCP Agent Mail alias
function am
    mcp-agent-mail serve http
end
# <<< MCP Agent Mail alias
"#
    } else {
        // Bash/Zsh syntax
        r#"
# >>> MCP Agent Mail alias
alias am='mcp-agent-mail serve http'
# <<< MCP Agent Mail alias
"#
    }
}

/// Handle the 'install alias' command.
fn handle_install_alias(force: bool) -> anyhow::Result<()> {
    // Detect shell rc file
    let rc_path = detect_shell_rc().ok_or_else(|| {
        anyhow::anyhow!("Could not detect shell configuration file. Set HOME environment variable.")
    })?;

    println!("Detected shell config: {}", rc_path.display());

    // Check if our marker already exists
    if alias_marker_exists(&rc_path) {
        if force {
            println!("Updating existing 'am' alias...");
            // Remove old marker block and re-add
            if let Ok(contents) = std::fs::read_to_string(&rc_path) {
                let mut new_contents = String::new();
                let mut in_block = false;

                for line in contents.lines() {
                    if line.contains("# >>> MCP Agent Mail alias") {
                        in_block = true;
                        continue;
                    }
                    if line.contains("# <<< MCP Agent Mail alias") {
                        in_block = false;
                        continue;
                    }
                    if !in_block {
                        new_contents.push_str(line);
                        new_contents.push('\n');
                    }
                }

                // Append new alias
                new_contents.push_str(generate_alias_snippet(&rc_path));

                std::fs::write(&rc_path, new_contents)?;
                println!("✓ Updated 'am' alias in {}", rc_path.display());
            }
        } else {
            println!("✓ 'am' alias already installed in {}", rc_path.display());
            println!("  Use --force to update the alias.");
        }
        return Ok(());
    }

    // Check for conflicting alias
    if other_am_alias_exists(&rc_path) && !force {
        println!(
            "⚠ An existing 'am' alias was found in {}",
            rc_path.display()
        );
        println!("  Use --force to overwrite it.");
        return Ok(());
    }

    // Ensure parent directory exists (for fish)
    if let Some(parent) = rc_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Append alias to rc file
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_path)?;

    file.write_all(generate_alias_snippet(&rc_path).as_bytes())?;

    println!("✓ Added 'am' alias to {}", rc_path.display());
    println!();
    println!("To use the alias now, run:");
    println!("  source {}", rc_path.display());
    println!();
    println!("Or open a new terminal, then run:");
    println!("  am");
    println!();
    println!("This starts the HTTP server on port 8765.");

    Ok(())
}

// ============================================================================
// Service Command Handlers (PORT-6.2)
// ============================================================================

/// Find the PID of the process listening on a given port.
/// Uses lsof on macOS/Linux.
fn find_pid_on_port(port: u16) -> Option<u32> {
    let output = std::process::Command::new("lsof")
        .args(["-ti", &format!(":{}", port)])
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // lsof -ti returns PIDs, one per line
        stdout
            .lines()
            .next()
            .and_then(|s| s.trim().parse::<u32>().ok())
    } else {
        None
    }
}

/// Kill a process by PID.
fn kill_process(pid: u32) -> std::io::Result<()> {
    let status = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "Failed to kill process {}",
            pid
        )))
    }
}

/// Handle the 'service stop' command.
fn handle_service_stop(port: u16) -> anyhow::Result<()> {
    println!("Stopping server on port {}...", port);

    if let Some(pid) = find_pid_on_port(port) {
        println!("Found process {} on port {}", pid, port);
        kill_process(pid)?;
        println!("✓ Stopped server (PID {})", pid);
    } else {
        println!("No server running on port {}", port);
    }

    Ok(())
}

/// Handle the 'service status' command.
async fn handle_service_status(port: u16) -> anyhow::Result<()> {
    // Try to connect to health endpoint
    let health_url = format!("http://127.0.0.1:{}/health", port);

    match reqwest::get(&health_url).await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            println!("✓ Server RUNNING on port {}", port);
            println!("  Health: {}", body);

            if let Some(pid) = find_pid_on_port(port) {
                println!("  PID: {}", pid);
            }
        }
        _ => {
            if let Some(pid) = find_pid_on_port(port) {
                println!(
                    "⚠ Process {} is on port {} but not responding to health checks",
                    pid, port
                );
            } else {
                println!("✗ No server running on port {}", port);
            }
        }
    }

    Ok(())
}

/// Handle the 'service restart' command.
async fn handle_service_restart(port: u16, config: AppConfig) -> anyhow::Result<()> {
    println!("Restarting server on port {}...", port);

    // Stop existing server
    if let Some(pid) = find_pid_on_port(port) {
        println!("Stopping existing server (PID {})...", pid);
        kill_process(pid)?;
        // Wait a moment for the port to be released
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Start new server
    println!("Starting server on port {}...", port);
    handle_serve_http(Some(port), true, false, config).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    setup_tracing(cli.log_format == "json")?;
    let config = load_config();

    match cli.command {
        Commands::Serve(args) => match args.command {
            ServeCommands::Http {
                port,
                with_ui,
                no_ui,
            } => handle_serve_http(port, with_ui, no_ui, config).await?,
            ServeCommands::Mcp { transport, port } => {
                handle_serve_mcp(transport, port, config).await?
            }
        },
        Commands::Health { url } => handle_health(url).await?,
        Commands::Schema { format, output } => handle_schema(format, output)?,
        Commands::Tools => handle_tools(),
        Commands::Install(args) => match args.command {
            InstallCommands::Alias { force } => handle_install_alias(force)?,
        },
        Commands::Service(args) => match args.command {
            ServiceCommands::Stop { port } => handle_service_stop(port)?,
            ServiceCommands::Status { port } => handle_service_status(port).await?,
            ServiceCommands::Restart { port } => handle_service_restart(port, config).await?,
        },
        Commands::Version => println!("mcp-agent-mail v{}", env!("CARGO_PKG_VERSION")),
    }

    Ok(())
}
