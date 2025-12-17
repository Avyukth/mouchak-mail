use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::Result;
use crate::store::git_store;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub project_id: i64,
    pub sender_id: i64,
    pub thread_id: Option<String>,
    pub subject: String,
    pub body_md: String,
    pub importance: String,
    pub ack_required: bool,
    pub created_ts: NaiveDateTime,
    pub attachments: Vec<Value>, // Use Vec<Value> for attachments
    pub sender_name: String, // Added sender_name for inbox display
}

#[derive(Deserialize, Serialize)]
pub struct MessageForCreate {
    pub project_id: i64,
    pub sender_id: i64,
    pub recipient_ids: Vec<i64>,     // "to" recipients
    pub cc_ids: Option<Vec<i64>>,    // "cc" recipients
    pub bcc_ids: Option<Vec<i64>>,   // "bcc" recipients
    pub subject: String,
    pub body_md: String,
    pub thread_id: Option<String>,
    pub importance: Option<String>,
}

pub struct MessageBmc;

impl MessageBmc {
    pub async fn create(_ctx: &Ctx, mm: &ModelManager, msg_c: MessageForCreate) -> Result<i64> {
        let db = mm.db();

        // 1. Insert into DB
        let thread_id = msg_c.thread_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let importance = msg_c.importance.unwrap_or("normal".to_string());

        // Helper to serialize attachments (empty for now)
        let attachments_json = "[]";

        let stmt = db.prepare(
            r#"
            INSERT INTO messages (project_id, sender_id, thread_id, subject, body_md, importance, attachments)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        ).await?;

        let mut rows = stmt.query((
            msg_c.project_id,
            msg_c.sender_id,
            thread_id.as_str(),
            msg_c.subject.as_str(),
            msg_c.body_md.as_str(),
            importance.as_str(),
            attachments_json
        )).await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput("Failed to create message".into()));
        };

        // 2. Insert Recipients with recipient_type (BATCHED)
        let mut recipient_tuples = Vec::new();
        for rid in &msg_c.recipient_ids { recipient_tuples.push((*rid, "to")); }
        if let Some(cc) = &msg_c.cc_ids { for rid in cc { recipient_tuples.push((*rid, "cc")); } }
        if let Some(bcc) = &msg_c.bcc_ids { for rid in bcc { recipient_tuples.push((*rid, "bcc")); } }

        if !recipient_tuples.is_empty() {
             // Constuct batch insert query: ... VALUES (?, ?, ?), (?, ?, ?)
             let mut query = String::from("INSERT INTO message_recipients (message_id, agent_id, recipient_type) VALUES ");
             let mut params: Vec<libsql::Value> = Vec::with_capacity(recipient_tuples.len() * 3);
             
             for (i, (rid, rtype)) in recipient_tuples.iter().enumerate() {
                 if i > 0 { query.push_str(", "); }
                 query.push_str("(?, ?, ?)");
                 params.push(id.into());
                 params.push((*rid).into());
                 params.push((*rtype).to_string().into());
             }

             let stmt = db.prepare(&query).await?;
             stmt.execute(libsql::params::Params::Positional(params)).await?;
        }

        // 3. Git Operations - DEFERRED to background task for low latency
        // Collect data needed for background git commit
        let stmt = db.prepare("SELECT slug FROM projects WHERE id = ?").await?;
        let mut rows = stmt.query([msg_c.project_id]).await?;
        let project_slug: String = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(crate::Error::ProjectNotFound(format!("ID: {}", msg_c.project_id)));
        };

        // Batch fetch sender and recipient names
        let mut needed_ids = vec![msg_c.sender_id];
        needed_ids.extend_from_slice(&msg_c.recipient_ids);
        
        let placeholders = needed_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT id, name FROM agents WHERE id IN ({})", placeholders);
        
        let stmt = db.prepare(&query).await?;
        let params: Vec<libsql::Value> = needed_ids.iter().map(|&id| id.into()).collect();
        let mut rows = stmt.query(libsql::params::Params::Positional(params)).await?;
        
        let mut agent_map = std::collections::HashMap::new();
        while let Some(row) = rows.next().await? {
            let aid: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            agent_map.insert(aid, name);
        }

        let sender_name = agent_map.remove(&msg_c.sender_id).ok_or_else(|| {
            crate::Error::AgentNotFound(format!("ID: {}", msg_c.sender_id))
        })?;

        let mut recipient_names = Vec::new();
        for recipient_id in &msg_c.recipient_ids {
            if let Some(name) = agent_map.get(recipient_id) {
                recipient_names.push(name.clone());
            } else {
                warn!("Recipient Name not found for ID: {}", recipient_id);
                recipient_names.push(format!("Unknown-{}", recipient_id));
            }
        }

        // Spawn background task for git operations (non-blocking)
        let git_lock = mm.git_lock.clone();
        let repo_root = mm.repo_root.clone();
        let subject = msg_c.subject.clone();
        let body_md = msg_c.body_md.clone();
        let thread_id_clone = thread_id.clone();
        let importance_clone = importance.clone();

        tokio::spawn(async move {
            if let Err(e) = commit_message_to_git(
                git_lock,
                repo_root,
                id,
                &project_slug,
                &sender_name,
                &recipient_names,
                &subject,
                &body_md,
                &thread_id_clone,
                &importance_clone,
            ).await {
                warn!("Background git commit failed for message {}: {}", id, e);
            }
        });

        Ok(id)
    }

    pub async fn list_inbox_for_agent(_ctx: &Ctx, mm: &ModelManager, project_id: i64, agent_id: i64, limit: i64) -> Result<Vec<Message>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT
                m.id, m.project_id, m.sender_id, ag.name as sender_name, m.thread_id, m.subject, m.body_md,
                m.importance, m.ack_required, m.created_ts, m.attachments
            FROM messages AS m
            JOIN message_recipients AS mr ON m.id = mr.message_id
            JOIN agents AS ag ON m.sender_id = ag.id
            WHERE mr.agent_id = ? AND m.project_id = ?
            ORDER BY m.created_ts DESC
            LIMIT ?
            "#
        ).await?;

        let mut rows = stmt.query((agent_id, project_id, limit)).await?;
        let mut messages = Vec::new();

        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let project_id: i64 = row.get(1)?;
            let sender_id: i64 = row.get(2)?;
            let sender_name: String = row.get(3)?;
            let thread_id: Option<String> = row.get(4)?;
            let subject: String = row.get(5)?;
            let body_md: String = row.get(6)?;
            let importance: String = row.get(7)?;
            let ack_required: bool = row.get(8)?;
            let created_ts_str: String = row.get(9)?;
            let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            let attachments_str: String = row.get(10)?;
            let attachments: Vec<Value> = serde_json::from_str(&attachments_str)?;

            messages.push(Message {
                id,
                project_id,
                sender_id,
                sender_name,
                thread_id,
                subject,
                body_md,
                importance,
                ack_required,
                created_ts,
                attachments,
            });
        }
        Ok(messages)
    }

    /// List outbox messages SENT BY an agent
    pub async fn list_outbox_for_agent(_ctx: &Ctx, mm: &ModelManager, project_id: i64, agent_id: i64, limit: i64) -> Result<Vec<Message>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT
                m.id, m.project_id, m.sender_id, ag.name as sender_name, m.thread_id, m.subject, m.body_md,
                m.importance, m.ack_required, m.created_ts, m.attachments
            FROM messages AS m
            JOIN agents AS ag ON m.sender_id = ag.id
            WHERE m.sender_id = ? AND m.project_id = ?
            ORDER BY m.created_ts DESC
            LIMIT ?
            "#
        ).await?;

        let mut rows = stmt.query((agent_id, project_id, limit)).await?;
        let mut messages = Vec::new();

        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let project_id: i64 = row.get(1)?;
            let sender_id: i64 = row.get(2)?;
            let sender_name: String = row.get(3)?;
            let thread_id: Option<String> = row.get(4)?;
            let subject: String = row.get(5)?;
            let body_md: String = row.get(6)?;
            let importance: String = row.get(7)?;
            let ack_required: bool = row.get(8)?;
            let created_ts_str: String = row.get(9)?;
            let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            let attachments_str: String = row.get(10)?;
            let attachments: Vec<Value> = serde_json::from_str(&attachments_str)?;

            messages.push(Message {
                id,
                project_id,
                sender_id,
                sender_name,
                thread_id,
                subject,
                body_md,
                importance,
                ack_required,
                created_ts,
                attachments,
            });
        }
        Ok(messages)
    }

    pub async fn get(_ctx: &Ctx, mm: &ModelManager, message_id: i64) -> Result<Message> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT
                m.id, m.project_id, m.sender_id, ag.name as sender_name, m.thread_id, m.subject, m.body_md,
                m.importance, m.ack_required, m.created_ts, m.attachments
            FROM messages AS m
            JOIN agents AS ag ON m.sender_id = ag.id
            WHERE m.id = ?
            "#
        ).await?;

        let mut rows = stmt.query([message_id]).await?;

        if let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let project_id: i64 = row.get(1)?;
            let sender_id: i64 = row.get(2)?;
            let sender_name: String = row.get(3)?;
            let thread_id: Option<String> = row.get(4)?;
            let subject: String = row.get(5)?;
            let body_md: String = row.get(6)?;
            let importance: String = row.get(7)?;
            let ack_required: bool = row.get(8)?;
            let created_ts_str: String = row.get(9)?;
            let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            let attachments_str: String = row.get(10)?;
            let attachments: Vec<Value> = serde_json::from_str(&attachments_str)?;

            Ok(Message {
                id,
                project_id,
                sender_id,
                sender_name,
                thread_id,
                subject,
                body_md,
                importance,
                ack_required,
                created_ts,
                attachments,
            })
        } else {
            Err(crate::Error::MessageNotFound(message_id))
        }
    }

    pub async fn list_by_thread(_ctx: &Ctx, mm: &ModelManager, project_id: i64, thread_id: &str) -> Result<Vec<Message>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT
                m.id, m.project_id, m.sender_id, ag.name as sender_name, m.thread_id, m.subject, m.body_md,
                m.importance, m.ack_required, m.created_ts, m.attachments
            FROM messages AS m
            JOIN agents AS ag ON m.sender_id = ag.id
            WHERE m.project_id = ? AND m.thread_id = ?
            ORDER BY m.created_ts ASC
            "#
        ).await?;

        let mut rows = stmt.query((project_id, thread_id)).await?;
        let mut messages = Vec::new();

        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let project_id: i64 = row.get(1)?;
            let sender_id: i64 = row.get(2)?;
            let sender_name: String = row.get(3)?;
            let thread_id: Option<String> = row.get(4)?;
            let subject: String = row.get(5)?;
            let body_md: String = row.get(6)?;
            let importance: String = row.get(7)?;
            let ack_required: bool = row.get(8)?;
            let created_ts_str: String = row.get(9)?;
            let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            let attachments_str: String = row.get(10)?;
            let attachments: Vec<Value> = serde_json::from_str(&attachments_str)?;

            messages.push(Message {
                id,
                project_id,
                sender_id,
                sender_name,
                thread_id,
                subject,
                body_md,
                importance,
                ack_required,
                created_ts,
                attachments,
            });
        }
        Ok(messages)
    }

    /// Full-text search messages using FTS5
    pub async fn search(_ctx: &Ctx, mm: &ModelManager, project_id: i64, query: &str, limit: i64) -> Result<Vec<Message>> {
        let db = mm.db();

        // Use FTS5 MATCH for full-text search, joining back to messages table
        // FTS5 MATCH interprets unquoted "term" as column:value - wrap in quotes for pure text search
        let fts_query = format!("\"{}\"", query.replace("\"", "\"\""));
        
        let stmt = db.prepare(
            r#"
            SELECT
                m.id, m.project_id, m.sender_id, ag.name as sender_name, m.thread_id, m.subject, m.body_md,
                m.importance, m.ack_required, m.created_ts, m.attachments
            FROM messages AS m
            JOIN agents AS ag ON m.sender_id = ag.id
            WHERE m.project_id = ? AND m.id IN (
                SELECT rowid FROM messages_fts WHERE messages_fts MATCH ?
            )
            ORDER BY m.created_ts DESC
            LIMIT ?
            "#
        ).await?;

        let mut rows = stmt.query((project_id, fts_query.as_str(), limit)).await?;
        let mut messages = Vec::new();

        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let project_id: i64 = row.get(1)?;
            let sender_id: i64 = row.get(2)?;
            let sender_name: String = row.get(3)?;
            let thread_id: Option<String> = row.get(4)?;
            let subject: String = row.get(5)?;
            let body_md: String = row.get(6)?;
            let importance: String = row.get(7)?;
            let ack_required: bool = row.get(8)?;
            let created_ts_str: String = row.get(9)?;
            let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            let attachments_str: String = row.get(10)?;
            let attachments: Vec<Value> = serde_json::from_str(&attachments_str)?;

            messages.push(Message {
                id,
                project_id,
                sender_id,
                sender_name,
                thread_id,
                subject,
                body_md,
                importance,
                ack_required,
                created_ts,
                attachments,
            });
        }
        Ok(messages)
    }

    /// Mark a message as read by a recipient
    pub async fn mark_read(_ctx: &Ctx, mm: &ModelManager, message_id: i64, agent_id: i64) -> Result<()> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db.prepare(
            r#"
            UPDATE message_recipients SET read_ts = ? WHERE message_id = ? AND agent_id = ? AND read_ts IS NULL
            "#
        ).await?;
        stmt.execute((now_str, message_id, agent_id)).await?;
        Ok(())
    }

    /// Acknowledge a message by a recipient
    pub async fn acknowledge(_ctx: &Ctx, mm: &ModelManager, message_id: i64, agent_id: i64) -> Result<()> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        // Also mark as read if not already
        let stmt = db.prepare(
            r#"
            UPDATE message_recipients
            SET ack_ts = ?, read_ts = COALESCE(read_ts, ?)
            WHERE message_id = ? AND agent_id = ?
            "#
        ).await?;
        stmt.execute((now_str.as_str(), now_str.as_str(), message_id, agent_id)).await?;
        Ok(())
    }

    /// List distinct threads for a project
    pub async fn list_threads(_ctx: &Ctx, mm: &ModelManager, project_id: i64, limit: i64) -> Result<Vec<ThreadSummary>> {
        let db = mm.db();

        let stmt = db.prepare(
            r#"
            SELECT
                m.thread_id,
                MIN(m.subject) as subject,
                COUNT(*) as message_count,
                MAX(m.created_ts) as last_message_ts
            FROM messages AS m
            WHERE m.project_id = ? AND m.thread_id IS NOT NULL
            GROUP BY m.thread_id
            ORDER BY last_message_ts DESC
            LIMIT ?
            "#
        ).await?;

        let mut rows = stmt.query((project_id, limit)).await?;
        let mut threads = Vec::new();

        while let Some(row) = rows.next().await? {
            let thread_id: String = row.get(0)?;
            let subject: String = row.get(1)?;
            let message_count: i64 = row.get(2)?;
            let last_message_ts_str: String = row.get(3)?;
            let last_message_ts = NaiveDateTime::parse_from_str(&last_message_ts_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            threads.push(ThreadSummary {
                thread_id,
                subject,
                message_count: message_count as usize,
                last_message_ts,
            });
        }
        Ok(threads)
    }

    /// List recent messages for a project
    pub async fn list_recent(_ctx: &Ctx, mm: &ModelManager, project_id: i64, limit: i64) -> Result<Vec<Message>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT
                m.id, m.project_id, m.sender_id, ag.name as sender_name, m.thread_id, m.subject, m.body_md,
                m.importance, m.ack_required, m.created_ts, m.attachments
            FROM messages AS m
            JOIN agents AS ag ON m.sender_id = ag.id
            WHERE m.project_id = ?
            ORDER BY m.created_ts DESC
            LIMIT ?
            "#
        ).await?;

        let mut rows = stmt.query((project_id, limit)).await?;
        let mut messages = Vec::new();

        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let project_id: i64 = row.get(1)?;
            let sender_id: i64 = row.get(2)?;
            let sender_name: String = row.get(3)?;
            let thread_id: Option<String> = row.get(4)?;
            let subject: String = row.get(5)?;
            let body_md: String = row.get(6)?;
            let importance: String = row.get(7)?;
            let ack_required: bool = row.get(8)?;
            let created_ts_str: String = row.get(9)?;
            let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            let attachments_str: String = row.get(10)?;
            let attachments: Vec<Value> = serde_json::from_str(&attachments_str)?;

            messages.push(Message {
                id,
                project_id,
                sender_id,
                sender_name,
                thread_id,
                subject,
                body_md,
                importance,
                ack_required,
                created_ts,
                attachments,
            });
        }
        Ok(messages)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub thread_id: String,
    pub subject: String,
    pub message_count: usize,
    pub last_message_ts: NaiveDateTime,
}

