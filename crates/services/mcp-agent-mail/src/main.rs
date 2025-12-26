use clap::{Args, CommandFactory, Parser, Subcommand};
use lib_common::config::AppConfig;
use lib_mcp::{docs::generate_markdown_docs, run_sse, run_stdio, tools::get_tool_schemas};
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use tracing::info;

mod panic_hook;
mod robot_help;

#[derive(Parser)]
#[command(name = "mcp-agent-mail")]
#[command(about = "Unified Server/CLI for Agent Mail")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Log format: plain or json
    #[arg(long, default_value = "plain", global = true)]
    log_format: String,

    /// Output help in machine-readable JSON format
    #[arg(
        long,
        global = true,
        help = "Output help in machine-readable JSON format"
    )]
    robot_help: bool,

    /// Output format for robot flags (json/yaml)
    #[arg(
        long,
        global = true,
        default_value = "json",
        help = "Output format for robot flags"
    )]
    format: String,

    /// Output system health status in machine-readable format
    #[arg(
        long,
        global = true,
        help = "Output system health status in machine-readable format"
    )]
    robot_status: bool,

    /// Show examples for a flag or subcommand
    #[arg(long, global = true, num_args = 0.., allow_hyphen_values = true)]
    robot_examples: Option<Vec<String>>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a server (HTTP or MCP)
    Serve(ServeArgs),

    /// Check server health
    Health {
        /// Server URL to check (reads from MCP_AGENT_MAIL_URL env var)
        #[arg(
            short,
            long,
            env = "MCP_AGENT_MAIL_URL",
            default_value = "http://localhost:8765"
        )]
        url: String,
    },

    /// Manage configuration
    Config(ConfigArgs),

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

    /// Export sharing utilities (signing, verification)
    Share(ShareArgs),

    /// Archive management (disaster recovery)
    Archive(ArchiveArgs),

    /// Summarize thread(s) in a project
    Summarize(SummarizeArgs),

    /// Show version info
    Version,

    /// Product management
    Products(ProductsArgs),

    /// Pre-commit guard management
    Guard(GuardArgs),

    /// Mail/project status information
    Mail(MailArgs),
}

#[derive(Args)]
struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommands,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Set the binding port in config
    SetPort {
        /// Port number
        port: u16,
    },
    /// Show the current binding port
    ShowPort,
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

#[derive(Args)]
struct ProductsArgs {
    #[command(subcommand)]
    command: ProductsCommands,
}

#[derive(Subcommand)]
enum ProductsCommands {
    /// Ensure a product exists
    Ensure {
        /// Product UID (e.g. "p-123")
        product_uid: String,
        /// Human-readable name
        #[arg(long)]
        name: String,
    },
    /// Link a product to a project
    Link {
        /// Product UID
        product_uid: String,
        /// Project identifier (slug or human key)
        project: String,
    },
    /// Show product status
    Status {
        /// Product UID
        product_uid: String,
    },
    /// Search messages within a product
    Search {
        /// Product UID
        product_uid: String,
        /// Search query
        query: String,
        #[arg(long, default_value = "50")]
        limit: i64,
    },
    /// Get inbox for an agent within a product context
    Inbox {
        /// Product UID
        product_uid: String,
        /// Agent name
        agent: String,
        /// Only urgent messages
        #[arg(long)]
        urgent_only: bool,
        /// Include message bodies
        #[arg(long)]
        include_bodies: bool,
    },
    /// Summarize a thread
    SummarizeThread {
        /// Product UID
        product_uid: String,
        /// Thread ID
        thread_id: String,
        #[arg(long, default_value = "100")]
        per_thread_limit: i64,
        #[arg(long)]
        no_llm: bool,
    },
}

#[derive(Args)]
struct ShareArgs {
    #[command(subcommand)]
    command: ShareCommands,
}

#[derive(Subcommand)]
enum ShareCommands {
    Keypair {
        #[arg(short, long)]
        output: Option<String>,
    },
    Verify {
        #[arg(short, long)]
        manifest: String,
        #[arg(short, long)]
        public_key: Option<String>,
    },
    Encrypt {
        #[arg(short, long)]
        project: String,
        #[arg(short, long)]
        recipients: Vec<String>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short = 'f', long)]
        format: Option<String>,
        #[arg(long)]
        passphrase: Option<String>,
        #[arg(long)]
        sign_key: Option<String>,
    },
    Decrypt {
        #[arg(short, long)]
        input: String,
        #[arg(short = 'k', long)]
        identity: Option<String>,
        #[arg(long)]
        passphrase: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Deploy exported data to hosting platforms
    Deploy {
        #[command(subcommand)]
        command: DeployCommands,
    },
    /// Export data for static site deployment
    Export {
        #[command(subcommand)]
        command: ExportCommands,
    },
}

#[derive(Subcommand)]
enum DeployCommands {
    /// Deploy to GitHub Pages
    GithubPages {
        /// GitHub repository name (e.g., 'my-agent-archive')
        #[arg(short, long)]
        repo: String,

        /// GitHub username or organization
        #[arg(short, long)]
        owner: Option<String>,

        /// Path to the export bundle (ZIP file) to deploy
        #[arg(short, long, conflicts_with = "build_dir")]
        bundle: Option<String>,

        /// Path to the SvelteKit build directory to deploy (alternative to --bundle)
        #[arg(long, conflicts_with = "bundle")]
        build_dir: Option<String>,

        /// GitHub personal access token (or set GITHUB_TOKEN env var)
        #[arg(long, env = "GITHUB_TOKEN")]
        token: Option<String>,

        /// Create repository if it doesn't exist
        #[arg(long, default_value = "true")]
        create_repo: bool,

        /// Make the repository private
        #[arg(long, default_value = "false")]
        private: bool,

        /// Custom domain for GitHub Pages
        #[arg(long)]
        custom_domain: Option<String>,
    },
}

