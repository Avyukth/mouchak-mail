use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::Result;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSlot {
    pub id: i64,
    pub project_id: i64,
    pub agent_id: i64,
    pub slot_name: String,
    pub created_ts: NaiveDateTime,
    pub expires_ts: NaiveDateTime,
    pub released_ts: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSlotForCreate {
    pub project_id: i64,
    pub agent_id: i64,
    pub slot_name: String,
    pub ttl_seconds: i64,
}

pub struct BuildSlotBmc;

impl BuildSlotBmc {
    pub async fn acquire(_ctx: &Ctx, mm: &ModelManager, slot_c: BuildSlotForCreate) -> Result<i64> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let expires = now + chrono::Duration::seconds(slot_c.ttl_seconds);
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let expires_str = expires.format("%Y-%m-%d %H:%M:%S").to_string();

        // Check if slot is already held
        let stmt = db.prepare(
            r#"
            SELECT id FROM build_slots
            WHERE project_id = ? AND slot_name = ? AND released_ts IS NULL AND expires_ts > ?
            "#
        ).await?;
        let mut rows = stmt.query((slot_c.project_id, slot_c.slot_name.as_str(), now_str.as_str())).await?;

        if rows.next().await?.is_some() {
            return Err(crate::Error::InvalidInput("Build slot already held".into()));
        }

        let stmt = db.prepare(
            r#"
            INSERT INTO build_slots (project_id, agent_id, slot_name, expires_ts)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#
        ).await?;

        let mut rows = stmt.query((
            slot_c.project_id,
            slot_c.agent_id,
            slot_c.slot_name.as_str(),
            expires_str.as_str(),
        )).await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput("Failed to acquire build slot".into()));
        };

        Ok(id)
    }

    pub async fn renew(_ctx: &Ctx, mm: &ModelManager, slot_id: i64, ttl_seconds: i64) -> Result<NaiveDateTime> {
        let db = mm.db();
        let new_expires = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl_seconds);
        let expires_str = new_expires.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db.prepare(
            r#"
            UPDATE build_slots SET expires_ts = ? WHERE id = ? AND released_ts IS NULL
            "#
        ).await?;
        stmt.execute((expires_str.as_str(), slot_id)).await?;
        Ok(new_expires)
    }

    pub async fn release(_ctx: &Ctx, mm: &ModelManager, slot_id: i64) -> Result<()> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db.prepare(
            r#"
            UPDATE build_slots SET released_ts = ? WHERE id = ? AND released_ts IS NULL
            "#
        ).await?;
        stmt.execute((now_str.as_str(), slot_id)).await?;
        Ok(())
    }

    pub async fn list_active(_ctx: &Ctx, mm: &ModelManager, project_id: i64) -> Result<Vec<BuildSlot>> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db.prepare(
            r#"
            SELECT id, project_id, agent_id, slot_name, created_ts, expires_ts, released_ts
            FROM build_slots
            WHERE project_id = ? AND released_ts IS NULL AND expires_ts > ?
            ORDER BY created_ts DESC
            "#
        ).await?;

        let mut rows = stmt.query((project_id, now_str.as_str())).await?;
        let mut slots = Vec::new();

        while let Some(row) = rows.next().await? {
            slots.push(Self::from_row(row)?);
        }
        Ok(slots)
    }

    fn from_row(row: libsql::Row) -> Result<BuildSlot> {
        let created_ts_str: String = row.get(4).unwrap_or_default();
        let expires_ts_str: String = row.get(5).unwrap_or_default();
        let released_ts_str: Option<String> = row.get(6).unwrap_or_default();

        let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_default();
        let expires_ts = NaiveDateTime::parse_from_str(&expires_ts_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_default();
        let released_ts = released_ts_str.and_then(|s|
            NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()
        );

        Ok(BuildSlot {
            id: row.get(0)?,
            project_id: row.get(1)?,
            agent_id: row.get(2)?,
            slot_name: row.get(3)?,
            created_ts,
            expires_ts,
            released_ts,
        })
    }
}
