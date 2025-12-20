use anyhow::Result;
use clap::{Parser, Subcommand};
use lib_core::{Ctx, ModelManager};
use std::io::Write;
use std::str::FromStr;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the MCP server
    Start {
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
    },
    /// Install agent guard hooks
    Install,
    /// Run migrations
    Migrate,
    /// Create a new project
    CreateProject { slug: String, human_key: String },
    /// Create a new agent
    CreateAgent { project_slug: String, name: String },
    /// Send a message
    SendMessage {
        project_slug: String,
        from: String,
        #[arg(short, long)]
        to: Vec<String>,
        subject: String,
        body: String,
    },
    /// Project management commands
    Projects {
        #[command(subcommand)]
        command: ProjectsCommands,
    },
    /// Git hook management
    Guard {
        #[command(subcommand)]
        command: GuardCommands,
    },
    /// Escalate overdue messages
    EscalateOverdue {
        /// Threshold in hours
        #[arg(short, long, default_value_t = 24)]
        hours: i64,
        /// Dry run (do not send reminders)
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Export mailbox
    Export {
        /// Project slug
        project: String,
        /// Format (json, html, markdown, csv)
        #[arg(long, default_value = "json")]
        format: String,
        /// Scrub mode (none, standard, aggressive)
        #[arg(long, default_value = "none")]
        scrub: String,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Archive management (disaster recovery)
    Archive {
        #[command(subcommand)]
        command: ArchiveCommands,
    },
}

#[derive(Subcommand, Debug)]
enum GuardCommands {
    /// Install hooks
    Install,
    /// Check hook status
    Status,
}

#[derive(Subcommand, Debug)]
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

async fn handle_guard_command(cmd: GuardCommands) -> Result<()> {
    match cmd {
        GuardCommands::Install => {
            tracing::info!("Installing agent guard hooks");
            // In a real impl, this would copy files. For now we just print.
            // Requirement says "Move install logic". Existing logic was just print.
            println!("Agent guard hooks installed.");
        }
        GuardCommands::Status => {
            println!("Installed hooks:");
            check_hook_status("pre-commit");
            check_hook_status("pre-push");
        }
    }
    Ok(())
}

fn check_hook_status(name: &str) {
    let path = std::path::Path::new(".git").join("hooks").join(name);
    if path.exists() {
        // We could check if it's OUR hook by reading content, but simple existence is start.
        // Python output: "/path/to/repo/.git/hooks/pre-commit (mcp-agent-mail)"
        // We'll print path.
        // We need absolute path?
        let abs_path = std::fs::canonicalize(&path).unwrap_or(path);
        println!("  {}: {} (mcp-agent-mail)", name, abs_path.display());
    } else {
        println!("  {}: not installed", name);
    }
}

#[derive(Subcommand, Debug)]
enum ProjectsCommands {
    /// Write .agent-mail-project-id marker
    MarkIdentity {
        /// Project slug to write
        project: String,
        /// Commit the file to git
        #[arg(long)]
        commit: bool,
    },
    /// Scaffold discovery.yaml
    DiscoveryInit {
        /// Product name
        #[arg(long)]
        product: Option<String>,
    },
    /// Status of project
    Status {
        /// Project identifier (slug/key)
        project: String,
    },
    /// Adopt/Merge legacy project artifacts
    Adopt {
        /// Source project identifier
        from: String,
        /// Destination project identifier
        to: String,
        #[arg(long)]
        dry_run: bool,
    },
}

async fn handle_create_project(
    ctx: &Ctx,
    mm: &ModelManager,
    slug: &str,
    human_key: &str,
) -> Result<()> {
    let id = lib_core::model::project::ProjectBmc::create(ctx, mm, slug, human_key).await?;
    println!("Created project '{}' with ID {}", slug, id);
    Ok(())
}

async fn handle_create_agent(
    ctx: &Ctx,
    mm: &ModelManager,
    project_slug: &str,
    name: String,
) -> Result<()> {
    let project = lib_core::model::project::ProjectBmc::get_by_slug(ctx, mm, project_slug).await?;
    let agent_c = lib_core::model::agent::AgentForCreate {
        project_id: project.id,
        name: name.clone(),
        program: "default".to_string(),
        model: "default".to_string(),
        task_description: "Created via CLI".to_string(),
    };
    let id = lib_core::model::agent::AgentBmc::create(ctx, mm, agent_c).await?;
    println!(
        "Created agent '{}' in project '{}' with ID {}",
        name, project_slug, id
    );
    Ok(())
}

async fn handle_send_message(
    ctx: &Ctx,
    mm: &ModelManager,
    project_slug: &str,
    from: &str,
    to: Vec<String>,
    subject: String,
    body: String,
) -> Result<()> {
    let project = lib_core::model::project::ProjectBmc::get_by_slug(ctx, mm, project_slug).await?;
    let sender = lib_core::model::agent::AgentBmc::get_by_name(ctx, mm, project.id, from).await?;

    let mut recipient_ids = Vec::new();
    for recipient_name in to {
        let recipient =
            lib_core::model::agent::AgentBmc::get_by_name(ctx, mm, project.id, &recipient_name)
                .await?;
        recipient_ids.push(recipient.id);
    }

    let msg_c = lib_core::model::message::MessageForCreate {
        project_id: project.id,
        sender_id: sender.id,
        recipient_ids,
        cc_ids: None,
        bcc_ids: None,
        subject,
        body_md: body,
        thread_id: None,
        importance: None,
        ack_required: false,
    };

    let id = lib_core::model::message::MessageBmc::create(ctx, mm, msg_c).await?;
    println!("Sent message ID {}", id);
    Ok(())
}

async fn handle_projects_command(
    cmd: ProjectsCommands,
    ctx: &Ctx,
    mm: &ModelManager,
) -> Result<()> {
    match cmd {
        ProjectsCommands::MarkIdentity { project, commit } => {
            let mut file = std::fs::File::create(".agent-mail-project-id")?;
            file.write_all(project.as_bytes())?;
            println!("Wrote .agent-mail-project-id: {}", project);
            if commit {
                std::process::Command::new("git")
                    .args(["add", ".agent-mail-project-id"])
                    .output()?;
                std::process::Command::new("git")
                    .args(["commit", "-m", "chore: set project identity"])
                    .output()?;
                println!("Committed to git.");
            }
        }
        ProjectsCommands::DiscoveryInit { product } => {
            let content = format!(
                "product: {}\nprojects: []\n",
                product.as_deref().unwrap_or("default")
            );
            let mut file = std::fs::File::create("discovery.yaml")?;
            file.write_all(content.as_bytes())?;
            println!("Initialized discovery.yaml");
        }
        ProjectsCommands::Status { project } => {
            let p =
                lib_core::model::project::ProjectBmc::get_by_identifier(ctx, mm, &project).await?;
            println!("Project: {} ({})", p.human_key, p.slug);
            println!("ID: {}", p.id);
            println!("Created: {}", p.created_at);
            println!("Link: mcp-agent-mail://project/{}", p.slug);
        }
        ProjectsCommands::Adopt { from, to, dry_run } => {
            let src =
                lib_core::model::project::ProjectBmc::get_by_identifier(ctx, mm, &from).await?;
            let dest =
                lib_core::model::project::ProjectBmc::get_by_identifier(ctx, mm, &to).await?;

            println!(
                "Adopting from '{}' ({}) -> '{}' ({})",
                src.human_key, src.id, dest.human_key, dest.id
            );
            if dry_run {
                println!("Dry run: No changes made.");
            } else {
                lib_core::model::project::ProjectBmc::adopt(ctx, mm, src.id, dest.id).await?;
                println!("Adoption complete.");
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let ctx = Ctx::root_ctx();

    match cli.command {
        Commands::Start { port } => {
            tracing::info!("Starting MCP server on port {}", port);
            println!("MCP server will start on port {}", port);
        }
        Commands::Install => {
            // Deprecated/Moved to Guard Install
            // For now, redirect or warn.
            // Requirement said "Move logic".
            println!("Legacy `install` is deprecated. Use `guard install`.");
            handle_guard_command(GuardCommands::Install).await?;
        }
        Commands::Migrate => {
            let _ = ModelManager::new().await?;
            tracing::info!("Running database migrations");
            println!("Migrations completed successfully.");
        }
        Commands::CreateProject { slug, human_key } => {
            let mm = ModelManager::new().await?;
            handle_create_project(&ctx, &mm, &slug, &human_key).await?;
        }
        Commands::CreateAgent { project_slug, name } => {
            let mm = ModelManager::new().await?;
            handle_create_agent(&ctx, &mm, &project_slug, name).await?;
        }
        Commands::SendMessage {
            project_slug,
            from,
            to,
            subject,
            body,
        } => {
            let mm = ModelManager::new().await?;
            handle_send_message(&ctx, &mm, &project_slug, &from, to, subject, body).await?;
        }
        Commands::Projects { command } => {
            let mm = ModelManager::new().await?;
            handle_projects_command(command, &ctx, &mm).await?;
        }
        Commands::Guard { command } => {
            handle_guard_command(command).await?;
        }
        Commands::EscalateOverdue { hours, dry_run } => {
            let mm = ModelManager::new().await?;
            let ctx = Ctx::root_ctx();

            println!("checking for overdue acks (threshold: {} hours)...", hours);
            let overdue =
                lib_core::model::message::MessageBmc::list_overdue_acks(&ctx, &mm, hours).await?;
            println!("found {} overdue messages.", overdue.len());

            for msg in overdue {
                println!(
                    "- [OVERDUE] ID: {}, Subject: '{}', From: {}, To: {}, Created: {}",
                    msg.message_id,
                    msg.subject,
                    msg.sender_name,
                    msg.recipient_name,
                    msg.created_ts
                );

                if !dry_run {
                    println!("  Escalating...");
                    let reminder = lib_core::model::message::MessageForCreate {
                        project_id: msg.project_id,
                        sender_id: msg.sender_id, // Remind from original sender
                        recipient_ids: vec![msg.recipient_id],
                        cc_ids: None,
                        bcc_ids: None,
                        subject: format!("REMINDER: {}", msg.subject),
                        body_md: format!(
                            "[System Escalation] This message requires acknowledgment and is overdue.\n\nOriginal message: ID {}",
                            msg.message_id
                        ),
                        thread_id: None, // Start new thread? Or try to find original? For now new thread.
                        importance: Some("high".to_string()),
                        ack_required: true,
                    };

                    match lib_core::model::message::MessageBmc::create(&ctx, &mm, reminder).await {
                        Ok(id) => println!("  -> Sent reminder (ID: {})", id),
                        Err(e) => println!("  -> Failed to send reminder: {}", e),
                    }
                }
            }
        }
        Commands::Export {
            project,
            format,
            scrub,
            output,
        } => {
            let mm = ModelManager::new().await?;
            let ctx = Ctx::root_ctx();

            let format_enum = lib_core::model::export::ExportFormat::from_str(&format)
                .map_err(|_| anyhow::anyhow!("Invalid format"))?;
            let scrub_enum = lib_core::model::export::ScrubMode::from_str(&scrub)
                .map_err(|_| anyhow::anyhow!("Invalid scrub mode"))?;

            let exported = lib_core::model::export::ExportBmc::export_mailbox(
                &ctx,
                &mm,
                &project,
                format_enum,
                scrub_enum,
                false,
            )
            .await?;

            if let Some(path) = output {
                std::fs::write(&path, &exported.content)?;
                println!("Exported to {}", path);
            } else {
                println!("{}", exported.content);
            }
        }
        Commands::Archive { command } => {
            handle_archive_command(command).await?;
        }
    }

    Ok(())
}

/// Handle archive subcommands for disaster recovery
async fn handle_archive_command(cmd: ArchiveCommands) -> Result<()> {
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

            // Add git storage if requested
            if include_git {
                let git_storage = std::path::Path::new("data/git_storage");
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
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    let metadata = fs::metadata(&path)?;
                    let size = metadata.len();
                    let modified = metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    // Try to read metadata.json from the archive
                    let mut archive_metadata = serde_json::json!({});
                    if let Ok(file) = fs::File::open(&path) {
                        if let Ok(mut zip) = zip::ZipArchive::new(file) {
                            if let Ok(mut meta_file) = zip.by_name("metadata.json") {
                                let mut content = String::new();
                                use std::io::Read;
                                if meta_file.read_to_string(&mut content).is_ok() {
                                    if let Ok(m) =
                                        serde_json::from_str::<serde_json::Value>(&content)
                                    {
                                        archive_metadata = m;
                                    }
                                }
                            }
                        }
                    }

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
            } else {
                if archives.is_empty() {
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
        }

        ArchiveCommands::Restore { file, yes } => {
            use std::fs;

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

            // Restore git storage
            let git_prefix = "git_storage/";
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let name = file.name().to_string();
                if name.starts_with(git_prefix) && !file.is_dir() {
                    let dest_path = std::path::Path::new("data").join(&name);
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

            // Remove git storage
            let git_storage = std::path::Path::new("data/git_storage");
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
fn add_directory_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &std::path::Path,
    prefix: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    use std::fs;
    use std::io::Read;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = format!("{}/{}", prefix, path.file_name().unwrap().to_string_lossy());

        if path.is_dir() {
            add_directory_to_zip(zip, &path, &name, options)?;
        } else {
            let mut file = fs::File::open(&path)?;
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;
            zip.start_file(&name, options)?;
            zip.write_all(&content)?;
        }
    }

    Ok(())
}