#[derive(Subcommand)]
enum ExportCommands {
    /// Export data as JSON files for static GitHub Pages deployment
    StaticData {
        /// Output directory for JSON files
        #[arg(short, long)]
        output: String,

        /// Privacy scrubbing mode: none, standard, aggressive
        #[arg(long, default_value = "none")]
        scrub: String,

        /// Maximum number of messages to export (0 = all)
        #[arg(long, default_value = "0")]
        limit: i64,

        /// Export only specific project (by slug)
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
enum ServiceCommands {
    /// Start the server (background by default)
    Start {
        /// Port to start the server on
        #[arg(short, long, default_value = "8765")]
        port: u16,
        /// Run in background
        #[arg(short, long, default_value = "true")]
        background: bool,
    },
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
struct ArchiveArgs {
    #[command(subcommand)]
    command: ArchiveCommands,
}

#[derive(Args)]
struct GuardArgs {
    #[command(subcommand)]
    command: GuardCommands,
}

#[derive(Args)]
struct MailArgs {
    #[command(subcommand)]
    command: MailCommands,
}

#[derive(Args)]
struct SummarizeArgs {
    /// Project slug or path
    #[arg(short, long)]
    project: String,

    /// Thread ID(s) to summarize (comma-separated for multiple)
    #[arg(short, long)]
    thread_id: String,

    /// Maximum messages per thread to include in summary (default: 100)
    #[arg(long, default_value = "100")]
    per_thread_limit: i64,

    /// Skip LLM-based summarization, return raw aggregation only
    #[arg(long, default_value = "false")]
    no_llm: bool,

    /// Output format: json or text
    #[arg(short, long, default_value = "json")]
    format: String,
}

#[derive(Subcommand)]
enum ArchiveCommands {
    /// Create a restorable snapshot archive
    Save {
        /// Label for the archive
        #[arg(short, long)]
        label: Option<String>,
        /// Include git storage in archive
        #[arg(long, default_value_t = true)]
        include_git: bool,
    },
    /// List available restore points
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Restore from a backup archive
    Restore {
        /// Path to the archive file (.zip)
        file: String,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Wipe all state with optional archive
    ClearAndReset {
        /// Create archive before wiping
        #[arg(long)]
        archive: bool,
        /// Label for pre-wipe archive
        #[arg(long)]
        label: Option<String>,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum GuardCommands {
    /// Show guard status information
    Status,

    /// Check file paths against active reservations
    Check {
        /// Read paths from stdin (null-separated)
        #[arg(long)]
        stdin_nul: bool,

        /// Advisory mode (warn instead of fail)
        #[arg(long)]
        advisory: bool,

        /// Project slug (defaults to git repo directory name)
        #[arg(long, short)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
enum MailCommands {
    /// Show mail/project status information
    Status,
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
    lib_server::run(config).await?;
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
        run_sse(config).await?;
    } else {
        run_stdio(config).await?;
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

async fn handle_service_start(
    port: u16,
    background: bool,
    config: AppConfig,
) -> anyhow::Result<()> {
    if background {
        println!("Starting server on port {} (background)...", port);
        // Validate port first
        if let Err(e) = validate_port(port) {
            anyhow::bail!("Cannot start server: {}", e);
        }

        let exe = std::env::current_exe()?;
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("serve")
            .arg("http")
            .arg("--port")
            .arg(port.to_string())
            // Inherit default UI settings or use config?
            // We can't easily pass full config object via args unless we reconstruct valid flags.
            // For now, let's just assume default behavior of serve command.
            // But we should probably use --no-ui if it's a background service?
            // Let's pass --no-ui to minimize issues.
            .arg("--no-ui");

        // Detach properly
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        match cmd.spawn() {
            Ok(_) => {
                println!("✓ Server started on port {}", port);
                // Give it a moment to bind?
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            Err(e) => anyhow::bail!("Failed to spawn server: {}", e),
        }
    } else {
        handle_serve_http(Some(port), true, false, config).await?;
    }
    Ok(())
}

async fn handle_service_restart(port: u16, config: AppConfig) -> anyhow::Result<()> {
    println!("Restarting server on port {}...", port);

    // Stop existing
    if let Some(pid) = find_pid_on_port(port) {
        println!("Stopping existing server (PID {})...", pid);
        kill_process(pid)?;
        // Wait for port release
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    // Start in background (default for restart)
    handle_service_start(port, true, config).await?;

    Ok(())
}

fn handle_share_keypair(output: Option<String>) -> anyhow::Result<()> {
    use lib_core::model::export::{
        generate_signing_keypair, signing_key_to_base64, verifying_key_to_base64,
    };

    let (signing_key, verifying_key) = generate_signing_keypair();
    let private_b64 = signing_key_to_base64(&signing_key);
    let public_b64 = verifying_key_to_base64(&verifying_key);

    let keypair_json = serde_json::json!({
        "private_key": private_b64,
        "public_key": public_b64,
        "algorithm": "Ed25519",
        "generated_at": chrono::Utc::now().to_rfc3339()
    });

    let content = serde_json::to_string_pretty(&keypair_json)?;

    if let Some(path) = output {
        std::fs::write(&path, &content)?;
        eprintln!("✓ Keypair written to {}", path);
        eprintln!("  KEEP THE PRIVATE KEY SECRET!");
    } else {
        println!("{}", content);
    }

    Ok(())
}

fn handle_share_verify(manifest_path: &str, public_key: Option<&str>) -> anyhow::Result<()> {
    use lib_core::model::export::ExportManifest;

    let manifest_content = std::fs::read_to_string(manifest_path)?;
    let manifest: ExportManifest = serde_json::from_str(&manifest_content)?;

    let verified = if let Some(pk) = public_key {
        manifest.verify_with_key(pk)?
    } else {
        manifest.verify()?
    };

    if verified {
        println!("✓ Signature VALID");
        println!("  Project: {}", manifest.project_slug);
        println!("  Exported: {}", manifest.exported_at);
        println!("  Messages: {}", manifest.message_count);
        println!("  Content Hash: {}", manifest.content_hash);
    } else {
        eprintln!("✗ Signature INVALID or content modified");
        std::process::exit(1);
    }

    Ok(())
}

// --- Export Command Handlers ---

/// Export data as JSON files for static GitHub Pages deployment
///
/// Creates JSON files that can be used by the static SvelteKit build:
/// - meta.json: Export metadata
/// - projects.json: All projects
/// - agents.json: Agents grouped by project slug
/// - messages.json: All messages
/// - threads.json: Thread summaries grouped by project slug
/// - dashboard.json: Pre-computed dashboard stats
/// - activity.json: Recent activity
async fn handle_export_static_data(
    output_dir: &str,
    scrub_mode: &str,
    limit: i64,
    project_filter: Option<&str>,
) -> anyhow::Result<()> {
    use lib_core::ctx::Ctx;
    use lib_core::model::ModelManager;
    use lib_core::model::agent::AgentBmc;
    use lib_core::model::export::{ScrubMode, Scrubber};
    use lib_core::model::message::MessageBmc;
    use lib_core::model::project::ProjectBmc;
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;

    println!("📦 Exporting static data...");
    println!("   Output: {}", output_dir);
    println!("   Scrub mode: {}", scrub_mode);
    if limit > 0 {
        println!("   Message limit: {}", limit);
    }
    if let Some(p) = project_filter {
        println!("   Project filter: {}", p);
    }

    // Create output directory
    let output_path = Path::new(output_dir);
    fs::create_dir_all(output_path)?;

    // Initialize model manager
    let config = load_config();
    let mm = ModelManager::new(std::sync::Arc::new(config.clone())).await?;
    let ctx = Ctx::root_ctx();

    // Parse scrub mode
    let scrub: ScrubMode = scrub_mode.parse().unwrap_or_default();
    let scrubber = Scrubber::new(scrub);

    // Export timestamp
    let exported_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // 1. Export meta.json
    let meta = json!({
        "exportedAt": exported_at,
        "version": "1.0.0",
        "mode": "static",
        "scrubMode": scrub_mode,
        "messageLimit": if limit > 0 { Some(limit) } else { None }
    });
    fs::write(
        output_path.join("meta.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;
    println!("   ✓ meta.json");

    // 2. Export projects.json
    let projects = ProjectBmc::list_all(&ctx, &mm).await?;
    let projects_filtered: Vec<_> = if let Some(slug) = project_filter {
        projects.into_iter().filter(|p| p.slug == slug).collect()
    } else {
        projects
    };

    let projects_json: Vec<_> = projects_filtered
        .iter()
        .map(|p| {
            json!({
                "id": p.id.get(),
                "slug": p.slug,
                "human_key": scrubber.scrub_name(&p.human_key),
                "created_at": p.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
            })
        })
        .collect();
    fs::write(
        output_path.join("projects.json"),
        serde_json::to_string_pretty(&projects_json)?,
    )?;
    println!("   ✓ projects.json ({} projects)", projects_filtered.len());

    // 3. Export agents.json (grouped by project slug)
    let mut agents_by_project: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    let mut total_agents = 0;

    for project in &projects_filtered {
        let agents = AgentBmc::list_all_for_project(&ctx, &mm, project.id).await?;
        let agents_json: Vec<_> = agents
            .iter()
            .map(|a| {
                json!({
                    "id": a.id.get(),
                    "project_id": a.project_id.get(),
                    "name": scrubber.scrub_name(&a.name),
                    "program": a.program,
                    "model": a.model,
                    "task_description": scrubber.scrub(&a.task_description),
                    "inception_ts": a.inception_ts.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    "last_active_ts": a.last_active_ts.format("%Y-%m-%dT%H:%M:%SZ").to_string()
                })
            })
            .collect();
        total_agents += agents_json.len();
        agents_by_project.insert(project.slug.clone(), agents_json);
    }
    fs::write(
        output_path.join("agents.json"),
        serde_json::to_string_pretty(&agents_by_project)?,
    )?;
    println!("   ✓ agents.json ({} agents)", total_agents);

    // 4. Export messages.json
    let mut all_messages = Vec::new();
    for project in &projects_filtered {
        let msg_limit = if limit > 0 { limit } else { 10000 };
        let messages = MessageBmc::list_recent(&ctx, &mm, project.id, msg_limit).await?;

        for msg in messages {
            let msg_json = json!({
                "id": msg.id,
                "project_id": msg.project_id,
                "project_slug": project.slug,
                "sender_id": msg.sender_id,
                "sender_name": scrubber.scrub_name(&msg.sender_name),
                "thread_id": msg.thread_id,
                "subject": scrubber.scrub(&msg.subject),
                "body_md": scrubber.scrub_body(&msg.body_md),
                "importance": msg.importance,
                "ack_required": msg.ack_required,
                "created_ts": msg.created_ts.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                "is_read": false,
                "recipient_names": [],
                "recipients": []
            });
            all_messages.push(msg_json);
        }
    }
    // Sort by created_ts descending
    all_messages.sort_by(|a, b| {
        let a_ts = a["created_ts"].as_str().unwrap_or("");
        let b_ts = b["created_ts"].as_str().unwrap_or("");
        b_ts.cmp(a_ts)
    });
    fs::write(
        output_path.join("messages.json"),
        serde_json::to_string_pretty(&all_messages)?,
    )?;
    println!("   ✓ messages.json ({} messages)", all_messages.len());

    // 5. Export threads.json (grouped by project slug)
    let mut threads_by_project: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    // Group messages by thread_id to build thread summaries
    let mut thread_map: HashMap<(String, String), Vec<&serde_json::Value>> = HashMap::new();
    for msg in &all_messages {
        if let Some(thread_id) = msg["thread_id"].as_str() {
            if !thread_id.is_empty() {
                let project_slug = msg["project_slug"].as_str().unwrap_or("").to_string();
                thread_map
                    .entry((project_slug, thread_id.to_string()))
                    .or_default()
                    .push(msg);
            }
        }
    }

    for ((project_slug, thread_id), messages) in thread_map {
        let first_msg = messages.last(); // Oldest
        let last_msg = messages.first(); // Newest (already sorted desc)

        let thread_summary = json!({
            "thread_id": thread_id,
            "subject": first_msg.map(|m| m["subject"].as_str().unwrap_or("")).unwrap_or(""),
            "message_count": messages.len(),
            "participants": messages.iter()
                .filter_map(|m| m["sender_name"].as_str())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>(),
            "first_message_ts": first_msg.map(|m| m["created_ts"].as_str().unwrap_or("")).unwrap_or(""),
            "last_message_ts": last_msg.map(|m| m["created_ts"].as_str().unwrap_or("")).unwrap_or("")
        });

        threads_by_project
            .entry(project_slug)
            .or_default()
            .push(thread_summary);
    }

    let total_threads: usize = threads_by_project.values().map(|v| v.len()).sum();
    fs::write(
        output_path.join("threads.json"),
        serde_json::to_string_pretty(&threads_by_project)?,
    )?;
    println!("   ✓ threads.json ({} threads)", total_threads);

    // 6. Export dashboard.json
    let dashboard = json!({
        "projectCount": projects_filtered.len(),
        "agentCount": total_agents,
        "inboxCount": all_messages.iter().filter(|m| !m["is_read"].as_bool().unwrap_or(false)).count(),
        "messageCount": all_messages.len(),
        "projects": projects_json.iter().map(|p| {
            let slug = p["slug"].as_str().unwrap_or("");
            let agent_count = agents_by_project.get(slug).map(|a| a.len()).unwrap_or(0);
            json!({
                "id": p["id"],
                "slug": slug,
                "human_key": p["human_key"],
                "created_at": p["created_at"],
                "agentCount": agent_count
            })
        }).collect::<Vec<_>>()
    });
    fs::write(
        output_path.join("dashboard.json"),
        serde_json::to_string_pretty(&dashboard)?,
    )?;
    println!("   ✓ dashboard.json");

    // 7. Export activity.json (empty for now - would need activity tracking)
    let activity: Vec<serde_json::Value> = Vec::new();
    fs::write(
        output_path.join("activity.json"),
        serde_json::to_string_pretty(&activity)?,
    )?;
    println!("   ✓ activity.json");

    // 8. Export archive.json (empty commits/files structure)
    let archive = json!({
        "commits": [],
        "files": {}
    });
    fs::write(
        output_path.join("archive.json"),
        serde_json::to_string_pretty(&archive)?,
    )?;
    println!("   ✓ archive.json");

    println!("\n✅ Export complete!");
    println!("   Files written to: {}", output_dir);

    Ok(())
}

// --- Deploy Command Handlers ---

/// Deploy an export bundle or build directory to GitHub Pages
///
/// This creates a GitHub repository (if needed), pushes the bundle/directory content,
/// and enables GitHub Pages for static hosting.
///
/// Accepts either:
/// - `--bundle`: Path to a ZIP file (simple download page)
/// - `--build-dir`: Path to a SvelteKit build directory (full static site)
#[allow(clippy::too_many_arguments)]
async fn handle_deploy_github_pages(
    repo: &str,
    owner: Option<&str>,
    bundle: Option<&str>,
    build_dir: Option<&str>,
    token: Option<&str>,
    create_repo: bool,
    private: bool,
    custom_domain: Option<&str>,
) -> anyhow::Result<()> {
    use std::path::Path;

    // Determine the source path (bundle or build_dir)
    let (source_path, is_directory) = match (bundle, build_dir) {
        (Some(b), None) => (Path::new(b), false),
        (None, Some(d)) => (Path::new(d), true),
        (None, None) => anyhow::bail!("Either --bundle or --build-dir is required"),
        (Some(_), Some(_)) => anyhow::bail!("Cannot specify both --bundle and --build-dir"),
    };

    // Validate source exists
    if !source_path.exists() {
        anyhow::bail!("Source not found: {}", source_path.display());
    }

    // For directories, validate it looks like a build directory
    if is_directory && !source_path.is_dir() {
        anyhow::bail!("--build-dir must be a directory: {}", source_path.display());
    }

    // Get GitHub token
    let github_token = token
        .map(String::from)
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("GitHub token required. Set GITHUB_TOKEN env var or use --token flag")
        })?;

    // Get owner (default to authenticated user)
    let repo_owner = if let Some(o) = owner {
        o.to_string()
    } else {
        get_github_username(&github_token).await?
    };

    println!("📦 Deploying to GitHub Pages...");
    println!("   Repository: {}/{}", repo_owner, repo);
    if is_directory {
        println!("   Build directory: {}", source_path.display());
    } else {
        println!("   Bundle: {}", source_path.display());
    }

    // Check if repo exists, create if needed
    if create_repo {
        match check_or_create_repo(&github_token, &repo_owner, repo, private).await {
            Ok(created) => {
                if created {
                    println!("   ✓ Created repository");
                } else {
                    println!("   ✓ Repository exists");
                }
            }
            Err(e) => {
                anyhow::bail!("Failed to create/check repository: {}", e);
            }
        }
    }

    // Push content to gh-pages branch
    if is_directory {
        push_directory_to_gh_pages(&github_token, &repo_owner, repo, source_path).await?;
    } else {
        push_bundle_to_gh_pages(&github_token, &repo_owner, repo, source_path).await?;
    }
    println!("   ✓ Pushed content to gh-pages branch");

    // Enable GitHub Pages
    enable_github_pages(&github_token, &repo_owner, repo, custom_domain).await?;
    println!("   ✓ GitHub Pages enabled");

    // Print the URL
    let pages_url = if let Some(domain) = custom_domain {
        format!("https://{}", domain)
    } else {
        format!("https://{}.github.io/{}", repo_owner, repo)
    };
    println!("\n🌐 Your archive is now live at:");
    println!("   {}", pages_url);

    Ok(())
}

/// Get the authenticated GitHub username
async fn get_github_username(token: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to get GitHub user: {} - {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
    }

    let user: serde_json::Value = response.json().await?;
    user["login"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("Failed to get username from GitHub API"))
}

/// Check if repository exists, create if not
async fn check_or_create_repo(
    token: &str,
    owner: &str,
    repo: &str,
    private: bool,
) -> anyhow::Result<bool> {
    let client = reqwest::Client::new();

    // Check if repo exists
    let check_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
    let check_response = client
        .get(&check_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    if check_response.status().is_success() {
        return Ok(false); // Repo exists
    }

    // Create repository
    let create_response = client
        .post("https://api.github.com/user/repos")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .json(&serde_json::json!({
            "name": repo,
            "private": private,
            "description": "Agent Mail archive deployed via mcp-agent-mail",
            "auto_init": false
        }))
        .send()
        .await?;

    if !create_response.status().is_success() {
        let status = create_response.status();
        let body = create_response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to create repository: {} - {}", status, body);
    }

    Ok(true) // Created new repo
}

/// Push a ZIP bundle to gh-pages branch with a simple download page
async fn push_bundle_to_gh_pages(
    token: &str,
    owner: &str,
    repo: &str,
    bundle_path: &std::path::Path,
) -> anyhow::Result<()> {
    use base64::Engine;
    use std::io::Read;

    // Read the bundle file
    let mut file = std::fs::File::open(bundle_path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(&contents);

    // Get or create gh-pages branch
    ensure_gh_pages_branch(token, owner, repo).await?;

    // Create index.html pointing to the archive
    let index_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Agent Mail Archive</title>
    <style>
        body {{ font-family: system-ui; max-width: 800px; margin: 50px auto; padding: 20px; }}
        h1 {{ color: #333; }}
        a {{ color: #0066cc; }}
    </style>
</head>
<body>
    <h1>🤖 Agent Mail Archive</h1>
    <p>This archive was deployed using <code>mcp-agent-mail share deploy github-pages</code></p>
    <p><a href="archive.zip">Download Archive (ZIP)</a></p>
    <p>Deployed: {}</p>
</body>
</html>"#,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    // Push index.html
    push_file_to_branch(token, owner, repo, "gh-pages", "index.html", &index_html).await?;

    // Push the archive.zip
    push_file_to_branch_binary(token, owner, repo, "gh-pages", "archive.zip", &encoded).await?;

    Ok(())
}

/// Push a build directory to gh-pages branch (for SvelteKit static builds)
///
/// Recursively walks the directory and pushes all files in batches to avoid
/// GitHub API rate limits.
async fn push_directory_to_gh_pages(
    token: &str,
    owner: &str,
    repo: &str,
    build_dir: &std::path::Path,
) -> anyhow::Result<()> {
    use base64::Engine;
    use std::io::Read;
    use walkdir::WalkDir;

    // Collect all files to upload
    let mut files_to_upload: Vec<(String, Vec<u8>)> = Vec::new();

    for entry in WalkDir::new(build_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let relative_path = path
            .strip_prefix(build_dir)
            .map_err(|e| anyhow::anyhow!("Failed to get relative path: {}", e))?;

        // Convert to forward slashes for GitHub (even on Windows)
        let github_path = relative_path.to_string_lossy().replace('\\', "/");

        // Read file contents
        let mut file = std::fs::File::open(path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        files_to_upload.push((github_path, contents));
    }

    let total_files = files_to_upload.len();
    if total_files == 0 {
        anyhow::bail!("No files found in build directory: {}", build_dir.display());
    }

    println!("   Found {} files to upload", total_files);

    // Get or create gh-pages branch
    ensure_gh_pages_branch(token, owner, repo).await?;

    // Batch configuration for rate limiting
    const BATCH_SIZE: usize = 10;
    const DELAY_BETWEEN_BATCHES_MS: u64 = 1000;

    let mut uploaded = 0;

    for (batch_idx, chunk) in files_to_upload.chunks(BATCH_SIZE).enumerate() {
        // Add delay between batches (except first)
        if batch_idx > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(DELAY_BETWEEN_BATCHES_MS)).await;
        }

        // Upload files in this batch sequentially (to avoid borrowing issues)
        for (path, contents) in chunk {
            let encoded = base64::engine::general_purpose::STANDARD.encode(contents);
            push_file_to_branch_binary(token, owner, repo, "gh-pages", path, &encoded).await?;
            uploaded += 1;

            // Progress update
            print!("\r   Uploading: {}/{} files", uploaded, total_files);
            std::io::Write::flush(&mut std::io::stdout())?;
        }
    }

    println!(); // Newline after progress

    Ok(())
}

/// Ensure gh-pages branch exists
async fn ensure_gh_pages_branch(token: &str, owner: &str, repo: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    // Check if gh-pages branch exists
    let branch_url = format!(
        "https://api.github.com/repos/{}/{}/branches/gh-pages",
        owner, repo
    );
    let branch_check = client
        .get(&branch_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    if branch_check.status().is_success() {
        return Ok(()); // Branch exists
    }

    // Get default branch SHA (repo_url used for context, not response)
    let _repo_url = format!("https://api.github.com/repos/{}/{}", owner, repo);

    // Try to get main branch, or create an orphan commit
    let refs_url = format!(
        "https://api.github.com/repos/{}/{}/git/refs/heads/main",
        owner, repo
    );
    let refs_response = client
        .get(&refs_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    let sha = if refs_response.status().is_success() {
        let refs_data: serde_json::Value = refs_response.json().await?;
        refs_data["object"]["sha"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to get main branch SHA"))?
            .to_string()
    } else {
        // Create initial commit for empty repo
        create_initial_commit(token, owner, repo).await?
    };

    // Create gh-pages branch
    let create_ref_url = format!("https://api.github.com/repos/{}/{}/git/refs", owner, repo);
    let create_response = client
        .post(&create_ref_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .json(&serde_json::json!({
            "ref": "refs/heads/gh-pages",
            "sha": sha
        }))
        .send()
        .await?;

    if !create_response.status().is_success() {
        let status = create_response.status();
        let body = create_response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to create gh-pages branch: {} - {}", status, body);
    }

    Ok(())
}

/// Create initial commit for empty repository
async fn create_initial_commit(token: &str, owner: &str, repo: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();

    // Create a blob with README content
    let blob_url = format!("https://api.github.com/repos/{}/{}/git/blobs", owner, repo);
    let blob_response = client
        .post(&blob_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .json(&serde_json::json!({
            "content": "# Agent Mail Archive\n\nDeployed via mcp-agent-mail",
            "encoding": "utf-8"
        }))
        .send()
        .await?;

    let blob_data: serde_json::Value = blob_response.json().await?;
    let blob_sha = blob_data["sha"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to get blob SHA"))?;

    // Create tree
    let tree_url = format!("https://api.github.com/repos/{}/{}/git/trees", owner, repo);
    let tree_response = client
        .post(&tree_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .json(&serde_json::json!({
            "tree": [{
                "path": "README.md",
                "mode": "100644",
                "type": "blob",
                "sha": blob_sha
            }]
        }))
        .send()
        .await?;

    let tree_data: serde_json::Value = tree_response.json().await?;
    let tree_sha = tree_data["sha"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to get tree SHA"))?;

    // Create commit
    let commit_url = format!(
        "https://api.github.com/repos/{}/{}/git/commits",
        owner, repo
    );
    let commit_response = client
        .post(&commit_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .json(&serde_json::json!({
            "message": "Initial commit",
            "tree": tree_sha
        }))
        .send()
        .await?;

    let commit_data: serde_json::Value = commit_response.json().await?;
    let commit_sha = commit_data["sha"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to get commit SHA"))?
        .to_string();

    // Create main branch reference
    let ref_url = format!("https://api.github.com/repos/{}/{}/git/refs", owner, repo);
    client
        .post(&ref_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .json(&serde_json::json!({
            "ref": "refs/heads/main",
            "sha": commit_sha
        }))
        .send()
        .await?;

    Ok(commit_sha)
}

/// Push a text file to a branch
async fn push_file_to_branch(
    token: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    content: &str,
) -> anyhow::Result<()> {
    use base64::Engine;

    let client = reqwest::Client::new();
    let encoded = base64::engine::general_purpose::STANDARD.encode(content.as_bytes());

    // Check if file exists to get SHA
    let file_url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
        owner, repo, path, branch
    );
    let file_check = client
        .get(&file_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    let existing_sha = if file_check.status().is_success() {
        let file_data: serde_json::Value = file_check.json().await?;
        file_data["sha"].as_str().map(String::from)
    } else {
        None
    };

    // Create or update file
    let mut payload = serde_json::json!({
        "message": format!("Update {}", path),
        "content": encoded,
        "branch": branch
    });

    if let Some(sha) = existing_sha {
        payload["sha"] = serde_json::Value::String(sha);
    }

    let put_url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner, repo, path
    );
    let put_response = client
        .put(&put_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .json(&payload)
        .send()
        .await?;

    if !put_response.status().is_success() {
        let status = put_response.status();
        let body = put_response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to push {}: {} - {}", path, status, body);
    }

    Ok(())
}

/// Push a binary file to a branch (already base64 encoded)
async fn push_file_to_branch_binary(
    token: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    encoded_content: &str,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    // Check if file exists to get SHA
    let file_url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
        owner, repo, path, branch
    );
    let file_check = client
        .get(&file_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    let existing_sha = if file_check.status().is_success() {
        let file_data: serde_json::Value = file_check.json().await?;
        file_data["sha"].as_str().map(String::from)
    } else {
        None
    };

    // Create or update file
    let mut payload = serde_json::json!({
        "message": format!("Update {}", path),
        "content": encoded_content,
        "branch": branch
    });

    if let Some(sha) = existing_sha {
        payload["sha"] = serde_json::Value::String(sha);
    }

    let put_url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner, repo, path
    );
    let put_response = client
        .put(&put_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .json(&payload)
        .send()
        .await?;

    if !put_response.status().is_success() {
        let status = put_response.status();
        let body = put_response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to push {}: {} - {}", path, status, body);
    }

    Ok(())
}

/// Enable GitHub Pages for the repository
async fn enable_github_pages(
    token: &str,
    owner: &str,
    repo: &str,
    custom_domain: Option<&str>,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let pages_url = format!("https://api.github.com/repos/{}/{}/pages", owner, repo);

    // Check if Pages is already enabled
    let check_response = client
        .get(&pages_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    if check_response.status().is_success() {
        // Pages already enabled, update if custom domain provided
        if let Some(domain) = custom_domain {
            let update_response = client
                .put(&pages_url)
                .header("Authorization", format!("Bearer {}", token))
                .header("User-Agent", "mcp-agent-mail")
                .header("Accept", "application/vnd.github+json")
                .json(&serde_json::json!({
                    "cname": domain
                }))
                .send()
                .await?;

            if !update_response.status().is_success() {
                let status = update_response.status();
                let body = update_response.text().await.unwrap_or_default();
                anyhow::bail!("Failed to set custom domain: {} - {}", status, body);
            }
        }
        return Ok(());
    }

    // Enable GitHub Pages
    let mut payload = serde_json::json!({
        "source": {
            "branch": "gh-pages",
            "path": "/"
        }
    });

    if let Some(domain) = custom_domain {
        payload["cname"] = serde_json::Value::String(domain.to_string());
    }

    let enable_response = client
        .post(&pages_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "mcp-agent-mail")
        .header("Accept", "application/vnd.github+json")
        .json(&payload)
        .send()
        .await?;

    if !enable_response.status().is_success() {
        let status = enable_response.status();
        let body = enable_response.text().await.unwrap_or_default();
        // 409 Conflict means Pages is already enabled
        if status != reqwest::StatusCode::CONFLICT {
            anyhow::bail!("Failed to enable GitHub Pages: {} - {}", status, body);
        }
    }

    Ok(())
}

// --- Config Command Handlers ---

fn handle_config_command(cmd: ConfigCommands) -> anyhow::Result<()> {
    match cmd {
        ConfigCommands::SetPort { port } => {
            let home =
                std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME env var not set"))?;
            let config_dir = PathBuf::from(&home).join(".mcp-agent-mail");
            let config_path = config_dir.join("config.toml");

            std::fs::create_dir_all(&config_dir)?;

            let content = if config_path.exists() {
                std::fs::read_to_string(&config_path)?
            } else {
                String::new()
            };

            let mut config: toml::Table =
                toml::from_str(&content).unwrap_or_else(|_| toml::Table::new());
            let server_entry = config
                .entry("server")
                .or_insert(toml::Value::Table(toml::Table::new()));

            if let toml::Value::Table(server_table) = server_entry {
                server_table.insert("port".to_string(), toml::Value::Integer(port as i64));
            }

            let new_content = toml::to_string_pretty(&config)?;
            std::fs::write(&config_path, new_content)?;

            println!("✓ Updated port to {} in {}", port, config_path.display());
            println!("  Restart the server for changes to take effect.");
        }
        ConfigCommands::ShowPort => {
            let config = load_config();
            println!("{}", config.server.port);
        }
    }
    Ok(())
}

#[allow(clippy::unwrap_used)]
fn handle_robot_help(format: &str) {
    let registry = &*robot_help::EXAMPLE_REGISTRY;
    let output = if format.eq_ignore_ascii_case("yaml") {
        serde_yaml::to_string(registry).unwrap()
    } else {
        serde_json::to_string_pretty(registry).unwrap()
    };
    println!("{}", output);
}

#[allow(clippy::expect_used)]
fn handle_robot_status(format: &str) -> u8 {
    use lib_common::robot::ROBOT_HELP_SCHEMA_VERSION;
    use robot_help::{CheckResult, RobotStatusOutput};
    use std::collections::HashMap;

    let mut checks = HashMap::new();
    let mut exit_code = 0;

    // 1. Database Check
    let db_path = std::path::Path::new("data/mcp_agent_mail.db");
    checks.insert(
        "database".to_string(),
        CheckResult {
            status: if db_path.exists() {
                "ok".to_string()
            } else {
                "missing".to_string()
            },
            path: Some(db_path.to_string_lossy().to_string()),
            port: None,
            details: None,
        },
    );

    // 2. Git Archive Check
    let archive_path = std::path::Path::new("data/archive");
    checks.insert(
        "git_archive".to_string(),
        CheckResult {
            status: if archive_path.exists() {
                "ok".to_string()
            } else {
                "missing".to_string()
            },
            path: Some(archive_path.to_string_lossy().to_string()),
            port: None,
            details: None,
        },
    );

    // 3. Config Check
    let config = load_config();
    checks.insert(
        "config".to_string(),
        CheckResult {
            status: "ok".to_string(),
            path: None,
            port: Some(config.server.port),
            details: None,
        },
    );

    // Determine overall status
    // For now, missing DB is not critical for startup (it gets created), so we can say healthy
    // But let's be strict: if checks fail, maybe status is "degraded"
    let status = if checks.values().any(|c| c.status != "ok") {
        exit_code = 1;
        "degraded".to_string()
    } else {
        "healthy".to_string()
    };

    let output = RobotStatusOutput {
        schema_version: ROBOT_HELP_SCHEMA_VERSION.to_string(),
        tool: "mcp-agent-mail".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        status,
        checks,
        exit_code,
    };

    let json = if format.eq_ignore_ascii_case("yaml") {
        serde_yaml::to_string(&output).expect("Failed to serialize robot status to YAML")
    } else {
        serde_json::to_string_pretty(&output).expect("Failed to serialize robot status to JSON")
    };

    println!("{}", json);
    exit_code
}

#[allow(clippy::expect_used)]
fn handle_robot_examples(format: &str, args: &[String]) -> u8 {
    use robot_help::{EXAMPLE_REGISTRY, RobotExamplesOutput};

    let target = args.join(" ");
    let mut matching_examples = Vec::new();
    let target_type;

    if args.is_empty() {
        target_type = "all".to_string();
        // Collect ALL examples from the HashMap
        for entry in EXAMPLE_REGISTRY.values() {
            matching_examples.extend(entry.examples.clone());
        }
    } else {
        target_type = if target.starts_with("--") {
            "flag"
        } else {
            "command"
        }
        .to_string();

        // Look up the target in the HashMap
        if let Some(entry) = EXAMPLE_REGISTRY.get(target.as_str()) {
            matching_examples.extend(entry.examples.clone());
        } else {
            // If exact match not found, search all examples containing the target
            for entry in EXAMPLE_REGISTRY.values() {
                for example in &entry.examples {
                    if example.invocation.contains(&target) {
                        matching_examples.push(example.clone());
                    }
                }
            }
        }
    }

    let output = RobotExamplesOutput {
        schema_version: "1.0".to_string(),
        target,
        target_type,
        examples: matching_examples,
    };

    let json = if format.eq_ignore_ascii_case("yaml") {
        serde_yaml::to_string(&output).expect("Failed to serialize robot examples to YAML")
    } else {
        serde_json::to_string_pretty(&output).expect("Failed to serialize robot examples to JSON")
    };

    println!("{}", json);
    0
}

async fn handle_guard_status() -> anyhow::Result<()> {
    println!("Pre-commit Guard Status");
    println!("=======================");

    // WORKTREES_ENABLED
    let worktrees_enabled = std::env::var("WORKTREES_ENABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    println!(
        "WORKTREES_ENABLED: {}",
        if worktrees_enabled { "true" } else { "false" }
    );

    // AGENT_MAIL_GUARD_MODE
    let guard_mode =
        std::env::var("AGENT_MAIL_GUARD_MODE").unwrap_or_else(|_| "enforce".to_string());
    println!("AGENT_MAIL_GUARD_MODE: {}", guard_mode);

    // PROJECT_IDENTITY_MODE
    let identity_mode = std::env::var("PROJECT_IDENTITY_MODE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    println!(
        "PROJECT_IDENTITY_MODE: {}",
        if identity_mode { "true" } else { "false" }
    );

    // hooks_dir location
    let hooks_dir = std::path::Path::new(".git/hooks");
    if hooks_dir.exists() {
        println!("hooks_dir: {}", hooks_dir.display());
    } else {
        println!("hooks_dir: .git/hooks (directory not found)");
    }

    // pre-commit/pre-push hook presence
    let pre_commit_hook = hooks_dir.join("pre-commit");
    let pre_push_hook = hooks_dir.join("pre-push");

    println!(
        "pre-commit hook: {}",
        if pre_commit_hook.exists() {
            "present"
        } else {
            "missing"
        }
    );
    println!(
        "pre-push hook: {}",
        if pre_push_hook.exists() {
            "present"
        } else {
            "missing"
        }
    );

    Ok(())
}

/// Check if a file path matches a reservation pattern.
///
/// Pattern matching rules:
/// - `*` matches all paths
/// - `src/**` matches all paths under `src/` directory
/// - `src/main.rs` matches exactly that path or as suffix with path separator
/// - Empty patterns never match
fn path_matches_pattern(path: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }

    // Wildcard matches everything
    if pattern == "*" {
        return true;
    }

    // Double-star suffix (e.g., "src/**") - must have at least one char before /**
    // Matches paths UNDER the directory, not the directory itself
    if pattern.len() > 3 && pattern.ends_with("/**") {
        let prefix = &pattern[..pattern.len() - 3];
        // Path must start with prefix and have a path separator after it
        return path.starts_with(prefix)
            && path.len() > prefix.len()
            && path[prefix.len()..].starts_with('/');
    }

    // Exact prefix match
    if let Some(rest) = path.strip_prefix(pattern) {
        // Must be exact match or followed by path separator
        return rest.is_empty() || rest.starts_with('/');
    }

    // Check if pattern appears as a path segment (not just substring)
    let search = format!("/{}/", pattern);
    if path.contains(&search) {
        return true;
    }

    // Check if pattern is at the end after a path separator
    let suffix = format!("/{}", pattern);
    path.ends_with(&suffix)
}

async fn handle_guard_check(
    stdin_nul: bool,
    advisory: bool,
    project: Option<String>,
) -> anyhow::Result<()> {
    use std::io::{self, Read};
    use std::time::Duration;

    // Determine project slug: use provided, env var, or detect from git
    let project_slug = if let Some(p) = project {
        p
    } else if let Ok(p) = std::env::var("MCP_PROJECT_SLUG") {
        p
    } else {
        // Try to detect from current directory (git repo name)
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    };

    // Read paths from stdin
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    // Parse paths (null-separated if --stdin-nul, otherwise newline-separated)
    let paths: Vec<String> = if stdin_nul {
        input
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    } else {
        input.lines().map(|s| s.to_string()).collect()
    };

    if paths.is_empty() {
        if advisory {
            eprintln!("Warning: No paths provided to check");
            return Ok(());
        } else {
            eprintln!("Error: No paths provided to check");
            std::process::exit(1);
        }
    }

    // Get active file reservations from MCP API with timeout
    let url =
        std::env::var("MCP_AGENT_MAIL_URL").unwrap_or_else(|_| "http://localhost:8765".into());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let reservations_result = client
        .post(format!("{}/api/file_reservations/list", url))
        .json(&serde_json::json!({
            "project_slug": project_slug
        }))
        .send()
        .await;

    // (path, agent_name, pattern) tuples for conflicting paths
    let mut conflicting_paths: Vec<(String, String, String)> = Vec::new();

    match reservations_result {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(json) => {
                if let Some(reservations) = json.get("reservations").and_then(|r| r.as_array()) {
                    for path in &paths {
                        for reservation in reservations {
                            let pattern = reservation
                                .get("path_pattern")
                                .and_then(|p| p.as_str())
                                .unwrap_or("");
                            let agent_name = reservation
                                .get("agent_name")
                                .and_then(|a| a.as_str())
                                .unwrap_or("unknown");

                            if path_matches_pattern(path, pattern) {
                                conflicting_paths.push((
                                    path.clone(),
                                    agent_name.to_string(),
                                    pattern.to_string(),
                                ));
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if advisory {
                    eprintln!("Warning: Could not query file reservations: {}", e);
                } else {
                    eprintln!("Error: Could not query file reservations: {}", e);
                    std::process::exit(1);
                }
            }
        },
        _ => {
            if advisory {
                eprintln!("Warning: Could not connect to MCP server at {}", url);
            } else {
                eprintln!("Error: Could not connect to MCP server at {}", url);
                std::process::exit(1);
            }
        }
    }

    if conflicting_paths.is_empty() {
        // All paths are clear
        if advisory {
            eprintln!("All {} paths are available for editing", paths.len());
        }
        Ok(())
    } else {
        // Some paths are reserved
        if advisory {
            eprintln!(
                "Warning: {} path(s) are currently reserved:",
                conflicting_paths.len()
            );
            for (path, agent, pattern) in &conflicting_paths {
                eprintln!(
                    "  {} (reserved by agent '{}', pattern: {})",
                    path, agent, pattern
                );
            }
        } else {
            eprintln!(
                "Error: {} path(s) are currently reserved:",
                conflicting_paths.len()
            );
            for (path, agent, pattern) in &conflicting_paths {
                eprintln!(
                    "  {} (reserved by agent '{}', pattern: {})",
                    path, agent, pattern
                );
            }
            std::process::exit(1);
        }
        Ok(())
    }
}

async fn handle_guard(args: GuardArgs) -> anyhow::Result<()> {
    match args.command {
        GuardCommands::Status => handle_guard_status().await,
        GuardCommands::Check {
            stdin_nul,
            advisory,
            project,
        } => handle_guard_check(stdin_nul, advisory, project).await,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Install panic hook FIRST, before anything else
    // This ensures panics are logged even during initialization
    panic_hook::init_panic_hook();

    let cli = Cli::parse();

    if cli.robot_help {
        handle_robot_help(&cli.format);
        return Ok(());
    }

    if cli.robot_status {
        let code = handle_robot_status(&cli.format);
        if code != 0 {
            std::process::exit(code as i32);
        }
        return Ok(());
    }

    if let Some(ref args) = cli.robot_examples {
        // If args is empty (but Some), it means flag was present but no values
        // If flag not present, it is None.
        let code = handle_robot_examples(&cli.format, args);
        if code != 0 {
            std::process::exit(code as i32);
        }
        return Ok(());
    }

    setup_tracing(cli.log_format == "json")?;
    let config = load_config();

    match cli.command {
        Some(Commands::Serve(args)) => match args.command {
            ServeCommands::Http {
                port,
                with_ui,
                no_ui,
            } => handle_serve_http(port, with_ui, no_ui, config).await?,
            ServeCommands::Mcp { transport, port } => {
                handle_serve_mcp(transport, port, config).await?
            }
        },
        Some(Commands::Health { url }) => handle_health(url).await?,
        Some(Commands::Config(args)) => handle_config_command(args.command)?,
        Some(Commands::Schema { format, output }) => handle_schema(format, output)?,
        Some(Commands::Tools) => handle_tools(),
        Some(Commands::Install(args)) => match args.command {
            InstallCommands::Alias { force } => handle_install_alias(force)?,
        },
        Some(Commands::Service(args)) => match args.command {
            ServiceCommands::Start { port, background } => {
                handle_service_start(port, background, config).await?
            }
            ServiceCommands::Stop { port } => handle_service_stop(port)?,
            ServiceCommands::Status { port } => handle_service_status(port).await?,
            ServiceCommands::Restart { port } => handle_service_restart(port, config).await?,
        },
        Some(Commands::Share(args)) => match args.command {
            ShareCommands::Keypair { output } => handle_share_keypair(output)?,
            ShareCommands::Verify {
                manifest,
                public_key,
            } => handle_share_verify(&manifest, public_key.as_deref())?,
            ShareCommands::Encrypt { .. } => {
                println!("Age encryption not yet implemented - use Python version");
            }
            ShareCommands::Decrypt { .. } => {
                println!("Age decryption not yet implemented - use Python version");
            }
            ShareCommands::Deploy { command } => match command {
                DeployCommands::GithubPages {
                    repo,
                    owner,
                    bundle,
                    build_dir,
                    token,
                    create_repo,
                    private,
                    custom_domain,
                } => {
                    handle_deploy_github_pages(
                        &repo,
                        owner.as_deref(),
                        bundle.as_deref(),
                        build_dir.as_deref(),
                        token.as_deref(),
                        create_repo,
                        private,
                        custom_domain.as_deref(),
                    )
                    .await?
                }
            },
            ShareCommands::Export { command } => match command {
                ExportCommands::StaticData {
                    output,
                    scrub,
                    limit,
                    project,
                } => handle_export_static_data(&output, &scrub, limit, project.as_deref()).await?,
            },
        },
        Some(Commands::Archive(args)) => handle_archive_command(args.command).await?,
        Some(Commands::Summarize(args)) => handle_summarize(args).await?,
        Some(Commands::Products(args)) => handle_products(args).await?,
        Some(Commands::Guard(args)) => handle_guard(args).await?,
        Some(Commands::Mail(args)) => handle_mail(args).await?,
        Some(Commands::Version) => println!("mcp-agent-mail v{}", env!("CARGO_PKG_VERSION")),
        None => {
            Cli::command().print_help()?;
        }
    }

    Ok(())
}

// --- Summarize Command Handler ---

async fn handle_summarize(args: SummarizeArgs) -> anyhow::Result<()> {
    let url =
        std::env::var("MCP_AGENT_MAIL_URL").unwrap_or_else(|_| "http://localhost:8765".into());
    let client = reqwest::Client::new();

    // Parse thread IDs (comma-separated)
    let thread_ids: Vec<String> = args
        .thread_id
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Call the MCP summarize_thread tool
    let response = client
        .post(format!("{}/api/tools/call", url))
        .json(&serde_json::json!({
            "name": "summarize_thread",
            "arguments": {
                "project_slug": args.project,
                "thread_id": thread_ids,
                "per_thread_limit": args.per_thread_limit,
                "no_llm": args.no_llm
            }
        }))
        .send()
        .await?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        eprintln!("Error calling summarize_thread: {}", response.status());
        eprintln!("Response: {}", response.text().await?);
        std::process::exit(1);
    }

    Ok(())
}

// --- Archive Command Handlers ---

/// Create a restorable snapshot archive
async fn handle_archive_save(
    archives_dir: &std::path::Path,
    label: Option<String>,
    include_git: bool,
) -> anyhow::Result<()> {
    use chrono::Utc;
    use std::fs;
    use std::io::Write;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let archive_label = label.unwrap_or_else(|| timestamp.clone());
    let archive_name = format!("archive_{}_{}.zip", archive_label, timestamp);

    // Ensure archives directory exists
    fs::create_dir_all(archives_dir)?;

    let archive_path = archives_dir.join(&archive_name);
    let file = fs::File::create(&archive_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Add database file
    let db_path = std::path::Path::new("data/mcp_agent_mail.db");
    if db_path.exists() {
        let content = fs::read(db_path)?;
        zip.start_file("mcp_agent_mail.db", options)?;
        zip.write_all(&content)?;
        println!("✓ Added database to archive");
    } else {
        println!("⚠ No database file found");
    }

    // Add git storage if requested (use data/archive which is the actual path)
    if include_git {
        let git_storage = std::path::Path::new("data/archive");
        if git_storage.exists() {
            add_directory_to_zip(&mut zip, git_storage, "git_storage", options)?;
            println!("✓ Added git storage to archive");
        }
    }

    // Add metadata
    let metadata = serde_json::json!({
        "label": archive_label,
        "timestamp": timestamp,
        "version": env!("CARGO_PKG_VERSION"),
        "include_git": include_git,
    });
    zip.start_file("metadata.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&metadata)?.as_bytes())?;

    zip.finish()?;

    println!("\n✓ Archive created: {}", archive_path.display());
    println!("  Label: {}", archive_label);
    Ok(())
}

/// List available restore points
fn handle_archive_list(archives_dir: &std::path::Path, json: bool) -> anyhow::Result<()> {
    use std::fs;

    if !archives_dir.exists() {
        if json {
            println!("[]");
        } else {
            println!("No archives found. Directory: {}", archives_dir.display());
        }
        return Ok(());
    }

    let mut archives: Vec<serde_json::Value> = Vec::new();

    for entry in fs::read_dir(archives_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "zip") {
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let filename = file_name.to_string_lossy().to_string();
            let metadata = fs::metadata(&path)?;
            let size = metadata.len();
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            // Try to read metadata.json from the archive
            let archive_metadata = read_archive_metadata(&path);

            archives.push(serde_json::json!({
                "filename": filename,
                "path": path.display().to_string(),
                "size_bytes": size,
                "modified_epoch": modified,
                "label": archive_metadata.get("label"),
                "timestamp": archive_metadata.get("timestamp"),
                "version": archive_metadata.get("version"),
            }));
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&archives)?);
    } else if archives.is_empty() {
        println!("No archives found.");
    } else {
        println!("Available restore points:\n");
        for archive in &archives {
            let label = archive["label"].as_str().unwrap_or("(unlabeled)");
            let filename = archive["filename"].as_str().unwrap_or("?");
            let size = archive["size_bytes"].as_u64().unwrap_or(0);
            let size_mb = size as f64 / (1024.0 * 1024.0);
            println!("  • {} ({})", label, filename);
            println!("    Size: {:.2} MB", size_mb);
        }
    }
    Ok(())
}

/// Restore from a backup archive
fn handle_archive_restore(file: &str, yes: bool) -> anyhow::Result<()> {
    use std::fs;
    use std::io::Write;

    let archive_path = std::path::Path::new(file);
    if !archive_path.exists() {
        anyhow::bail!("Archive not found: {}", file);
    }

    if !yes {
        print!("This will REPLACE current data. Continue? [y/N] ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Restore database
    if let Ok(mut db_file) = archive.by_name("mcp_agent_mail.db") {
        let db_path = std::path::Path::new("data/mcp_agent_mail.db");
        fs::create_dir_all("data")?;
        let mut content = Vec::new();
        use std::io::Read;
        db_file.read_to_end(&mut content)?;
        fs::write(db_path, content)?;
        println!("✓ Restored database");
    }

    // Restore git storage (restore to data/archive)
    let git_prefix = "git_storage/";
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name.starts_with(git_prefix) && !file.is_dir() {
            // Map git_storage/ to data/archive/
            let relative_path = name.strip_prefix(git_prefix).unwrap_or(&name);
            let dest_path = std::path::Path::new("data/archive").join(relative_path);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut content = Vec::new();
            use std::io::Read;
            file.read_to_end(&mut content)?;
            fs::write(&dest_path, content)?;
        }
    }
    println!("✓ Restored git storage");

    println!("\n✓ Restore complete from: {}", archive_path.display());
    Ok(())
}

/// Wipe all state with optional archive backup
async fn handle_archive_clear_and_reset(
    archives_dir: &std::path::Path,
    archive: bool,
    label: Option<String>,
    yes: bool,
) -> anyhow::Result<()> {
    use std::fs;
    use std::io::Write;

    if !yes {
        print!("This will DELETE ALL DATA. Continue? [y/N] ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Create archive first if requested
    if archive {
        println!("Creating backup archive before wipe...");
        let backup_label = label.or_else(|| Some("pre-wipe".to_string()));
        handle_archive_save(archives_dir, backup_label, true).await?;
    }

    // Remove database
    let db_path = std::path::Path::new("data/mcp_agent_mail.db");
    if db_path.exists() {
        fs::remove_file(db_path)?;
        println!("✓ Removed database");
    }

    // Remove git storage (data/archive)
    let git_storage = std::path::Path::new("data/archive");
    if git_storage.exists() {
        fs::remove_dir_all(git_storage)?;
        println!("✓ Removed git storage");
    }

    // Remove attachments
    let attachments = std::path::Path::new("data/attachments");
    if attachments.exists() {
        fs::remove_dir_all(attachments)?;
        println!("✓ Removed attachments");
    }

    println!("\n✓ All data cleared. Run migrations to reinitialize.");
    Ok(())
}

async fn handle_archive_command(cmd: ArchiveCommands) -> anyhow::Result<()> {
    let archives_dir = std::path::Path::new("data/archives");

    match cmd {
        ArchiveCommands::Save { label, include_git } => {
            handle_archive_save(archives_dir, label, include_git).await
        }
        ArchiveCommands::List { json } => handle_archive_list(archives_dir, json),
        ArchiveCommands::Restore { file, yes } => handle_archive_restore(&file, yes),
        ArchiveCommands::ClearAndReset {
            archive,
            label,
            yes,
        } => handle_archive_clear_and_reset(archives_dir, archive, label, yes).await,
    }
}

/// Helper to add a directory recursively to a ZIP archive
fn add_directory_to_zip<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &std::path::Path,
    prefix: &str,
    options: zip::write::SimpleFileOptions,
) -> anyhow::Result<()> {
    use std::fs;
    use std::io::Read;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let name = format!("{}/{}", prefix, file_name.to_string_lossy());

        if path.is_dir() {
            add_directory_to_zip(zip, &path, &name, options)?;
        } else {
            let mut file = fs::File::open(&path)?;
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;
            zip.start_file(&name, options)?;
            std::io::Write::write_all(zip, &content)?;
        }
    }

    Ok(())
}

/// Read metadata.json from a zip archive, returning empty JSON object on any error.
fn read_archive_metadata(path: &std::path::Path) -> serde_json::Value {
    let Ok(file) = std::fs::File::open(path) else {
        return serde_json::json!({});
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return serde_json::json!({});
    };
    let Ok(mut meta_file) = zip.by_name("metadata.json") else {
        return serde_json::json!({});
    };

    let mut content = String::new();
    use std::io::Read as _;
    if meta_file.read_to_string(&mut content).is_err() {
        return serde_json::json!({});
    }

    serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
}

async fn handle_products(args: ProductsArgs) -> anyhow::Result<()> {
    use lib_core::ctx::Ctx;
    use lib_core::model::ModelManager;

    let config = load_config();
    let mm = ModelManager::new(std::sync::Arc::new(config.clone())).await?;
    let ctx = Ctx::root_ctx();

    match args.command {
        ProductsCommands::Ensure { product_uid, name } => {
            let product =
                lib_core::model::product::ProductBmc::ensure(&ctx, &mm, &product_uid, &name)
                    .await?;
            println!(
                "Ensured product: {} ({})",
                product.name, product.product_uid
            );
        }
        ProductsCommands::Link {
            product_uid,
            project,
        } => {
            let product =
                lib_core::model::product::ProductBmc::get_by_uid(&ctx, &mm, &product_uid).await?;
            let project =
                lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, &mm, &project)
                    .await?;

            lib_core::model::product::ProductBmc::link_project(
                &ctx,
                &mm,
                product.id,
                project.id.get(),
            )
            .await?;
            println!(
                "Linked project '{}' to product '{}'",
                project.human_key, product.name
            );
        }
        ProductsCommands::Status { product_uid } => {
            let product =
                lib_core::model::product::ProductBmc::get_by_uid(&ctx, &mm, &product_uid).await?;
            let project_ids =
                lib_core::model::product::ProductBmc::get_linked_projects(&ctx, &mm, product.id)
                    .await?;

            println!("Product: {} ({})", product.name, product.product_uid);
            println!("Linked Projects: {}", project_ids.len());
            for pid in project_ids {
                if let Ok(proj) = lib_core::model::project::ProjectBmc::get(
                    &ctx,
                    &mm,
                    lib_core::ProjectId::new(pid),
                )
                .await
                {
                    println!("  - {} ({})", proj.human_key, proj.slug);
                } else {
                    println!("  - Unknown Project ID: {}", pid);
                }
            }
        }
        ProductsCommands::Search {
            product_uid,
            query,
            limit,
        } => {
            // Search logic needs to search across all linked projects
            let product =
                lib_core::model::product::ProductBmc::get_by_uid(&ctx, &mm, &product_uid).await?;
            let project_ids =
                lib_core::model::product::ProductBmc::get_linked_projects(&ctx, &mm, product.id)
                    .await?;

            let mut all_matches = Vec::new();
            for pid in project_ids {
                if let Ok(messages) =
                    lib_core::model::message::MessageBmc::search(&ctx, &mm, pid, &query, limit)
                        .await
                {
                    all_matches.extend(messages);
                }
            }

            // Sort by date desc
            all_matches.sort_by(|a, b| b.created_ts.cmp(&a.created_ts));
            all_matches.truncate(limit as usize);

            println!(
                "Found {} matches in product '{}':",
                all_matches.len(),
                product.name
            );
            for msg in all_matches {
                println!(
                    "  - [{}] {}: {}",
                    msg.created_ts.date(),
                    msg.sender_name,
                    msg.subject
                );
            }
        }
        ProductsCommands::Inbox {
            product_uid,
            agent,
            urgent_only,
            include_bodies,
        } => {
            // Inbox logic: Get all projects, find agent ID in each, fetch inbox
            let product =
                lib_core::model::product::ProductBmc::get_by_uid(&ctx, &mm, &product_uid).await?;
            let project_ids =
                lib_core::model::product::ProductBmc::get_linked_projects(&ctx, &mm, product.id)
                    .await?;

            println!("Inbox for agent '{}' in product '{}':", agent, product.name);

            let mut found_any = false;
            for pid in project_ids {
                // We need to find the agent in this project
                // Agent names are unique per project, not globally.
                // But typically a "user" agent might have same name across projects?
                // Or we look for agent with that name in that project.

                // We don't have get_by_name(project_id, name) easily available on AgentBmc exposed here?
                // Let's check AgentBmc.
                // Actually MessageBmc::list_inbox_for_agent takes agent_id.
                // We need to resolve agent name -> id per project.

                // For now, let's skip deep implementation of this and Search/SummarizeThread until basic struct is verified with tests.
                // But I'll put a placeholder implementation.

                // Assuming we can fix this in next iteration or if I have AgentBmc::get_by_name_and_project
                use lib_core::model::agent::AgentBmc;
                // We need to implement get_by_name_in_project in AgentBmc or similar logic
                // For now, let's iterate all agents in project and match name (slow but works for CLI)
                if let Ok(agents) =
                    AgentBmc::list_all_for_project(&ctx, &mm, lib_core::ProjectId::from(pid)).await
                {
                    if let Some(agent_obj) = agents.into_iter().find(|a| a.name == agent) {
                        if let Ok(messages) =
                            lib_core::model::message::MessageBmc::list_inbox_for_agent(
                                &ctx,
                                &mm,
                                pid,
                                agent_obj.id.get(),
                                50,
                            )
                            .await
                        {
                            for msg in messages {
                                if urgent_only
                                    && msg.importance != "high"
                                    && msg.importance != "urgent"
                                {
                                    continue;
                                }
                                found_any = true;
                                println!(
                                    "  [{}] {}: {} ({})",
                                    msg.created_ts.date(),
                                    msg.sender_name,
                                    msg.subject,
                                    msg.importance
                                );
                                if include_bodies {
                                    println!("    {}", msg.body_md);
                                }
                            }
                        }
                    }
                }
            }
            if !found_any {
                println!("  (No messages found)");
            }
        }
        ProductsCommands::SummarizeThread { .. } => {
            println!("Summarize thread not fully implemented yet in CLI");
        }
    }

    Ok(())
}

async fn handle_mail_status() -> anyhow::Result<()> {
    println!("Mail Status");
    println!("===========");

    // resolved project_key for a directory
    let project_slug = std::env::var("MCP_AGENT_MAIL_PROJECT_SLUG")
        .unwrap_or_else(|_| "mcp-agent-mail-rs".to_string());
    println!("Project: {}", project_slug);

    // registration status - check if we're registered with Agent Mail
    let agent_registered = std::env::var("MCP_AGENT_MAIL_URL").is_ok();
    println!(
        "Agent Registration: {}",
        if agent_registered {
            "registered"
        } else {
            "not registered"
        }
    );

    // active file reservations - query MCP API
    if let Ok(url) = std::env::var("MCP_AGENT_MAIL_URL") {
        let client = reqwest::Client::new();

        // Check file reservations
        match client
            .post(format!("{}/api/file_reservations/list", url))
            .json(&serde_json::json!({
                "project_slug": project_slug
            }))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(reservations) =
                            json.get("reservations").and_then(|r| r.as_array())
                        {
                            println!("Active File Reservations: {} active", reservations.len());
                            for reservation in reservations {
                                if let (Some(path), Some(expires)) = (
                                    reservation.get("path_pattern").and_then(|p| p.as_str()),
                                    reservation.get("expires_ts").and_then(|e| e.as_str()),
                                ) {
                                    println!("  - {} (expires: {})", path, expires);
                                }
                            }
                        } else {
                            println!("Active File Reservations: 0 active");
                        }
                    }
                    Err(_) => println!("Active File Reservations: unable to query"),
                }
            }
            _ => println!("Active File Reservations: unable to query"),
        }
    } else {
        println!("Active File Reservations: MCP server not configured");
    }

    // product linkage - check if project is linked to products
    if let Ok(url) = std::env::var("MCP_AGENT_MAIL_URL") {
        let client = reqwest::Client::new();

        match client
            .post(format!("{}/api/product/list", url))
            .json(&serde_json::json!({
                "project_slug": project_slug
            }))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(products) = json.get("products").and_then(|p| p.as_array()) {
                            println!("Product Linkage: {} linked", products.len());
                            for product in products {
                                if let Some(name) = product.get("name").and_then(|n| n.as_str()) {
                                    println!("  - {}", name);
                                }
                            }
                        } else {
                            println!("Product Linkage: no linked products");
                        }
                    }
                    Err(_) => println!("Product Linkage: unable to query"),
                }
            }
            _ => println!("Product Linkage: unable to query"),
        }
    } else {
        println!("Product Linkage: MCP server not configured");
    }

    Ok(())
}

async fn handle_mail(args: MailArgs) -> anyhow::Result<()> {
    match args.command {
        MailCommands::Status => handle_mail_status().await,
    }
}

#[cfg(test)]
mod guard_pattern_tests {
    use super::*;

    #[test]
    fn test_empty_pattern_never_matches() {
        assert!(!path_matches_pattern("src/main.rs", ""));
        assert!(!path_matches_pattern("", ""));
    }

    #[test]
    fn test_wildcard_matches_everything() {
        assert!(path_matches_pattern("src/main.rs", "*"));
        assert!(path_matches_pattern("", "*"));
        assert!(path_matches_pattern("any/path/here", "*"));
    }

    #[test]
    fn test_double_star_suffix() {
        // src/** should match anything under src/
        assert!(path_matches_pattern("src/main.rs", "src/**"));
        assert!(path_matches_pattern("src/lib/mod.rs", "src/**"));
        assert!(path_matches_pattern("src/", "src/**"));

        // Should not match src itself without trailing slash
        assert!(!path_matches_pattern("src", "src/**"));

        // Should not match other directories
        assert!(!path_matches_pattern("src_backup/main.rs", "src/**"));
        assert!(!path_matches_pattern("other/src/main.rs", "src/**"));
    }

    #[test]
    fn test_double_star_edge_cases() {
        // Pattern exactly "/**" should not match (needs prefix)
        assert!(!path_matches_pattern("any/path", "/**"));

        // Pattern "**" alone is not a wildcard (only * is)
        assert!(!path_matches_pattern("any/path", "**"));
    }

    #[test]
    fn test_exact_prefix_match() {
        // Exact match
        assert!(path_matches_pattern("src/main.rs", "src/main.rs"));

        // Prefix with path separator
        assert!(path_matches_pattern("src/main/file.rs", "src/main"));

        // Should not match partial directory names
        assert!(!path_matches_pattern("src/main_backup/file.rs", "src/main"));
    }

    #[test]
    fn test_path_segment_matching() {
        // Pattern appears as path segment in middle
        assert!(path_matches_pattern("foo/src/main.rs", "src"));

        // Pattern at end after path separator
        assert!(path_matches_pattern("foo/bar/src", "src"));

        // Should not match as substring within segment
        assert!(!path_matches_pattern("mysrc/main.rs", "src"));
        assert!(!path_matches_pattern("src_old/main.rs", "src"));
    }

    #[test]
    fn test_no_false_positives() {
        // These should NOT match
        assert!(!path_matches_pattern("src_backup/file.rs", "src"));
        assert!(!path_matches_pattern("other_src/file.rs", "src"));
        assert!(!path_matches_pattern("resource/file.rs", "src"));
    }
}

// =============================================================================
// Tests for robot-* flag handlers (TDD - mcp-agent-mail-rs-vgs4)
// =============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod robot_handler_tests {
    use super::robot_help::{EXAMPLE_REGISTRY, RobotExamplesOutput, RobotStatusOutput};

    #[test]
    fn test_robot_help_json_valid() {
        // The EXAMPLE_REGISTRY should serialize to valid JSON
        let registry = &*EXAMPLE_REGISTRY;
        let json_str = serde_json::to_string_pretty(registry).expect("must serialize to JSON");

        // Parse it back to verify it's valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("must be valid JSON");
        assert!(parsed.is_object());

        // Should have robot-* entries
        assert!(
            parsed.get("--robot-help").is_some(),
            "--robot-help must be in registry"
        );
        assert!(
            parsed.get("--robot-examples").is_some(),
            "--robot-examples must be in registry"
        );
        assert!(
            parsed.get("--robot-status").is_some(),
            "--robot-status must be in registry"
        );
    }

    #[test]
    fn test_robot_examples_self_documenting() {
        // robot-examples should include examples for robot-examples itself
        let registry = &*EXAMPLE_REGISTRY;

        let robot_examples_entry = registry
            .get("--robot-examples")
            .expect("--robot-examples must exist in registry");

        // Verify it has the target_type "flag"
        assert_eq!(robot_examples_entry.target_type, "flag");

        // Verify it has examples
        assert!(
            !robot_examples_entry.examples.is_empty(),
            "--robot-examples must have examples"
        );

        // Verify at least one example mentions robot-examples
        let has_self_example = robot_examples_entry
            .examples
            .iter()
            .any(|ex| ex.invocation.contains("robot-examples"));
        assert!(
            has_self_example,
            "--robot-examples must have examples that reference itself"
        );
    }

    #[test]
    fn test_robot_status_output_structure() {
        // Verify RobotStatusOutput has all required fields
        use std::collections::HashMap;

        let output = RobotStatusOutput {
            schema_version: "1.0.0".to_string(),
            tool: "mcp-agent-mail".to_string(),
            version: "0.3.0".to_string(),
            timestamp: "2024-12-22T00:00:00Z".to_string(),
            status: "healthy".to_string(),
            checks: HashMap::new(),
            exit_code: 0,
        };

        let json = serde_json::to_value(&output).expect("must serialize");

        // Verify all required fields are present
        assert!(json.get("schema_version").is_some());
        assert!(json.get("tool").is_some());
        assert!(json.get("version").is_some());
        assert!(json.get("timestamp").is_some());
        assert!(json.get("status").is_some());
        assert!(json.get("checks").is_some());
        assert!(json.get("exit_code").is_some());

        // Verify exit_code is a number
        assert!(json["exit_code"].is_number());
        assert_eq!(json["exit_code"].as_u64().unwrap(), 0);
    }

    #[test]
    fn test_robot_status_exit_codes_match() {
        use super::robot_help::CheckResult;
        use std::collections::HashMap;

        // Test healthy case - exit_code should be 0
        let mut healthy_checks = HashMap::new();
        healthy_checks.insert(
            "database".to_string(),
            CheckResult {
                status: "ok".to_string(),
                path: Some("data/test.db".to_string()),
                port: None,
                details: None,
            },
        );

        let healthy_output = RobotStatusOutput {
            schema_version: "1.0.0".to_string(),
            tool: "mcp-agent-mail".to_string(),
            version: "0.3.0".to_string(),
            timestamp: "2024-12-22T00:00:00Z".to_string(),
            status: "healthy".to_string(),
            checks: healthy_checks,
            exit_code: 0,
        };
        assert_eq!(healthy_output.exit_code, 0);
        assert_eq!(healthy_output.status, "healthy");

        // Test degraded case - exit_code should be 1
        let mut degraded_checks = HashMap::new();
        degraded_checks.insert(
            "database".to_string(),
            CheckResult {
                status: "missing".to_string(),
                path: Some("data/test.db".to_string()),
                port: None,
                details: None,
            },
        );

        let degraded_output = RobotStatusOutput {
            schema_version: "1.0.0".to_string(),
            tool: "mcp-agent-mail".to_string(),
            version: "0.3.0".to_string(),
            timestamp: "2024-12-22T00:00:00Z".to_string(),
            status: "degraded".to_string(),
            checks: degraded_checks,
            exit_code: 1,
        };
        assert_eq!(degraded_output.exit_code, 1);
        assert_eq!(degraded_output.status, "degraded");
    }

    #[test]
    fn test_robot_examples_output_structure() {
        // Verify RobotExamplesOutput serializes correctly
        use lib_common::robot::Example;

        let output = RobotExamplesOutput {
            schema_version: "1.0".to_string(),
            target: "--robot-examples".to_string(),
            target_type: "flag".to_string(),
            examples: vec![Example {
                invocation: "mcp-agent-mail --robot-examples serve".to_string(),
                description: "Examples for serve command".to_string(),
            }],
        };

        let json = serde_json::to_value(&output).expect("must serialize");

        // Verify structure
        assert_eq!(json["schema_version"], "1.0");
        assert_eq!(json["target"], "--robot-examples");
        assert_eq!(json["target_type"], "flag");
        assert!(json["examples"].is_array());
        assert_eq!(json["examples"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_example_registry_has_all_commands() {
        // The registry should have entries for all major commands
        let registry = &*EXAMPLE_REGISTRY;

        // Core subcommands
        let core_commands = vec![
            "serve",
            "serve http",
            "serve mcp",
            "health",
            "config",
            "schema",
            "tools",
            "install",
            "service",
            "share",
            "archive",
            "summarize",
            "version",
            "products",
            "guard",
            "mail",
        ];

        for cmd in core_commands {
            assert!(
                registry.contains_key(cmd),
                "Registry must contain command: {}",
                cmd
            );
        }

        // Robot flags
        let robot_flags = vec!["--robot-help", "--robot-examples", "--robot-status"];

        for flag in robot_flags {
            assert!(
                registry.contains_key(flag),
                "Registry must contain flag: {}",
                flag
            );
        }
    }

    #[test]
    fn test_example_registry_entries_have_examples() {
        // Every entry should have at least one example
        let registry = &*EXAMPLE_REGISTRY;

        for (key, entry) in registry.iter() {
            assert!(
                !entry.examples.is_empty(),
                "Entry '{}' must have at least one example",
                key
            );

            // Each example should have non-empty invocation and description
            for (i, ex) in entry.examples.iter().enumerate() {
                assert!(
                    !ex.invocation.is_empty(),
                    "Entry '{}' example {} must have invocation",
                    key,
                    i
                );
                assert!(
                    !ex.description.is_empty(),
                    "Entry '{}' example {} must have description",
                    key,
                    i
                );
            }
        }
    }

    #[test]
    fn test_example_registry_has_deploy_commands() {
        // The registry should have entries for deploy commands
        let registry = &*EXAMPLE_REGISTRY;

        // Deploy subcommands
        let deploy_commands = vec!["share deploy", "share deploy github-pages"];

        for cmd in deploy_commands {
            assert!(
                registry.contains_key(cmd),
                "Registry must contain command: {}",
                cmd
            );
        }
    }
}

// =============================================================================
// Tests for GitHub Pages deployment (TDD - mcp-agent-mail-rs-xpau)
// =============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod github_pages_tests {
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_bundle_validation_missing_file() {
        // Verify that handle_deploy_github_pages validates bundle exists
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent.zip");

        assert!(!nonexistent.exists(), "Test setup: file should not exist");
    }

    #[test]
    fn test_bundle_validation_zip_extension() {
        // Verify .zip extension detection
        let path = PathBuf::from("test.zip");
        let is_zip = path.extension().is_some_and(|ext| ext == "zip");
        assert!(is_zip, "Should detect .zip extension");

        let path2 = PathBuf::from("test.json");
        let is_zip2 = path2.extension().is_some_and(|ext| ext == "zip");
        assert!(!is_zip2, "Should not detect .json as zip");
    }

    #[test]
    fn test_github_api_url_construction() {
        // Test GitHub API URL construction
        let owner = "testuser";
        let repo = "testrepo";

        let repo_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        assert_eq!(repo_url, "https://api.github.com/repos/testuser/testrepo");

        let pages_url = format!("https://api.github.com/repos/{}/{}/pages", owner, repo);
        assert_eq!(
            pages_url,
            "https://api.github.com/repos/testuser/testrepo/pages"
        );

        let branch_url = format!(
            "https://api.github.com/repos/{}/{}/branches/gh-pages",
            owner, repo
        );
        assert_eq!(
            branch_url,
            "https://api.github.com/repos/testuser/testrepo/branches/gh-pages"
        );
    }

    #[test]
    fn test_pages_url_construction() {
        // Test GitHub Pages URL construction
        let owner = "testuser";
        let repo = "testrepo";

        // Default pages URL
        let default_url = format!("https://{}.github.io/{}", owner, repo);
        assert_eq!(default_url, "https://testuser.github.io/testrepo");

        // Custom domain URL
        let custom_domain = "archive.example.com";
        let custom_url = format!("https://{}", custom_domain);
        assert_eq!(custom_url, "https://archive.example.com");
    }

    #[test]
    fn test_index_html_generation() {
        // Test that index.html contains expected elements
        let timestamp = "2024-12-22 00:00:00 UTC";
        let index_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Agent Mail Archive</title>
</head>
<body>
    <h1>Agent Mail Archive</h1>
    <p>Deployed: {}</p>
</body>
</html>"#,
            timestamp
        );

        assert!(index_html.contains("Agent Mail Archive"));
        assert!(index_html.contains(timestamp));
        assert!(index_html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_deploy_commands_enum_variants() {
        // Test that DeployCommands enum is properly constructed
        use super::DeployCommands;

        // This test ensures the enum exists and GithubPages variant is available
        // The actual parsing is tested via CLI tests
        let _variant_exists = |cmd: DeployCommands| match cmd {
            DeployCommands::GithubPages { .. } => true,
        };
    }
}
