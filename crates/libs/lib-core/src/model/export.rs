//! Export functionality for mailbox data
//!
//! Supports exporting messages in HTML, JSON, and Markdown formats.

use crate::Result;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::model::message::MessageBmc;
use crate::model::project::ProjectBmc;
use serde::{Deserialize, Serialize};

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

use lazy_static::lazy_static;
use regex::Regex;

/// Scrubbing mode for privacy protection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrubMode {
    #[default]
    None,
    Standard,   // Email, Phone
    Aggressive, // Email, Phone, Names
}

impl std::str::FromStr for ScrubMode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "standard" => Self::Standard,
            "aggressive" => Self::Aggressive,
            _ => Self::None,
        })
    }
}

pub struct Scrubber {
    mode: ScrubMode,
}

impl Scrubber {
    pub fn new(mode: ScrubMode) -> Self {
        Self { mode }
    }

    pub fn scrub(&self, text: &str) -> String {
        if self.mode == ScrubMode::None {
            return text.to_string();
        }

        let mut cleaned = text.to_string();

        lazy_static! {
            static ref EMAIL_RE: Regex = Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").unwrap();
            // Simplified phone: generic patterns like 123-456-7890 or (123) 456 7890
            static ref PHONE_RE: Regex = Regex::new(r"\b\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap();
        }

        // Standard Scrubbing
        cleaned = EMAIL_RE.replace_all(&cleaned, "[EMAIL]").to_string();
        cleaned = PHONE_RE.replace_all(&cleaned, "[PHONE]").to_string();

        // Aggressive Scrubbing
        if self.mode == ScrubMode::Aggressive {
            // In a real system, we'd use NLP or a dictionary.
            // Here, we'll assuming specific sender/recipient names passed in are handled separately,
            // but for body text, aggressive might be too broad.
            // For this implementation, we will act heavily on Metadata in render_* methods,
            // but for body text, "Aggressive" will mostly just do the same as Standard + maybe credit cards?
            // Let's add Credit Cards.
            lazy_static! {
                static ref CC_RE: Regex = Regex::new(r"\b(?:\d[ -]*?){13,16}\b").unwrap();
                // SSN: \b\d{3}-\d{2}-\d{4}\b
                static ref SSN_RE: Regex = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap();
            }
            cleaned = CC_RE.replace_all(&cleaned, "[CREDIT-CARD]").to_string();
            cleaned = SSN_RE.replace_all(&cleaned, "[SSN]").to_string();
        }

        cleaned
    }

    pub fn scrub_name(&self, name: &str) -> String {
        match self.mode {
            ScrubMode::Aggressive => "[REDACTED-NAME]".to_string(),
            _ => name.to_string(),
        }
    }
}

pub struct ExportBmc;

impl ExportBmc {
    /// Export a project's mailbox to the specified format
    pub async fn export_mailbox(
        ctx: &Ctx,
        mm: &ModelManager,
        project_slug: &str,
        format: ExportFormat,
        scrub_mode: ScrubMode,
        _include_attachments: bool,
    ) -> Result<ExportedMailbox> {
        // Get project
        let project = ProjectBmc::get_by_slug(ctx, mm, project_slug).await?;

        // Get recent messages (limit to 100 for export)
        let messages = MessageBmc::list_recent(ctx, mm, project.id, 100).await?;

        let exported_at = chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        let message_count = messages.len();

        let scrubber = Scrubber::new(scrub_mode);

        let content = match format {
            ExportFormat::Html => Self::render_html(&project.slug, &messages, &scrubber),
            ExportFormat::Json => Self::render_json(&messages, &scrubber)?,
            ExportFormat::Markdown => Self::render_markdown(&project.slug, &messages, &scrubber),
            ExportFormat::Csv => Self::render_csv(&messages, &scrubber)?,
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

    fn render_html(
        project_slug: &str,
        messages: &[crate::model::message::Message],
        scrubber: &Scrubber,
    ) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str(&format!(
            "<title>Mailbox Export - {}</title>\n",
            project_slug
        ));
        html.push_str(
            "<style>
body { font-family: system-ui, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
.message { border: 1px solid #ddd; padding: 15px; margin: 10px 0; border-radius: 8px; }
.subject { font-weight: bold; font-size: 1.1em; }
.meta { color: #666; font-size: 0.9em; margin: 5px 0; }
.body { margin-top: 10px; white-space: pre-wrap; }
</style>\n</head>\n<body>\n",
        );
        html.push_str(&format!("<h1>Mailbox Export: {}</h1>\n", project_slug));
        html.push_str(&format!("<p>Total messages: {}</p>\n", messages.len()));

        for msg in messages {
            let scrubbed_subject = scrubber.scrub(&msg.subject);
            let scrubbed_body = scrubber.scrub(&msg.body_md);
            let scrubbed_sender = scrubber.scrub_name(&msg.sender_name);

            html.push_str("<div class=\"message\">\n");
            html.push_str(&format!(
                "<div class=\"subject\">{}</div>\n",
                html_escape(&scrubbed_subject)
            ));
            html.push_str(&format!(
                "<div class=\"meta\">From: {} | {}</div>\n",
                html_escape(&scrubbed_sender),
                msg.created_ts.format("%Y-%m-%d %H:%M")
            ));
            html.push_str(&format!(
                "<div class=\"body\">{}</div>\n",
                html_escape(&scrubbed_body)
            ));
            html.push_str("</div>\n");
        }

        html.push_str("</body>\n</html>");
        html
    }

    fn render_json(
        messages: &[crate::model::message::Message],
        scrubber: &Scrubber,
    ) -> Result<String> {
        // For JSON, we might want to clone and scrub fields.
        // Or create a scrubbed struct.
        // Easiest is to convert to Value, walk it? Or just map to a new Vec.
        // Let's use a temporary struct or just modify if we can.
        // Messy to redefine struct. Let's use serde_json::Value
        let mut vals = Vec::new();
        for msg in messages {
            let mut val = serde_json::to_value(msg)?;
            if let Some(obj) = val.as_object_mut() {
                if let Some(s) = obj.get("subject").and_then(|v| v.as_str()) {
                    obj.insert(
                        "subject".to_string(),
                        serde_json::Value::String(scrubber.scrub(s)),
                    );
                }
                if let Some(s) = obj.get("body_md").and_then(|v| v.as_str()) {
                    obj.insert(
                        "body_md".to_string(),
                        serde_json::Value::String(scrubber.scrub(s)),
                    );
                }
                if let Some(s) = obj.get("sender_name").and_then(|v| v.as_str()) {
                    obj.insert(
                        "sender_name".to_string(),
                        serde_json::Value::String(scrubber.scrub_name(s)),
                    );
                }
                // Scrub recipient names if available in future, but Message struct currently doesn't inline them nicely in JSON without extra work?
                // `Message` struct has `sender_name`.
            }
            vals.push(val);
        }
        Ok(serde_json::to_string_pretty(&vals)?)
    }

    fn render_markdown(
        project_slug: &str,
        messages: &[crate::model::message::Message],
        scrubber: &Scrubber,
    ) -> String {
        let mut md = String::new();
        md.push_str(&format!("# Mailbox Export: {}\n\n", project_slug));
        md.push_str(&format!("Total messages: {}\n\n---\n\n", messages.len()));

        for msg in messages {
            let scrubbed_subject = scrubber.scrub(&msg.subject);
            let scrubbed_body = scrubber.scrub(&msg.body_md);
            let scrubbed_sender = scrubber.scrub_name(&msg.sender_name);

            md.push_str(&format!("## {}\n\n", scrubbed_subject));
            md.push_str(&format!(
                "**From:** {} | **Date:** {}\n\n",
                scrubbed_sender,
                msg.created_ts.format("%Y-%m-%d %H:%M")
            ));
            md.push_str(&format!("{}\n\n---\n\n", scrubbed_body));
        }

        md
    }

    fn render_csv(
        messages: &[crate::model::message::Message],
        scrubber: &Scrubber,
    ) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Header
        wtr.write_record(["id", "created_at", "sender", "subject", "body"])
            .map_err(|e| crate::Error::InvalidInput(format!("CSV Error: {}", e)))?;

        // Rows
        for msg in messages {
            wtr.write_record(&[
                msg.id.to_string(),
                msg.created_ts.format("%Y-%m-%d %H:%M:%S").to_string(),
                scrubber.scrub_name(&msg.sender_name),
                scrubber.scrub(&msg.subject),
                scrubber.scrub(&msg.body_md),
            ])
            .map_err(|e| crate::Error::InvalidInput(format!("CSV Error: {}", e)))?;
        }

        let data = wtr
            .into_inner()
            .map_err(|e| crate::Error::InvalidInput(format!("CSV Error: {}", e)))?;
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
        let exported = Self::export_mailbox(
            ctx,
            mm,
            project_slug,
            ExportFormat::Markdown,
            ScrubMode::None,
            true,
        )
        .await?;

        // 2. Determine file path in repo
        let now = chrono::Utc::now();
        let filename = format!("{}_{}.md", project_slug, now.format("%Y%m%d_%H%M%S"));
        let rel_path = std::path::Path::new("mailboxes")
            .join(project_slug)
            .join(&filename);

        // 3. Git Operations - serialized to prevent lock contention
        let _git_guard = mm.git_lock.lock().await;

        // Use cached repository to prevent FD exhaustion
        let repo_arc = mm.get_repo().await?;
        let repo = repo_arc.lock().await;

        // 4. Commit
        let oid = crate::store::git_store::commit_file(
            &repo,
            &rel_path,
            &exported.content,
            message,
            "MCP Agent Mail",       // Committer name
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