/// Background git commit for message archival
/// This runs async after the DB commit returns, keeping API latency low
#[allow(clippy::too_many_arguments)]
async fn commit_message_to_git(
    git_lock: Arc<Mutex<()>>,
    repo_root: PathBuf,
    id: i64,
    project_slug: &str,
    sender_name: &str,
    recipient_names: &[String],
    subject: &str,
    body_md: &str,
    thread_id: &str,
    importance: &str,
) -> Result<()> {
    // Construct paths
    let now = chrono::Utc::now();
    let y_dir = now.format("%Y").to_string();
    let m_dir = now.format("%m").to_string();
    let created_iso = now.format("%Y-%m-%dT%H-%M-%SZ").to_string();

    let subject_slug = slug::slugify(subject);
    let filename = format!("{}__{}__{}.md", created_iso, subject_slug, id);

    let project_root = PathBuf::from("projects").join(project_slug);
    let canonical_path = project_root.join("messages").join(&y_dir).join(&m_dir).join(&filename);

    let outbox_path = project_root.join("agents").join(sender_name).join("outbox").join(&y_dir).join(&m_dir).join(&filename);

    let mut inbox_paths = Vec::new();
    for recipient_name in recipient_names {
        inbox_paths.push(
            project_root.join("agents").join(recipient_name).join("inbox").join(&y_dir).join(&m_dir).join(&filename)
        );
    }

    // Content
    let frontmatter = serde_json::json!({
        "id": id,
        "project": project_slug,
        "from": sender_name,
        "to": recipient_names,
        "subject": subject,
        "thread_id": thread_id,
        "created": created_iso,
        "importance": importance,
    });
    let content = format!("---json\n{}\n---\n\n{}", serde_json::to_string_pretty(&frontmatter)?, body_md);

    // Git Operations - serialized to prevent lock contention
    let _git_guard = git_lock.lock().await;

    let repo = git_store::open_repo(&repo_root)?;
    let commit_msg = format!("mail: {} -> {} | {}", sender_name, recipient_names.join(", "), subject);

    let workdir = repo.workdir().ok_or(crate::Error::InvalidInput("No workdir".into()))?;

    fn write_file(root: &std::path::Path, rel: &std::path::Path, content: &str) -> Result<()> {
         let full = root.join(rel);
         if let Some(p) = full.parent() {
             std::fs::create_dir_all(p)?;
         }
         std::fs::write(full, content)?;
         Ok(())
    }

    write_file(workdir, &canonical_path, &content)?;
    write_file(workdir, &outbox_path, &content)?;
    for inbox_path in &inbox_paths {
        write_file(workdir, inbox_path, &content)?;
    }

    // Collect all paths to commit
    let mut all_paths = vec![canonical_path.clone()];
    all_paths.push(outbox_path.clone());
    all_paths.extend(inbox_paths.clone());

    let all_paths_as_ref: Vec<&std::path::Path> = all_paths.iter().map(|p| p.as_path()).collect();

    git_store::commit_paths(
        &repo,
        &all_paths_as_ref,
        &commit_msg,
        "mcp-bot",
        "mcp-bot@localhost",
    )?;

    info!("Background git commit succeeded for message {}", id);
    Ok(())
}
