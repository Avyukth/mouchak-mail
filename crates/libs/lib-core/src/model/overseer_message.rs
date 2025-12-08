use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::Result;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverseerMessageForCreate {
    pub project_id: i64,
    pub sender_id: i64,
    pub subject: String,
    pub body_md: String,
    pub importance: String,
}

pub struct OverseerMessageBmc;

impl OverseerMessageBmc {
    pub async fn create(_ctx: &Ctx, mm: &ModelManager, msg_c: OverseerMessageForCreate) -> Result<i64> {
        let db = mm.db();

        let stmt = db.prepare(
            r#"
            INSERT INTO overseer_messages (project_id, sender_id, subject, body_md, importance)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#
        ).await?;

        let mut rows = stmt.query((
            msg_c.project_id,
            msg_c.sender_id,
            msg_c.subject.as_str(),
            msg_c.body_md.as_str(),
            msg_c.importance.as_str(),
        )).await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput("Failed to create overseer message".into()));
        };

        Ok(id)
    }

    pub async fn list_unread(_ctx: &Ctx, mm: &ModelManager, project_id: i64) -> Result<Vec<OverseerMessage>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, project_id, sender_id, subject, body_md, importance, created_ts, read_ts
            FROM overseer_messages
            WHERE project_id = ? AND read_ts IS NULL
            ORDER BY created_ts DESC
            "#
        ).await?;

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

        let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_default();
        let read_ts = read_ts_str.and_then(|s|
            NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()
        );

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
