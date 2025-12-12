use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::Result;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub id: i64,
    pub agent_id: i64,
    pub capability: String,
    pub granted_at: NaiveDateTime,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AgentCapabilityForCreate {
    pub agent_id: i64,
    pub capability: String,
}

pub struct AgentCapabilityBmc;

impl AgentCapabilityBmc {
    pub async fn create(_ctx: &Ctx, mm: &ModelManager, capability_c: AgentCapabilityForCreate) -> Result<i64> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db.prepare(
            r#"
            INSERT INTO agent_capabilities (agent_id, capability, granted_at)
            VALUES (?, ?, ?)
            RETURNING id
            "#
        ).await?;

        let mut rows = stmt.query((
            capability_c.agent_id,
            capability_c.capability,
            now_str,
        )).await?;

        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Err(crate::Error::InvalidInput("Failed to create agent capability".into()))
        }
    }

    pub async fn list_for_agent(_ctx: &Ctx, mm: &ModelManager, agent_id: i64) -> Result<Vec<AgentCapability>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, agent_id, capability, granted_at
            FROM agent_capabilities WHERE agent_id = ? ORDER BY capability ASC
            "#
        ).await?;
        let mut rows = stmt.query([agent_id]).await?;

        let mut capabilities = Vec::new();
        while let Some(row) = rows.next().await? {
            let granted_at_str: String = row.get(3)?;
            let granted_at = NaiveDateTime::parse_from_str(&granted_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            capabilities.push(AgentCapability {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                capability: row.get(2)?,
                granted_at,
            });
        }
        Ok(capabilities)
    }

    pub async fn revoke(_ctx: &Ctx, mm: &ModelManager, id: i64) -> Result<()> {
        let db = mm.db();
        let stmt = db.prepare("DELETE FROM agent_capabilities WHERE id = ?").await?;
        stmt.execute([id]).await?;
        Ok(())
    }

    /// Check if an agent has a specific capability
    pub async fn check(_ctx: &Ctx, mm: &ModelManager, agent_id: i64, capability: &str) -> Result<bool> {
        let db = mm.db();
        let stmt = db.prepare(
            "SELECT COUNT(*) FROM agent_capabilities WHERE agent_id = ? AND capability = ?"
        ).await?;
        let mut rows = stmt.query((agent_id, capability)).await?;

        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }
}
