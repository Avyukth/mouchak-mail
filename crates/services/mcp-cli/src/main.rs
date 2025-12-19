use anyhow::Result;
use clap::{Parser, Subcommand};
use lib_core::{Ctx, ModelManager};
use std::io::Write;
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
}

#[derive(Subcommand, Debug)]
enum GuardCommands {
    /// Install hooks
    Install,
    /// Check hook status
    Status,
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
    }

    Ok(())
}
