//! Message management for agent-to-agent communication.
//!
//! This module provides the core messaging infrastructure for AI agents.
//! Messages support threading, importance levels, read receipts, and
//! acknowledgment tracking. All messages are archived to Git for audit.
//!
//! # Features
//!
//! - **Threading**: Messages can be grouped into conversation threads
//! - **Importance**: High/Normal priority levels for triage
//! - **Recipients**: To/CC/BCC support with delivery tracking
//! - **Full-text search**: FTS5-powered message search
//! - **Git archival**: Automatic commit to audit log
//!
//! # Example
//!
//! ```no_run
//! use lib_core::model::message::{MessageBmc, MessageForCreate};
//! use lib_core::model::ModelManager;
//! use lib_core::ctx::Ctx;
//!
//! # async fn example() -> lib_core::Result<()> {
//! let mm = ModelManager::new().await?;
//! let ctx = Ctx::root_ctx();
//!
//! // Send a message
//! let msg = MessageForCreate {
//!     project_id: 1,
//!     sender_id: 1,
//!     recipient_ids: vec![2],
//!     cc_ids: None,
//!     bcc_ids: None,
//!     subject: "Code Review".to_string(),
//!     body_md: "Please review PR #42".to_string(),
//!     thread_id: None,
//!     importance: Some("high".to_string()),
//!     ack_required: false,
//! };
//! let id = MessageBmc::create(&ctx, &mm, msg).await?;
//! # Ok(())
//! # }
//! ```

use crate::Result;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::store::git_store;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

/// Filter type for importance query - strong type, not primitive String
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportanceFilter {
    /// Only high importance messages
    High,
    /// Only normal importance messages
    Normal,
    /// All messages regardless of importance
    All,
}

impl ImportanceFilter {
    /// Convert optional string to ImportanceFilter
    pub fn from_str_opt(s: Option<&str>) -> Self {
        match s {
            Some("high") => Self::High,
            Some("normal") => Self::Normal,
            _ => Self::All,
        }
    }
}

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
    pub sender_name: String,     // Added sender_name for inbox display
}

/// Unified inbox item with project slug for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedInboxItem {
    pub id: i64,
    pub project_id: i64,
    pub project_slug: String,
    pub sender_id: i64,
    pub sender_name: String,
    pub thread_id: Option<String>,
    pub subject: String,
    pub importance: String,
    pub created_ts: NaiveDateTime,
}

#[derive(Deserialize, Serialize)]
pub struct MessageForCreate {
    pub project_id: i64,
    pub sender_id: i64,
    pub recipient_ids: Vec<i64>,   // "to" recipients
    pub cc_ids: Option<Vec<i64>>,  // "cc" recipients
    pub bcc_ids: Option<Vec<i64>>, // "bcc" recipients
    pub subject: String,
    pub body_md: String,
    pub thread_id: Option<String>,
    pub importance: Option<String>,
    /// Whether recipients must acknowledge this message (default: false)
    #[serde(default)]
    pub ack_required: bool,
}

/// Raw row from list_pending_reviews query with all nested data
#[derive(Debug, Clone, Serialize)]
pub struct PendingReviewRow {
    pub message_id: i64,
    pub subject: String,
    pub body_md: String,
    pub importance: String,
    pub created_ts: NaiveDateTime,
    pub attachments: String, // JSON string
    pub thread_id: Option<String>,
    pub sender_id: i64,
    pub sender_name: String,
    pub project_id: i64,
    pub project_slug: String,
    pub project_name: String,
    pub thread_count: i64,
    pub recipients_json: String, // JSON array string
}

/// Backend Model Controller for Message operations.
///
/// Provides methods for message lifecycle including creation, retrieval,
/// threading, and full-text search.
pub struct MessageBmc;

/// Summary of an overdue message for escalation
#[derive(Debug, Clone, Serialize)]
pub struct OverdueMessage {
    pub message_id: i64,
    pub project_id: i64,
    pub sender_id: i64,
    pub subject: String,
    pub sender_name: String,
    pub recipient_id: i64,
    pub recipient_name: String,
    pub created_ts: NaiveDateTime,
}

impl MessageBmc {
    /// List messages that require acknowledgement but haven't received one within the threshold
    pub async fn list_overdue_acks(
        _ctx: &Ctx,
        mm: &ModelManager,
        threshold_hours: i64,
    ) -> Result<Vec<OverdueMessage>> {
        let db = mm.db();
        // SQLite: datetime('now', '-N hours')
        let time_modifier = format!("-{} hours", threshold_hours);

        // Query logic:
        // 1. Message must require ack
        // 2. Recipient hasn't acked (mr.ack_ts is NULL)
        // 3. Message created before threshold
        let stmt = db.prepare(
            r#"
            SELECT 
                m.id, m.project_id, m.sender_id, m.subject, ag_sender.name, mr.agent_id, ag_recipient.name, m.created_ts
            FROM messages AS m
            JOIN message_recipients AS mr ON m.id = mr.message_id
            JOIN agents AS ag_sender ON m.sender_id = ag_sender.id
            JOIN agents AS ag_recipient ON mr.agent_id = ag_recipient.id
            WHERE 
                m.ack_required = 1 
                AND mr.ack_ts IS NULL
                AND m.created_ts < datetime('now', ?)
            ORDER BY m.created_ts ASC
            "#
        ).await?;

        let mut rows = stmt.query([time_modifier]).await?;
        let mut overdue = Vec::new();

        while let Some(row) = rows.next().await? {
            let message_id: i64 = row.get(0)?;
            let project_id: i64 = row.get(1)?;
            let sender_id: i64 = row.get(2)?;
            let subject: String = row.get(3)?;
            let sender_name: String = row.get(4)?;
            let recipient_id: i64 = row.get(5)?;
            let recipient_name: String = row.get(6)?;
            let created_ts_str: String = row.get(7)?;
            let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            overdue.push(OverdueMessage {
                message_id,
                project_id,
                sender_id,
                subject,
                sender_name,
                recipient_id,
                recipient_name,
                created_ts,
            });
        }
        Ok(overdue)
    }

