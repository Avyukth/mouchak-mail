use crate::Result;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// A high-priority message for the human overseer.
///
/// These messages circumvent the normal inbox and are highlighted
/// to ensure immediate attention (e.g., for critical errors or approval).
///
/// # Fields
///
/// - `id` - Database primary key
/// - `project_id` - Project context
/// - `sender_id` - Reporting agent
/// - `subject` - Urgent subject line
/// - `body_md` - Detailed explanation in Markdown
/// - `importance` - "critical" or "high"
/// - `created_ts` - When it was sent
/// - `read_ts` - When the overseer opened it
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverseerMessage {
    pub id: i64,
    pub project_id: i64,
    pub sender_id: i64,
    pub subject: String,
    pub body_md: String,
    pub importance: String,
    pub created_ts: NaiveDateTime,
    pub read_ts: Option<NaiveDateTime>,
}

/// Input to create an overseer message.
///
/// # Fields
///
/// - `project_id` - Project context
/// - `sender_id` - Reporting agent
/// - `subject` - Concise summary
/// - `body_md` - Full report
/// - `importance` - "critical" or "high"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverseerMessageForCreate {
    pub project_id: i64,
    pub sender_id: i64,
    pub subject: String,
    pub body_md: String,
    pub importance: String,
}

/// Backend Model Controller for Overseer Message operations.
///
/// Manages high-priority messages sent to the human overseer (project owner).
/// These messages require human attention and are tracked separately from
/// regular inter-agent communication.
pub struct OverseerMessageBmc;

impl OverseerMessageBmc {
    /// Creates a new overseer message.
    ///
    /// Use this when an agent needs human attention (e.g., error escalation,
    /// approval requests, or critical notifications).
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `msg_c` - Message content with importance level
    ///
    /// # Returns
    /// The created message's database ID
    pub async fn create(
        _ctx: &Ctx,
        mm: &ModelManager,
        msg_c: OverseerMessageForCreate,
    ) -> Result<i64> {
        let db = mm.db();

        let stmt = db
            .prepare(
                r#"
            INSERT INTO overseer_messages (project_id, sender_id, subject, body_md, importance)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
            )
            .await?;

        let mut rows = stmt
            .query((
                msg_c.project_id,
                msg_c.sender_id,
                msg_c.subject.as_str(),
                msg_c.body_md.as_str(),
                msg_c.importance.as_str(),
            ))
            .await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput(
                "Failed to create overseer message".into(),
            ));
        };

        Ok(id)
    }

    /// Lists all unread overseer messages for a project.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `project_id` - Project database ID
    ///
    /// # Returns
    /// Vector of unread messages (newest first)
    pub async fn list_unread(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
    ) -> Result<Vec<OverseerMessage>> {
        let db = mm.db();
        let stmt = db
            .prepare(
                r#"
            SELECT id, project_id, sender_id, subject, body_md, importance, created_ts, read_ts
            FROM overseer_messages
            WHERE project_id = ? AND read_ts IS NULL
            ORDER BY created_ts DESC
            "#,
            )
            .await?;

        let mut rows = stmt.query([project_id]).await?;
        let mut messages = Vec::new();

        while let Some(row) = rows.next().await? {
            messages.push(Self::from_row(row)?);
        }
        Ok(messages)
    }

    fn from_row(row: libsql::Row) -> Result<OverseerMessage> {
        let created_ts_str: String = row.get(6).unwrap_or_default();
        let read_ts_str: Option<String> = row.get(7).unwrap_or_default();

        let created_ts =
            NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S").unwrap_or_default();
        let read_ts =
            read_ts_str.and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok());

        Ok(OverseerMessage {
            id: row.get(0)?,
            project_id: row.get(1)?,
            sender_id: row.get(2)?,
            subject: row.get(3)?,
            body_md: row.get(4)?,
            importance: row.get(5)?,
            created_ts,
            read_ts,
        })
    }
}
