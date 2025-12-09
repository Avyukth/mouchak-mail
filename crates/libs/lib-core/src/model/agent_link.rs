use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::Result;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLink {
    pub id: i64,
    pub a_project_id: i64,
    pub a_agent_id: i64,
    pub b_project_id: i64,
    pub b_agent_id: i64,
    pub status: String,
    pub reason: String,
    pub created_ts: NaiveDateTime,
    pub updated_ts: NaiveDateTime,
    pub expires_ts: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLinkForCreate {
    pub a_project_id: i64,
    pub a_agent_id: i64,
    pub b_project_id: i64,
    pub b_agent_id: i64,
    pub reason: String,
}

pub struct AgentLinkBmc;

impl AgentLinkBmc {
    /// Request contact from agent A to agent B
    pub async fn request_contact(_ctx: &Ctx, mm: &ModelManager, link_c: AgentLinkForCreate) -> Result<i64> {
        let db = mm.db();

        let stmt = db.prepare(
            r#"
            INSERT INTO agent_links (a_project_id, a_agent_id, b_project_id, b_agent_id, status, reason)
            VALUES (?, ?, ?, ?, 'pending', ?)
            RETURNING id
            "#
        ).await?;

        let mut rows = stmt.query((
            link_c.a_project_id,
            link_c.a_agent_id,
            link_c.b_project_id,
            link_c.b_agent_id,
            link_c.reason.as_str(),
        )).await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput("Failed to create contact request".into()));
        };

        Ok(id)
    }

    /// Respond to a contact request (accept or reject)
    pub async fn respond_contact(_ctx: &Ctx, mm: &ModelManager, link_id: i64, accept: bool) -> Result<()> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let status = if accept { "accepted" } else { "rejected" };

        let stmt = db.prepare(
            r#"
            UPDATE agent_links SET status = ?, updated_ts = ? WHERE id = ?
            "#
        ).await?;
        stmt.execute((status, now_str, link_id)).await?;
        Ok(())
    }

    /// List contacts for an agent (all accepted links where agent is either party)
    pub async fn list_contacts(_ctx: &Ctx, mm: &ModelManager, project_id: i64, agent_id: i64) -> Result<Vec<AgentLink>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, a_project_id, a_agent_id, b_project_id, b_agent_id, status, reason, created_ts, updated_ts, expires_ts
            FROM agent_links
            WHERE status = 'accepted' AND (
                (a_project_id = ? AND a_agent_id = ?) OR
                (b_project_id = ? AND b_agent_id = ?)
            )
            ORDER BY updated_ts DESC
            "#
        ).await?;

        let mut rows = stmt.query((project_id, agent_id, project_id, agent_id)).await?;
        let mut links = Vec::new();

        while let Some(row) = rows.next().await? {
            links.push(Self::from_row(row)?);
        }
        Ok(links)
    }

    /// List pending contact requests for an agent (where agent is B)
    pub async fn list_pending_requests(_ctx: &Ctx, mm: &ModelManager, project_id: i64, agent_id: i64) -> Result<Vec<AgentLink>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, a_project_id, a_agent_id, b_project_id, b_agent_id, status, reason, created_ts, updated_ts, expires_ts
            FROM agent_links
            WHERE status = 'pending' AND b_project_id = ? AND b_agent_id = ?
            ORDER BY created_ts DESC
            "#
        ).await?;

        let mut rows = stmt.query((project_id, agent_id)).await?;
        let mut links = Vec::new();

        while let Some(row) = rows.next().await? {
            links.push(Self::from_row(row)?);
        }
        Ok(links)
    }

    fn from_row(row: libsql::Row) -> Result<AgentLink> {
        let created_ts_str: String = row.get(7).unwrap_or_default();
        let updated_ts_str: String = row.get(8).unwrap_or_default();
        let expires_ts_str: Option<String> = row.get(9).unwrap_or_default();

        let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_default();
        let updated_ts = NaiveDateTime::parse_from_str(&updated_ts_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_default();
        let expires_ts = expires_ts_str.and_then(|s|
            NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()
        );

        Ok(AgentLink {
            id: row.get(0)?,
            a_project_id: row.get(1)?,
            a_agent_id: row.get(2)?,
            b_project_id: row.get(3)?,
            b_agent_id: row.get(4)?,
            status: row.get(5)?,
            reason: row.get(6)?,
            created_ts,
            updated_ts,
            expires_ts,
        })
    }
}