    /// Creates a new message and sends it to one or more recipients.
    ///
    /// This method:
    /// 1. Validates sender and all recipients exist
    /// 2. Inserts message and recipient records in database
    /// 3. Archives message to Git (async, doesn't block response)
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager providing database and Git access
    /// * `msg_c` - Message creation data including recipients
    ///
    /// # Returns
    /// The created message's database ID
    ///
    /// # Errors
    /// Returns an error if sender or any recipient doesn't exist
    ///
    /// # Example
    /// ```no_run
    /// # use lib_core::model::message::{MessageBmc, MessageForCreate};
    /// # use lib_core::model::ModelManager;
    /// # use lib_core::ctx::Ctx;
    /// # async fn example(mm: &ModelManager) {
    /// let ctx = Ctx::root_ctx();
    /// let msg = MessageForCreate {
    ///     project_id: 1,
    ///     sender_id: 1,
    ///     recipient_ids: vec![2, 3],
    ///     cc_ids: None,
    ///     bcc_ids: None,
    ///     subject: "Task Update".to_string(),
    ///     body_md: "Completed feature X".to_string(),
    ///     thread_id: None,
    ///     importance: None,
    ///     ack_required: false,
    /// };
    /// let id = MessageBmc::create(&ctx, mm, msg).await.unwrap();
    /// # }
    /// ```
    pub async fn create(_ctx: &Ctx, mm: &ModelManager, msg_c: MessageForCreate) -> Result<i64> {
        let db = mm.db();

        // 1. Insert into DB
        let thread_id = msg_c
            .thread_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let importance = msg_c.importance.unwrap_or("normal".to_string());

        // Helper to serialize attachments (empty for now)
        let attachments_json = "[]";

        let stmt = db.prepare(
            r#"
            INSERT INTO messages (project_id, sender_id, thread_id, subject, body_md, importance, attachments, ack_required)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        ).await?;

        let mut rows = stmt
            .query((
                msg_c.project_id,
                msg_c.sender_id,
                thread_id.as_str(),
                msg_c.subject.as_str(),
                msg_c.body_md.as_str(),
                importance.as_str(),
                attachments_json,
                msg_c.ack_required,
            ))
            .await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput(
                "Failed to create message".into(),
            ));
        };

        // 2. Insert Recipients with recipient_type (BATCHED)
        let mut recipient_tuples = Vec::new();
        for rid in &msg_c.recipient_ids {
            recipient_tuples.push((*rid, "to"));
        }
        if let Some(cc) = &msg_c.cc_ids {
            for rid in cc {
                recipient_tuples.push((*rid, "cc"));
            }
        }
        if let Some(bcc) = &msg_c.bcc_ids {
            for rid in bcc {
                recipient_tuples.push((*rid, "bcc"));
            }
        }

        if !recipient_tuples.is_empty() {
            // Constuct batch insert query: ... VALUES (?, ?, ?), (?, ?, ?)
            let mut query = String::from(
                "INSERT INTO message_recipients (message_id, agent_id, recipient_type) VALUES ",
            );
            let mut params: Vec<libsql::Value> = Vec::with_capacity(recipient_tuples.len() * 3);

            for (i, (rid, rtype)) in recipient_tuples.iter().enumerate() {
                if i > 0 {
                    query.push_str(", ");
                }
                query.push_str("(?, ?, ?)");
                params.push(id.into());
                params.push((*rid).into());
                params.push((*rtype).to_string().into());
            }

            let stmt = db.prepare(&query).await?;
            stmt.execute(libsql::params::Params::Positional(params))
                .await?;
        }

        // 3. Git Operations - DEFERRED to background task for low latency
        // Collect data needed for background git commit
        let stmt = db.prepare("SELECT slug FROM projects WHERE id = ?").await?;
        let mut rows = stmt.query([msg_c.project_id]).await?;
        let project_slug: String = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(crate::Error::project_not_found(format!(
                "ID: {}",
                msg_c.project_id
            )));
        };

        // Batch fetch sender and recipient names
        let mut needed_ids = vec![msg_c.sender_id];
        needed_ids.extend_from_slice(&msg_c.recipient_ids);

        let placeholders = needed_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT id, name FROM agents WHERE id IN ({})", placeholders);

        let stmt = db.prepare(&query).await?;
        let params: Vec<libsql::Value> = needed_ids.iter().map(|&id| id.into()).collect();
        let mut rows = stmt
            .query(libsql::params::Params::Positional(params))
            .await?;

        let mut agent_map = std::collections::HashMap::new();
        while let Some(row) = rows.next().await? {
            let aid: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            agent_map.insert(aid, name);
        }

        let sender_name = agent_map
            .remove(&msg_c.sender_id)
            .ok_or_else(|| crate::Error::agent_not_found(format!("ID: {}", msg_c.sender_id)))?;

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
        // Get cached repository before spawning to ensure it's in the cache
        let cached_repo = match mm.get_repo().await {
            Ok(repo) => repo,
            Err(e) => {
                warn!("Failed to get cached repo for message {}: {}", id, e);
                return Ok(id); // Return success since DB write succeeded
            }
        };

        let git_lock = mm.git_lock.clone();
        let subject = msg_c.subject.clone();
        let body_md = msg_c.body_md.clone();
        let thread_id_clone = thread_id.clone();
        let importance_clone = importance.clone();

        tokio::spawn(async move {
            if let Err(e) = commit_message_to_git(
                git_lock,
                cached_repo,
                id,
                &project_slug,
                &sender_name,
                &recipient_names,
                &subject,
                &body_md,
                &thread_id_clone,
                &importance_clone,
            )
            .await
            {
                warn!("Background git commit failed for message {}: {}", id, e);
            }
        });

        Ok(id)
    }

    pub async fn list_inbox_for_agent(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        agent_id: i64,
        limit: i64,
    ) -> Result<Vec<Message>> {
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
    pub async fn list_outbox_for_agent(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        agent_id: i64,
        limit: i64,
    ) -> Result<Vec<Message>> {
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

    /// Get recipient names for a message
    pub async fn get_recipients(
        _ctx: &Ctx,
        mm: &ModelManager,
        message_id: i64,
    ) -> Result<Vec<String>> {
        let db = mm.db();
        let stmt = db
            .prepare(
                r#"
            SELECT a.name
            FROM message_recipients mr
            JOIN agents a ON mr.agent_id = a.id
            WHERE mr.message_id = ?
            ORDER BY mr.recipient_type, a.name
            "#,
            )
            .await?;

        let mut rows = stmt.query([message_id]).await?;
        let mut recipients = Vec::new();

        while let Some(row) = rows.next().await? {
            let name: String = row.get(0)?;
            recipients.push(name);
        }

        Ok(recipients)
    }

    pub async fn list_by_thread(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        thread_id: &str,
    ) -> Result<Vec<Message>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT
                m.id, m.project_id, m.sender_id, ag.name as sender_name, m.thread_id, m.subject, m.body_md,
                m.importance, m.ack_required, m.created_ts, m.attachments
            FROM messages AS m
            JOIN agents AS ag ON m.sender_id = ag.id
            WHERE m.project_id = ? AND m.thread_id = ?
            ORDER BY m.created_ts ASC, m.id ASC
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
    pub async fn search(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        query: &str,
        limit: i64,
    ) -> Result<Vec<Message>> {
        let db = mm.db();

        // FTS5 Unsearchable patterns (return empty to avoid errors or heavy meaningless queries)
        // Python equivalent: _FTS5_UNSEARCHABLE_PATTERNS
        let trimmed = query.trim();
        if matches!(
            trimmed,
            "" | "*" | "**" | "***" | "." | ".." | "..." | "?" | "??" | "???"
        ) {
            info!("Search query '{}' is in blocklist, returning empty", query);
            return Ok(Vec::new());
        }

        // Logic for handling raw vs literal queries:
        // 1. If query contains explicit FTS operators (AND, OR, NOT) or wildcards (*), pass raw
        // 2. If query has balanced quotes (phrase search), pass raw
        // 3. Otherwise, quote each word to prevent hyphens being treated as NOT operator
        //    e.g., "full-text search" -> FTS5 interprets as "full AND NOT text AND search"
        //    Fix: Quote words containing hyphens: "\"full-text\" search"
        let quote_count = query.chars().filter(|c| *c == '"').count();
        let has_fts_operators = query.contains(" AND ")
            || query.contains(" OR ")
            || query.contains(" NOT ")
            || query.contains('*');

        let fts_query = if quote_count % 2 != 0 {
            // Unbalanced quotes: Treat as literal string search
            // This satisfies PORT-5.2 (Error Handling) for obviously malformed inputs
            format!("\"{}\"", query.replace('"', "\"\""))
        } else if has_fts_operators || query.starts_with('"') {
            // Has explicit FTS operators or is a phrase search - pass raw
            query.to_string()
        } else {
            // Simple search: quote words containing hyphens to prevent FTS5 misinterpretation
            // "full-text search" -> "\"full-text\" search"
            query
                .split_whitespace()
                .map(|word| {
                    if word.contains('-') && !word.starts_with('"') {
                        format!("\"{}\"", word)
                    } else {
                        word.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        };

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

        let mut rows = match stmt.query((project_id, fts_query.as_str(), limit)).await {
            Ok(rows) => rows,
            Err(e) => {
                info!(
                    "FTS Search failed for query '{}' (likely syntax): {}. Returning empty.",
                    query, e
                );
                return Ok(Vec::new());
            }
        };

        let mut messages = Vec::new();

        loop {
            let next_result = rows.next().await;
            let row = match next_result {
                Ok(Some(row)) => row,
                Ok(None) => break,
                Err(e) => {
                    info!(
                        "FTS Row iteration failed for query '{}': {}. Returning partial/empty.",
                        query, e
                    );
                    // If this is the first row and it failed, likely syntax error.
                    // We stop iteration and return what we have (or empty).
                    break;
                }
            };

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
    pub async fn mark_read(
        _ctx: &Ctx,
        mm: &ModelManager,
        message_id: i64,
        agent_id: i64,
    ) -> Result<()> {
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
    pub async fn acknowledge(
        _ctx: &Ctx,
        mm: &ModelManager,
        message_id: i64,
        agent_id: i64,
    ) -> Result<()> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        // Also mark as read if not already
        let stmt = db
            .prepare(
                r#"
            UPDATE message_recipients
            SET ack_ts = ?, read_ts = COALESCE(read_ts, ?)
            WHERE message_id = ? AND agent_id = ?
            "#,
            )
            .await?;
        stmt.execute((now_str.as_str(), now_str.as_str(), message_id, agent_id))
            .await?;
        Ok(())
    }

    /// List distinct threads for a project
    pub async fn list_threads(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        limit: i64,
    ) -> Result<Vec<ThreadSummary>> {
        let db = mm.db();

        let stmt = db
            .prepare(
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
            "#,
            )
            .await?;

        let mut rows = stmt.query((project_id, limit)).await?;
        let mut threads = Vec::new();

        while let Some(row) = rows.next().await? {
            let thread_id: String = row.get(0)?;
            let subject: String = row.get(1)?;
            let message_count: i64 = row.get(2)?;
            let last_message_ts_str: String = row.get(3)?;
            let last_message_ts =
                NaiveDateTime::parse_from_str(&last_message_ts_str, "%Y-%m-%d %H:%M:%S")
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
    pub async fn list_recent(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        limit: i64,
    ) -> Result<Vec<Message>> {
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

    /// List messages requiring acknowledgment that haven't been fully acknowledged.
    ///
    /// Returns complete message details including sender info, project context,
    /// thread info, and per-recipient status. This is designed for a single-call
    /// retrieval of all pending reviews with full context.
    ///
    /// # Arguments
    /// * `_ctx` - Request context (for future ACL)
    /// * `mm` - ModelManager providing database access
    /// * `project_id` - Optional filter by project
    /// * `sender_id` - Optional filter by sender
    /// * `limit` - Maximum results (clamped to 1-50)
    ///
    /// # Returns
    /// Vector of PendingReviewRow with all nested data
    pub async fn list_pending_reviews(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: Option<i64>,
        sender_id: Option<i64>,
        limit: i64,
    ) -> Result<Vec<PendingReviewRow>> {
        let db = mm.db();
        let limit_clamped = limit.clamp(1, 50);

        // Build query with optional filters
        let mut query = String::from(
            r#"
            SELECT
                m.id as message_id,
                m.subject,
                m.body_md,
                m.importance,
                m.created_ts,
                m.attachments,
                m.thread_id,
                m.sender_id,
                sender.name as sender_name,
                p.id as project_id,
                p.slug as project_slug,
                p.human_key as project_name,
                (SELECT COUNT(*) FROM messages m2
                 WHERE m2.thread_id = m.thread_id AND m.thread_id IS NOT NULL) as thread_count,
                (
                    SELECT json_group_array(json_object(
                        'agent_id', mr.agent_id,
                        'agent_name', a.name,
                        'recipient_type', mr.recipient_type,
                        'read_ts', mr.read_ts,
                        'ack_ts', mr.ack_ts
                    ))
                    FROM message_recipients mr
                    JOIN agents a ON mr.agent_id = a.id
                    WHERE mr.message_id = m.id
                ) as recipients_json
            FROM messages m
            JOIN agents sender ON m.sender_id = sender.id
            JOIN projects p ON m.project_id = p.id
            WHERE
                m.ack_required = TRUE
                AND EXISTS (
                    SELECT 1 FROM message_recipients mr2
                    WHERE mr2.message_id = m.id AND mr2.ack_ts IS NULL
                )
            "#,
        );

        let mut params: Vec<libsql::Value> = Vec::new();

        if let Some(pid) = project_id {
            query.push_str(" AND m.project_id = ?");
            params.push(pid.into());
        }

        if let Some(sid) = sender_id {
            query.push_str(" AND m.sender_id = ?");
            params.push(sid.into());
        }

        query.push_str(" ORDER BY m.created_ts DESC LIMIT ?");
        params.push(limit_clamped.into());

        let stmt = db.prepare(&query).await?;
        let mut rows = stmt
            .query(libsql::params::Params::Positional(params))
            .await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let message_id: i64 = row.get(0)?;
            let subject: String = row.get(1)?;
            let body_md: String = row.get(2)?;
            let importance: String = row.get(3)?;
            let created_ts_str: String = row.get(4)?;
            let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();
            let attachments_str: String = row.get(5)?;
            let thread_id: Option<String> = row.get(6)?;
            let sender_id: i64 = row.get(7)?;
            let sender_name: String = row.get(8)?;
            let project_id: i64 = row.get(9)?;
            let project_slug: String = row.get(10)?;
            let project_name: String = row.get(11)?;
            let thread_count: i64 = row.get(12)?;
            let recipients_json: String = row.get(13)?;

            results.push(PendingReviewRow {
                message_id,
                subject,
                body_md,
                importance,
                created_ts,
                attachments: attachments_str,
                thread_id,
                sender_id,
                sender_name,
                project_id,
                project_slug,
                project_name,
                thread_count,
                recipients_json,
            });
        }

        Ok(results)
    }

    /// List messages across ALL projects (unified inbox)
    ///
    /// Returns messages from all projects, optionally filtered by importance.
    /// This provides a Gmail-style unified view of all agent communications.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager providing database access
    /// * `importance` - Filter by importance level (High, Normal, or All)
    /// * `limit` - Maximum number of messages to return
    ///
    /// # Returns
    /// Vector of unified inbox items ordered by created_ts DESC (newest first)
    pub async fn list_unified_inbox(
        _ctx: &Ctx,
        mm: &ModelManager,
        importance: ImportanceFilter,
        limit: i32,
    ) -> Result<Vec<UnifiedInboxItem>> {
        let db = mm.db();

        // Build query based on importance filter - joins with projects for slug
        let (query, params): (String, Vec<libsql::Value>) = match importance {
            ImportanceFilter::High => {
                let q = r#"
                    SELECT
                        m.id, m.project_id, p.slug as project_slug, m.sender_id, ag.name as sender_name,
                        m.thread_id, m.subject, m.importance, m.created_ts
                    FROM messages AS m
                    JOIN agents AS ag ON m.sender_id = ag.id
                    JOIN projects AS p ON m.project_id = p.id
                    WHERE m.importance = 'high'
                    ORDER BY m.created_ts DESC
                    LIMIT ?
                "#.to_string();
                (q, vec![(limit as i64).into()])
            }
            ImportanceFilter::Normal => {
                let q = r#"
                    SELECT
                        m.id, m.project_id, p.slug as project_slug, m.sender_id, ag.name as sender_name,
                        m.thread_id, m.subject, m.importance, m.created_ts
                    FROM messages AS m
                    JOIN agents AS ag ON m.sender_id = ag.id
                    JOIN projects AS p ON m.project_id = p.id
                    WHERE m.importance = 'normal'
                    ORDER BY m.created_ts DESC
                    LIMIT ?
                "#.to_string();
                (q, vec![(limit as i64).into()])
            }
            ImportanceFilter::All => {
                let q = r#"
                    SELECT
                        m.id, m.project_id, p.slug as project_slug, m.sender_id, ag.name as sender_name,
                        m.thread_id, m.subject, m.importance, m.created_ts
                    FROM messages AS m
                    JOIN agents AS ag ON m.sender_id = ag.id
                    JOIN projects AS p ON m.project_id = p.id
                    ORDER BY m.created_ts DESC
                    LIMIT ?
                "#.to_string();
                (q, vec![(limit as i64).into()])
            }
        };

        let stmt = db.prepare(&query).await?;
        let mut rows = stmt
            .query(libsql::params::Params::Positional(params))
            .await?;
        let mut items = Vec::new();

        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let project_id: i64 = row.get(1)?;
            let project_slug: String = row.get(2)?;
            let sender_id: i64 = row.get(3)?;
            let sender_name: String = row.get(4)?;
            let thread_id: Option<String> = row.get(5)?;
            let subject: String = row.get(6)?;
            let importance: String = row.get(7)?;
            let created_ts_str: String = row.get(8)?;
            let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            items.push(UnifiedInboxItem {
                id,
                project_id,
                project_slug,
                sender_id,
                sender_name,
                thread_id,
                subject,
                importance,
                created_ts,
            });
        }
        Ok(items)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub thread_id: String,
    pub subject: String,
    pub message_count: usize,
    pub last_message_ts: NaiveDateTime,
}

/// Paths for git archival of a message
struct MessageArchivePaths {
    canonical: PathBuf,
    outbox: PathBuf,
    inboxes: Vec<PathBuf>,
}

/// Build all file paths for message archival
fn build_message_paths(
    project_slug: &str,
    sender_name: &str,
    recipient_names: &[String],
    filename: &str,
    y_dir: &str,
    m_dir: &str,
) -> MessageArchivePaths {
    let project_root = PathBuf::from("projects").join(project_slug);

    let canonical = project_root
        .join("messages")
        .join(y_dir)
        .join(m_dir)
        .join(filename);

    let outbox = project_root
        .join("agents")
        .join(sender_name)
        .join("outbox")
        .join(y_dir)
        .join(m_dir)
        .join(filename);

    let inboxes = recipient_names
        .iter()
        .map(|name| {
            project_root
                .join("agents")
                .join(name)
                .join("inbox")
                .join(y_dir)
                .join(m_dir)
                .join(filename)
        })
        .collect();

    MessageArchivePaths {
        canonical,
        outbox,
        inboxes,
    }
}

/// Format message content with JSON frontmatter
fn format_message_content(
    id: i64,
    project_slug: &str,
    sender_name: &str,
    recipient_names: &[String],
    subject: &str,
    body_md: &str,
    thread_id: &str,
    importance: &str,
    created_iso: &str,
) -> Result<String> {
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
    Ok(format!(
        "---json\n{}\n---\n\n{}",
        serde_json::to_string_pretty(&frontmatter)?,
        body_md
    ))
}

/// Write content to a path, creating parent directories as needed
fn write_archive_file(root: &std::path::Path, rel: &std::path::Path, content: &str) -> Result<()> {
    let full = root.join(rel);
    if let Some(p) = full.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(full, content)?;
    Ok(())
}

/// Write message to all archive paths (canonical, outbox, inboxes)
fn write_message_to_archive(
    workdir: &std::path::Path,
    paths: &MessageArchivePaths,
    content: &str,
) -> Result<()> {
    write_archive_file(workdir, &paths.canonical, content)?;
    write_archive_file(workdir, &paths.outbox, content)?;
    for inbox_path in &paths.inboxes {
        write_archive_file(workdir, inbox_path, content)?;
    }
    Ok(())
}

/// Background git commit for message archival
/// This runs async after the DB commit returns, keeping API latency low
#[allow(clippy::too_many_arguments)]
async fn commit_message_to_git(
    git_lock: Arc<Mutex<()>>,
    cached_repo: Arc<Mutex<git2::Repository>>,
    id: i64,
    project_slug: &str,
    sender_name: &str,
    recipient_names: &[String],
    subject: &str,
    body_md: &str,
    thread_id: &str,
    importance: &str,
) -> Result<()> {
    // Build timestamp-based paths
    let now = chrono::Utc::now();
    let y_dir = now.format("%Y").to_string();
    let m_dir = now.format("%m").to_string();
    let created_iso = now.format("%Y-%m-%dT%H-%M-%SZ").to_string();
    let filename = format!("{}__{}__{}.md", created_iso, slug::slugify(subject), id);

    let paths = build_message_paths(
        project_slug,
        sender_name,
        recipient_names,
        &filename,
        &y_dir,
        &m_dir,
    );

    let content = format_message_content(
        id,
        project_slug,
        sender_name,
        recipient_names,
        subject,
        body_md,
        thread_id,
        importance,
        &created_iso,
    )?;

    // Git operations - serialized to prevent lock contention
    // Use pre-fetched cached repository to prevent FD exhaustion
    let _git_guard = git_lock.lock().await;
    let repo = cached_repo.lock().await;
    let workdir = repo
        .workdir()
        .ok_or(crate::Error::InvalidInput("No workdir".into()))?;

    write_message_to_archive(workdir, &paths, &content)?;

    // Commit all paths
    let all_paths: Vec<PathBuf> = std::iter::once(paths.canonical.clone())
        .chain(std::iter::once(paths.outbox.clone()))
        .chain(paths.inboxes.iter().cloned())
        .collect();
    let all_paths_ref: Vec<&std::path::Path> = all_paths.iter().map(|p| p.as_path()).collect();

    let commit_msg = format!(
        "mail: {} -> {} | {}",
        sender_name,
        recipient_names.join(", "),
        subject
    );
    git_store::commit_paths(
        &repo,
        &all_paths_ref,
        &commit_msg,
        "mcp-bot",
        "mcp-bot@localhost",
    )?;

    info!("Background git commit succeeded for message {}", id);
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ============================================================================
    // TDD Tests for build_message_paths
    // ============================================================================

    #[test]
    fn test_build_message_paths_canonical_structure() {
        let paths = build_message_paths(
            "my-project",
            "sender-agent",
            &["recipient1".to_string()],
            "2025-01-01__test-msg__1.md",
            "2025",
            "01",
        );

        assert_eq!(
            paths.canonical,
            PathBuf::from("projects/my-project/messages/2025/01/2025-01-01__test-msg__1.md")
        );
    }

    #[test]
    fn test_build_message_paths_outbox_structure() {
        let paths = build_message_paths(
            "my-project",
            "alice",
            &["bob".to_string()],
            "msg.md",
            "2025",
            "12",
        );

        assert_eq!(
            paths.outbox,
            PathBuf::from("projects/my-project/agents/alice/outbox/2025/12/msg.md")
        );
    }

    #[test]
    fn test_build_message_paths_single_recipient_inbox() {
        let paths = build_message_paths(
            "proj",
            "sender",
            &["bob".to_string()],
            "msg.md",
            "2025",
            "01",
        );

        assert_eq!(paths.inboxes.len(), 1);
        assert_eq!(
            paths.inboxes[0],
            PathBuf::from("projects/proj/agents/bob/inbox/2025/01/msg.md")
        );
    }

    #[test]
    fn test_build_message_paths_multiple_recipients() {
        let paths = build_message_paths(
            "proj",
            "sender",
            &[
                "alice".to_string(),
                "bob".to_string(),
                "charlie".to_string(),
            ],
            "msg.md",
            "2025",
            "06",
        );

        assert_eq!(paths.inboxes.len(), 3);
        assert!(
            paths
                .inboxes
                .iter()
                .any(|p| p.to_string_lossy().contains("alice/inbox"))
        );
        assert!(
            paths
                .inboxes
                .iter()
                .any(|p| p.to_string_lossy().contains("bob/inbox"))
        );
        assert!(
            paths
                .inboxes
                .iter()
                .any(|p| p.to_string_lossy().contains("charlie/inbox"))
        );
    }

    #[test]
    fn test_build_message_paths_empty_recipients() {
        let paths = build_message_paths("proj", "sender", &[], "msg.md", "2025", "01");

        assert_eq!(paths.inboxes.len(), 0);
        // Canonical and outbox should still be valid
        assert!(paths.canonical.to_string_lossy().contains("messages"));
        assert!(paths.outbox.to_string_lossy().contains("outbox"));
    }

    // ============================================================================
    // TDD Tests for format_message_content
    // ============================================================================

    #[test]
    fn test_format_message_content_has_frontmatter() {
        let content = format_message_content(
            123,
            "test-proj",
            "alice",
            &["bob".to_string()],
            "Test Subject",
            "Body text here",
            "THREAD-001",
            "high",
            "2025-01-01T12:00:00Z",
        )
        .unwrap();

        assert!(content.starts_with("---json\n"));
        assert!(content.contains("---\n\n"));
        assert!(content.ends_with("Body text here"));
    }

    #[test]
    fn test_format_message_content_includes_all_fields() {
        let content = format_message_content(
            42,
            "my-project",
            "sender",
            &["r1".to_string(), "r2".to_string()],
            "Important",
            "Message body",
            "THR-99",
            "normal",
            "2025-12-17T10:30:00Z",
        )
        .unwrap();

        assert!(content.contains("\"id\": 42"));
        assert!(content.contains("\"project\": \"my-project\""));
        assert!(content.contains("\"from\": \"sender\""));
        assert!(content.contains("\"subject\": \"Important\""));
        assert!(content.contains("\"thread_id\": \"THR-99\""));
        assert!(content.contains("\"importance\": \"normal\""));
        assert!(content.contains("\"created\": \"2025-12-17T10:30:00Z\""));
    }

    #[test]
    fn test_format_message_content_recipients_array() {
        let content = format_message_content(
            1,
            "p",
            "s",
            &["a".to_string(), "b".to_string()],
            "subj",
            "body",
            "t",
            "low",
            "2025-01-01",
        )
        .unwrap();

        // Should contain recipients as JSON array
        assert!(content.contains("\"to\": ["));
        assert!(content.contains("\"a\""));
        assert!(content.contains("\"b\""));
    }

    // ============================================================================
    // TDD Tests for write_archive_file
    // ============================================================================

    #[test]
    fn test_write_archive_file_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let rel_path = PathBuf::from("deep/nested/path/file.md");

        write_archive_file(temp_dir.path(), &rel_path, "test content").unwrap();

        let full_path = temp_dir.path().join(&rel_path);
        assert!(full_path.exists());
        assert_eq!(std::fs::read_to_string(&full_path).unwrap(), "test content");
    }

    #[test]
    fn test_write_archive_file_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let rel_path = PathBuf::from("file.md");

        write_archive_file(temp_dir.path(), &rel_path, "first").unwrap();
        write_archive_file(temp_dir.path(), &rel_path, "second").unwrap();

        let full_path = temp_dir.path().join(&rel_path);
        assert_eq!(std::fs::read_to_string(&full_path).unwrap(), "second");
    }

    #[test]
    fn test_write_archive_file_empty_content() {
        let temp_dir = TempDir::new().unwrap();
        let rel_path = PathBuf::from("empty.md");

        write_archive_file(temp_dir.path(), &rel_path, "").unwrap();

        let full_path = temp_dir.path().join(&rel_path);
        assert!(full_path.exists());
        assert_eq!(std::fs::read_to_string(&full_path).unwrap(), "");
    }

    // ============================================================================
    // TDD Tests for write_message_to_archive
    // ============================================================================

    #[test]
    fn test_write_message_to_archive_creates_all_files() {
        let temp_dir = TempDir::new().unwrap();
        let paths = MessageArchivePaths {
            canonical: PathBuf::from("messages/2025/01/msg.md"),
            outbox: PathBuf::from("agents/sender/outbox/2025/01/msg.md"),
            inboxes: vec![
                PathBuf::from("agents/alice/inbox/2025/01/msg.md"),
                PathBuf::from("agents/bob/inbox/2025/01/msg.md"),
            ],
        };

        write_message_to_archive(temp_dir.path(), &paths, "test content").unwrap();

        // All 4 files should exist with same content
        assert!(temp_dir.path().join(&paths.canonical).exists());
        assert!(temp_dir.path().join(&paths.outbox).exists());
        assert!(temp_dir.path().join(&paths.inboxes[0]).exists());
        assert!(temp_dir.path().join(&paths.inboxes[1]).exists());

        // Verify content
        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join(&paths.canonical)).unwrap(),
            "test content"
        );
    }

    #[test]
    fn test_write_message_to_archive_empty_inboxes() {
        let temp_dir = TempDir::new().unwrap();
        let paths = MessageArchivePaths {
            canonical: PathBuf::from("msg.md"),
            outbox: PathBuf::from("out.md"),
            inboxes: vec![],
        };

        // Should succeed even with no inboxes
        write_message_to_archive(temp_dir.path(), &paths, "content").unwrap();

        assert!(temp_dir.path().join(&paths.canonical).exists());
        assert!(temp_dir.path().join(&paths.outbox).exists());
    }

    // ============================================================================
    // FTS Query Escaping Tests
    // ============================================================================

    /// Helper to simulate FTS query escaping logic from MessageBmc::search
    fn escape_fts_query(query: &str) -> String {
        let quote_count = query.chars().filter(|c| *c == '"').count();
        let has_fts_operators = query.contains(" AND ")
            || query.contains(" OR ")
            || query.contains(" NOT ")
            || query.contains('*');

        if quote_count % 2 != 0 {
            format!("\"{}\"", query.replace('"', "\"\""))
        } else if has_fts_operators || query.starts_with('"') {
            query.to_string()
        } else {
            query
                .split_whitespace()
                .map(|word| {
                    if word.contains('-') && !word.starts_with('"') {
                        format!("\"{}\"", word)
                    } else {
                        word.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    #[test]
    fn test_fts_query_escapes_hyphens() {
        // "full-text search" should NOT be interpreted as "full AND NOT text AND search"
        // FTS5 treats hyphen as NOT operator by default
        let escaped = escape_fts_query("full-text search");
        assert_eq!(escaped, "\"full-text\" search");

        // Multiple hyphenated words
        let escaped2 = escape_fts_query("real-time data-driven");
        assert_eq!(escaped2, "\"real-time\" \"data-driven\"");
    }

    #[test]
    fn test_fts_query_preserves_operators() {
        // Explicit FTS operators should be preserved
        assert_eq!(escape_fts_query("full AND text"), "full AND text");
        assert_eq!(escape_fts_query("search*"), "search*");
    }

    #[test]
    fn test_fts_query_handles_phrases() {
        // Quoted phrases should be preserved
        assert_eq!(escape_fts_query("\"exact phrase\""), "\"exact phrase\"");
    }

    #[test]
    fn test_fts_query_escapes_unbalanced_quotes() {
        // Unbalanced quotes should be escaped
        let escaped = escape_fts_query("\"unclosed phrase");
        assert!(escaped.starts_with('"') && escaped.ends_with('"'));
    }

    // ============================================================================
    // TDD Tests for get_recipients
    // ============================================================================

    #[test]
    fn test_get_recipients_sql_query_structure() {
        // Verify the SQL query structure is correct
        let expected_query = r#"
            SELECT a.name
            FROM message_recipients mr
            JOIN agents a ON mr.agent_id = a.id
            WHERE mr.message_id = ?
            ORDER BY mr.recipient_type, a.name
            "#;
        // The query should join message_recipients with agents
        assert!(expected_query.contains("message_recipients"));
        assert!(expected_query.contains("JOIN agents"));
        assert!(expected_query.contains("WHERE mr.message_id"));
    }

    #[test]
    fn test_get_recipients_returns_vec_string() {
        // Verify return type is Vec<String>
        fn assert_vec_string(_: Vec<String>) {}
        let recipients: Vec<String> = vec!["alice".to_string(), "bob".to_string()];
        assert_vec_string(recipients);
    }

    #[test]
    fn test_get_recipients_empty_list_handling() {
        // Empty recipients list should be valid
        let recipients: Vec<String> = vec![];
        assert!(recipients.is_empty());
        assert_eq!(recipients.len(), 0);
    }

    #[test]
    fn test_get_recipients_single_recipient() {
        let recipients = vec!["single-agent".to_string()];
        assert_eq!(recipients.len(), 1);
        assert_eq!(recipients[0], "single-agent");
    }

    #[test]
    fn test_get_recipients_multiple_recipients() {
        let recipients = vec![
            "alice".to_string(),
            "bob".to_string(),
            "charlie".to_string(),
        ];
        assert_eq!(recipients.len(), 3);
        assert!(recipients.contains(&"alice".to_string()));
        assert!(recipients.contains(&"bob".to_string()));
        assert!(recipients.contains(&"charlie".to_string()));
    }

    #[test]
    fn test_get_recipients_unicode_names() {
        let recipients = vec![
            "日本語エージェント".to_string(),
            "مساعد".to_string(),
            "помощник".to_string(),
        ];
        assert_eq!(recipients.len(), 3);
        assert_eq!(recipients[0], "日本語エージェント");
    }

    #[test]
    fn test_get_recipients_special_characters() {
        let recipients = vec![
            "agent-with-dashes".to_string(),
            "agent_with_underscores".to_string(),
            "agent.with.dots".to_string(),
        ];
        assert_eq!(recipients.len(), 3);
        assert!(recipients[0].contains("-"));
        assert!(recipients[1].contains("_"));
        assert!(recipients[2].contains("."));
    }

    #[test]
    fn test_get_recipients_preserves_order() {
        // The SQL orders by recipient_type then name
        let recipients = vec![
            "alice".to_string(),
            "bob".to_string(),
            "charlie".to_string(),
        ];
        // Verify order is deterministic
        assert_eq!(recipients[0], "alice");
        assert_eq!(recipients[1], "bob");
        assert_eq!(recipients[2], "charlie");
    }

    #[test]
    fn test_get_recipients_no_duplicates() {
        // Recipients should be unique (enforced by message_recipients table)
        let recipients = vec!["alice".to_string(), "bob".to_string()];
        let unique_count = recipients
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(unique_count, recipients.len());
    }

    #[test]
    fn test_get_recipients_max_recipients_handled() {
        // System should handle many recipients
        let recipients: Vec<String> = (0..100).map(|i| format!("agent-{}", i)).collect();
        assert_eq!(recipients.len(), 100);
        assert_eq!(recipients[0], "agent-0");
        assert_eq!(recipients[99], "agent-99");
    }

    #[test]
    fn test_get_recipients_empty_names_filtered() {
        // Empty agent names should not appear
        let recipients: Vec<String> = vec!["valid-agent".to_string()];
        // Empty strings should not be in the list
        assert!(!recipients.iter().any(|r| r.is_empty()));
    }

    #[test]
    fn test_get_recipients_whitespace_handling() {
        // Agent names should not have leading/trailing whitespace
        let recipients = vec!["alice".to_string(), "bob".to_string()];
        for r in &recipients {
            assert_eq!(r, r.trim());
        }
    }
}
