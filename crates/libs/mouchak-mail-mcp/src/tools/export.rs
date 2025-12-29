//! Export tool implementations

use mouchak_mail_core::{
    ctx::Ctx,
    model::{ModelManager, agent::AgentBmc, message::MessageBmc},
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::ExportMailboxParams;
use super::helpers;

pub async fn export_mailbox_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ExportMailboxParams,
) -> Result<CallToolResult, McpError> {
    let format = params.format.unwrap_or_else(|| "markdown".to_string());

    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agents = AgentBmc::list_all_for_project(ctx, mm, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let messages = MessageBmc::list_recent(ctx, mm, project.id, 1000)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let threads = MessageBmc::list_threads(ctx, mm, project.id.get(), 100)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    match format.as_str() {
        "json" => {
            let export = serde_json::json!({
                "project": {
                    "id": project.id,
                    "slug": project.slug,
                    "human_key": project.human_key,
                    "created_at": project.created_at.to_string(),
                },
                "agents": agents.iter().map(|a| serde_json::json!({
                    "id": a.id,
                    "name": a.name,
                    "program": a.program,
                    "model": a.model,
                    "task_description": a.task_description,
                })).collect::<Vec<_>>(),
                "messages": messages.iter().map(|m| serde_json::json!({
                    "id": m.id,
                    "sender_name": m.sender_name,
                    "subject": m.subject,
                    "body_md": m.body_md,
                    "thread_id": m.thread_id,
                    "importance": m.importance,
                    "created_ts": m.created_ts.to_string(),
                })).collect::<Vec<_>>(),
                "threads": threads.iter().map(|t| serde_json::json!({
                    "thread_id": t.thread_id,
                    "subject": t.subject,
                    "message_count": t.message_count,
                    "last_message_ts": t.last_message_ts.to_string(),
                })).collect::<Vec<_>>(),
                "exported_at": chrono::Utc::now().to_rfc3339(),
            });
            let json_str = serde_json::to_string_pretty(&export)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![Content::text(json_str)]))
        }
        "html" => {
            let mut html = format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mailbox Export: {}</title>
    <style>
        body {{ font-family: system-ui, -apple-system, sans-serif; max-width: 900px; margin: 0 auto; padding: 2rem; }}
        h1 {{ color: #1a1a1a; border-bottom: 2px solid #e0e0e0; padding-bottom: 0.5rem; }}
        h2 {{ color: #333; margin-top: 2rem; }}
        .message {{ background: #f5f5f5; border-radius: 8px; padding: 1rem; margin: 1rem 0; }}
        .message-header {{ font-weight: bold; color: #1976d2; }}
        .message-meta {{ color: #666; font-size: 0.9rem; }}
        .message-body {{ margin-top: 0.5rem; white-space: pre-wrap; }}
        .agent {{ display: inline-block; background: #e3f2fd; padding: 0.25rem 0.5rem; border-radius: 4px; margin: 0.25rem; }}
        .thread {{ background: #fff3e0; border-left: 4px solid #ff9800; padding: 0.5rem 1rem; margin: 0.5rem 0; }}
    </style>
</head>
<body>
    <h1>{} Mailbox Export</h1>
    <p>Project: {} | Exported: {}</p>
"#,
                project.human_key,
                project.human_key,
                project.slug,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
            );

            html.push_str("<h2>Agents</h2><div>");
            for a in &agents {
                html.push_str(&format!(
                    r#"<span class="agent">{} ({})</span>"#,
                    a.name, a.program
                ));
            }
            html.push_str("</div>");

            html.push_str("<h2>Threads</h2>");
            for t in &threads {
                html.push_str(&format!(
                    r#"<div class="thread"><strong>{}</strong> - {} messages (last: {})</div>"#,
                    t.subject, t.message_count, t.last_message_ts
                ));
            }

            html.push_str("<h2>Messages</h2>");
            for m in &messages {
                html.push_str(&format!(
                    r#"<div class="message">
    <div class="message-header">{}</div>
    <div class="message-meta">From: {} | {} | {}</div>
    <div class="message-body">{}</div>
</div>"#,
                    m.subject, m.sender_name, m.importance, m.created_ts, m.body_md
                ));
            }

            html.push_str("</body></html>");
            Ok(CallToolResult::success(vec![Content::text(html)]))
        }
        _ => {
            let mut md = format!(
                "# {} Mailbox Export\n\nProject: `{}`\nExported: {}\n\n",
                project.human_key,
                project.slug,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
            );

            md.push_str("## Agents\n\n");
            for a in &agents {
                md.push_str(&format!(
                    "- **{}** ({}) - {}\n",
                    a.name, a.program, a.task_description
                ));
            }

            md.push_str("\n## Threads\n\n");
            for t in &threads {
                md.push_str(&format!(
                    "- **{}** ({} messages, last: {})\n",
                    t.subject, t.message_count, t.last_message_ts
                ));
            }

            md.push_str("\n## Messages\n\n");
            for m in &messages {
                md.push_str(&format!(
                    "### {}\n\n**From:** {} | **Importance:** {} | **Date:** {}\n\n{}\n\n---\n\n",
                    m.subject, m.sender_name, m.importance, m.created_ts, m.body_md
                ));
            }

            Ok(CallToolResult::success(vec![Content::text(md)]))
        }
    }
}
