use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::Result;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroDef {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub description: String,
    pub steps: Vec<Value>,
    pub created_ts: NaiveDateTime,
    pub updated_ts: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroDefForCreate {
    pub project_id: i64,
    pub name: String,
    pub description: String,
    pub steps: Vec<Value>,
}

pub struct MacroDefBmc;

impl MacroDefBmc {
    pub async fn create(_ctx: &Ctx, mm: &ModelManager, macro_c: MacroDefForCreate) -> Result<i64> {
        let db = mm.db();
        let steps_json = serde_json::to_string(&macro_c.steps)?;

        let stmt = db.prepare(
            r#"
            INSERT INTO macros (project_id, name, description, steps)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#
        ).await?;

        let mut rows = stmt.query((
            macro_c.project_id,
            macro_c.name.as_str(),
            macro_c.description.as_str(),
            steps_json.as_str(),
        )).await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput("Failed to create macro".into()));
        };

        Ok(id)
    }

    pub async fn get_by_name(_ctx: &Ctx, mm: &ModelManager, project_id: i64, name: &str) -> Result<MacroDef> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, project_id, name, description, steps, created_ts, updated_ts
            FROM macros
            WHERE project_id = ? AND name = ?
            "#
        ).await?;

        let mut rows = stmt.query((project_id, name)).await?;

        if let Some(row) = rows.next().await? {
            Ok(Self::from_row(row)?)
        } else {
            Err(crate::Error::InvalidInput(format!("Macro not found: {}", name)))
        }
    }

    pub async fn list(_ctx: &Ctx, mm: &ModelManager, project_id: i64) -> Result<Vec<MacroDef>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, project_id, name, description, steps, created_ts, updated_ts
            FROM macros
            WHERE project_id = ?
            ORDER BY name ASC
            "#
        ).await?;

        let mut rows = stmt.query([project_id]).await?;
        let mut macros = Vec::new();

        while let Some(row) = rows.next().await? {
            macros.push(Self::from_row(row)?);
        }
        Ok(macros)
    }

    pub async fn delete(_ctx: &Ctx, mm: &ModelManager, project_id: i64, name: &str) -> Result<bool> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            DELETE FROM macros WHERE project_id = ? AND name = ?
            "#
        ).await?;
        let affected = stmt.execute((project_id, name)).await?;
        Ok(affected > 0)
    }

    fn from_row(row: libsql::Row) -> Result<MacroDef> {
        let created_ts_str: String = row.get(5).unwrap_or_default();
        let updated_ts_str: String = row.get(6).unwrap_or_default();
        let steps_str: String = row.get(4).unwrap_or_else(|_| "[]".to_string());

        let created_ts = NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_default();
        let updated_ts = NaiveDateTime::parse_from_str(&updated_ts_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_default();
        let steps: Vec<Value> = serde_json::from_str(&steps_str).unwrap_or_default();

        Ok(MacroDef {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            steps,
            created_ts,
            updated_ts,
        })
    }
}
