use clap::{Args, CommandFactory, Parser, Subcommand};
use lib_common::{
    config::AppConfig,
    robot::{RobotArg, RobotCommand, RobotHelp},
};
use lib_mcp::{docs::generate_markdown_docs, run_sse, run_stdio, tools::get_tool_schemas};
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use tracing::info;

mod panic_hook;

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

    /// Show version info
    Version,
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
        #[arg(short, long)]
        identity: Option<String>,
        #[arg(long)]
        passphrase: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
    },
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
struct ArchiveArgs {
    #[command(subcommand)]
    command: ArchiveCommands,
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

async fn handle_service_restart(port: u16, config: AppConfig) -> anyhow::Result<()> {
    println!("Restarting server on port {}...", port);

    if let Some(pid) = find_pid_on_port(port) {
        println!("Stopping existing server (PID {})...", pid);
        kill_process(pid)?;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    println!("Starting server on port {}...", port);
    handle_serve_http(Some(port), true, false, config).await?;

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

#[allow(clippy::unwrap_used)] // JSON serialization of RobotHelp is infallible
fn handle_robot_help() {
    let cmd = Cli::command();
    let help = build_robot_help(cmd);
    println!("{}", serde_json::to_string_pretty(&help).unwrap());
}

fn build_robot_help(cmd: clap::Command) -> RobotHelp {
    RobotHelp {
        program: cmd.get_name().to_string(),
        version: cmd.get_version().unwrap_or("unknown").to_string(),
        description: cmd.get_about().unwrap_or_default().to_string(),
        commands: cmd
            .get_subcommands()
            .map(|sub| build_robot_command(sub.clone()))
            .collect(),
    }
}

fn build_robot_command(cmd: clap::Command) -> RobotCommand {
    RobotCommand {
        name: cmd.get_name().to_string(),
        about: cmd.get_about().unwrap_or_default().to_string(),
        args: cmd.get_arguments().map(build_robot_arg).collect(),
        subcommands: cmd
            .get_subcommands()
            .map(|sub| build_robot_command(sub.clone()))
            .collect(),
    }
}

fn build_robot_arg(arg: &clap::Arg) -> RobotArg {
    RobotArg {
        name: arg.get_id().to_string(),
        long: arg.get_long().map(|s| format!("--{}", s)),
        short: arg.get_short(),
        help: arg.get_help().map(|s| s.to_string()).unwrap_or_default(),
        required: arg.is_required_set(),
        possible_values: arg
            .get_possible_values()
            .iter()
            .map(|v| v.get_name().to_string())
            .collect(),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Install panic hook FIRST, before anything else
    // This ensures panics are logged even during initialization
    panic_hook::init_panic_hook();

    let cli = Cli::parse();

    if cli.robot_help {
        handle_robot_help();
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
        },
        Some(Commands::Archive(args)) => handle_archive_command(args.command).await?,
        Some(Commands::Version) => println!("mcp-agent-mail v{}", env!("CARGO_PKG_VERSION")),
        None => {
            Cli::command().print_help()?;
        }
    }

    Ok(())
}

// --- Archive Command Handlers ---

async fn handle_archive_command(cmd: ArchiveCommands) -> anyhow::Result<()> {
    let archives_dir = std::path::Path::new("data/archives");

    match cmd {
        ArchiveCommands::Save { label, include_git } => {
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
        }

        ArchiveCommands::List { json } => {
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
        }

        ArchiveCommands::Restore { file, yes } => {
            use std::fs;
            use std::io::Write;

            let archive_path = std::path::Path::new(&file);
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
        }

        ArchiveCommands::ClearAndReset {
            archive,
            label,
            yes,
        } => {
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
                let save_cmd = ArchiveCommands::Save {
                    label: label.or_else(|| Some("pre-wipe".to_string())),
                    include_git: true,
                };
                // Box the recursive call to avoid infinite size
                Box::pin(handle_archive_command(save_cmd)).await?;
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
        }
    }

    Ok(())
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
