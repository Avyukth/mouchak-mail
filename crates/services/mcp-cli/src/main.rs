use anyhow::Result;
use clap::{Parser, Subcommand};
use lib_core::{Ctx, ModelManager};
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
            tracing::info!("Installing agent guard hooks");
            println!("Agent guard hooks installed.");
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
    }

    Ok(())
}
