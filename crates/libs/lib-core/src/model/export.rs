//! Export functionality for mailbox data
//! 
//! Supports exporting messages in HTML, JSON, and Markdown formats.

use serde::{Deserialize, Serialize};
use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::model::message::MessageBmc;
use crate::model::project::ProjectBmc;
use crate::Result;

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Html,
    Json,
    Markdown,
    Csv,
}

impl std::str::FromStr for ExportFormat {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "html" => Self::Html,
            "md" | "markdown" => Self::Markdown,
            "csv" => Self::Csv,
            _ => Self::Json, // default
        })
    }
}

/// Exported mailbox data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedMailbox {
    pub project_slug: String,
    pub project_name: String,
    pub message_count: usize,
    pub exported_at: String,
    pub content: String,
    pub format: String,
}

pub struct ExportBmc;

impl ExportBmc {
    /// Export a project's mailbox to the specified format
    pub async fn export_mailbox(
        ctx: &Ctx,
        mm: &ModelManager,
        project_slug: &str,
        format: ExportFormat,
        _include_attachments: bool,
    ) -> Result<ExportedMailbox> {
        // Get project
        let project = ProjectBmc::get_by_slug(ctx, mm, project_slug).await?;
        
        // Get recent messages (limit to 100 for export)
        let messages = MessageBmc::list_recent(ctx, mm, project.id, 100).await?;
        
        let exported_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let message_count = messages.len();
        
        let content = match format {
            ExportFormat::Html => Self::render_html(&project.slug, &messages),
            ExportFormat::Json => Self::render_json(&messages)?,
            ExportFormat::Markdown => Self::render_markdown(&project.slug, &messages),
            ExportFormat::Csv => Self::render_csv(&messages)?,
        };
        
        let format_str = match format {
            ExportFormat::Html => "html",
            ExportFormat::Json => "json",
            ExportFormat::Markdown => "markdown",
            ExportFormat::Csv => "csv",
        };
        
        Ok(ExportedMailbox {
            project_slug: project.slug.clone(),
            project_name: project.human_key.clone(),
            message_count,
            exported_at,
            content,
            format: format_str.to_string(),
        })
    }
    
    fn render_html(project_slug: &str, messages: &[crate::model::message::Message]) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str(&format!("<title>Mailbox Export - {}</title>\n", project_slug));
        html.push_str("<style>
body { font-family: system-ui, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
.message { border: 1px solid #ddd; padding: 15px; margin: 10px 0; border-radius: 8px; }
.subject { font-weight: bold; font-size: 1.1em; }
.meta { color: #666; font-size: 0.9em; margin: 5px 0; }
.body { margin-top: 10px; white-space: pre-wrap; }
</style>\n</head>\n<body>\n");
        html.push_str(&format!("<h1>Mailbox Export: {}</h1>\n", project_slug));
        html.push_str(&format!("<p>Total messages: {}</p>\n", messages.len()));
        
        for msg in messages {
            html.push_str("<div class=\"message\">\n");
            html.push_str(&format!("<div class=\"subject\">{}</div>\n", 
                html_escape(&msg.subject)));
            html.push_str(&format!("<div class=\"meta\">From: {} | {}</div>\n", 
                html_escape(&msg.sender_name),
                msg.created_ts.format("%Y-%m-%d %H:%M")));
            html.push_str(&format!("<div class=\"body\">{}</div>\n", 
                html_escape(&msg.body_md)));
            html.push_str("</div>\n");
        }
        
        html.push_str("</body>\n</html>");
        html
    }
    
    fn render_json(messages: &[crate::model::message::Message]) -> Result<String> {
        Ok(serde_json::to_string_pretty(messages)?)
    }
    
    fn render_markdown(project_slug: &str, messages: &[crate::model::message::Message]) -> String {
        let mut md = String::new();
        md.push_str(&format!("# Mailbox Export: {}\n\n", project_slug));
        md.push_str(&format!("Total messages: {}\n\n---\n\n", messages.len()));
        
        for msg in messages {
            md.push_str(&format!("## {}\n\n", msg.subject));
            md.push_str(&format!("**From:** {} | **Date:** {}\n\n", 
                msg.sender_name,
                msg.created_ts.format("%Y-%m-%d %H:%M")));
            md.push_str(&format!("{}\n\n---\n\n", msg.body_md));
        }
        
        md
    }

    fn render_csv(messages: &[crate::model::message::Message]) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);
        
        // Header
        wtr.write_record(["id", "created_at", "sender", "subject", "body"])
            .map_err(|e| crate::Error::InvalidInput(format!("CSV Error: {}", e)))?;
            
        // Rows
        for msg in messages {
            wtr.write_record(&[
                msg.id.to_string(),
                msg.created_ts.format("%Y-%m-%d %H:%M:%S").to_string(),
                msg.sender_name.clone(),
                msg.subject.clone(),
                msg.body_md.clone(),
            ]).map_err(|e| crate::Error::InvalidInput(format!("CSV Error: {}", e)))?;
        }
        
        let data = wtr.into_inner().map_err(|e| crate::Error::InvalidInput(format!("CSV Error: {}", e)))?;
        Ok(String::from_utf8(data).unwrap_or_default())
    }
}

impl ExportBmc {
    pub async fn commit_archive(
        ctx: &Ctx,
        mm: &ModelManager,
        project_slug: &str,
        message: &str,
    ) -> Result<String> {
        // 1. Export in Markdown (default for archive)
        let exported = Self::export_mailbox(ctx, mm, project_slug, ExportFormat::Markdown, true).await?;
        
        // 2. Determine file path in repo
        let now = chrono::Utc::now();
        let filename = format!("{}_{}.md", project_slug, now.format("%Y%m%d_%H%M%S"));
        let rel_path = std::path::Path::new("mailboxes").join(project_slug).join(&filename);
        
        // 3. Open Repo
        let repo = crate::store::git_store::open_repo(&mm.repo_root)?;
        
        // 4. Commit
        let oid = crate::store::git_store::commit_file(
            &repo,
            &rel_path,
            &exported.content,
            message,
            "MCP Agent Mail", // Committer name
            "mcp@generic-agent.ai", // Committer email
        )?;
        
        Ok(oid.to_string())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

