use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value; // Correct import for JSON Value

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: String,
    pub kind: String, // "message", "tool", "agent"
    pub project_id: i64,
    pub agent_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub metadata: Option<Value>,
    pub created_at: String, // ISO 8601 string for uniformity
}

pub struct ActivityBmc;

impl ActivityBmc {
    pub async fn list_recent(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        limit: i64,
    ) -> Result<Vec<ActivityItem>> {
        let db = mm.db();
        let mut items = Vec::new();

        // 1. Fetch recent Messages
        let limit_val: i64 = limit;
        let params: Vec<libsql::Value> = vec![project_id.into(), limit_val.into()];
        
        // We select fields manually to populate Message struct or just direct to fields we need
        // Let's use MessageBmc if available? No, MessageBmc doesn't expose list by project easily with limit sorted global?
        // Actually MessageBmc might not exist or we want raw. 
        // Let's assume we can query raw.
        // Needs to match crates/libs/lib-core/src/model/message.rs struct or manual map
        
        let sql_msg = r#"
            SELECT id, project_id, sender_id, subject, body_md, created_ts 
            FROM messages 
            WHERE project_id = ? 
            ORDER BY created_ts DESC 
            LIMIT ?
        "#;
        let stmt = db.prepare(sql_msg).await?;
        let mut rows = stmt.query(params).await?;
        
        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let pid: i64 = row.get(1)?;
            let sid: i64 = row.get(2)?;
            let subject: String = row.get(3)?;
            let body: String = row.get(4)?;
            // created_ts could be string or int depending on schema. 
            // In 001_initial_schema.sql: created_ts DATETIME DEFAULT CURRENT_TIMESTAMP (string)
            // But verify if struct maps it to NaiveDateTime.
            // Row.get can cast.
            let created_ts: String = row.get(5)?; 

            items.push(ActivityItem {
                id: format!("msg:{}", id),
                kind: "message".into(),
                project_id: pid,
                agent_id: Some(sid),
                title: subject,
                description: Some(body.chars().take(100).collect()),
                metadata: None,
                created_at: created_ts,
            });
        }

        // 2. Fetch recent Tool Metrics
        let tools = crate::model::tool_metric::ToolMetricBmc::list_recent(_ctx, mm, Some(project_id), limit).await?;
        for t in tools {
             items.push(ActivityItem {
                id: format!("tool:{}", t.id),
                kind: "tool".into(),
                project_id,
                agent_id: t.agent_id,
                title: format!("Tool Used: {}", t.tool_name),
                description: Some(format!("Status: {}, Duration: {}ms", t.status, t.duration_ms)),
                metadata: t.args_json.and_then(|s| serde_json::from_str(&s).ok()),
                created_at: t.created_at,
            });
        }

        // 3. Fetch recent Agents
        let params: Vec<libsql::Value> = vec![project_id.into(), limit_val.into()];
        let sql_agent = r#"
            SELECT id, project_id, name, task_description, inception_ts
            FROM agents
            WHERE project_id = ?
            ORDER BY inception_ts DESC
            LIMIT ?
        "#;
        let stmt = db.prepare(sql_agent).await?;
        let mut rows = stmt.query(params).await?;

        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let pid: i64 = row.get(1)?;
            let name: String = row.get(2)?;
            let desc: String = row.get(3)?;
            let inception_ts: String = row.get(4)?;

            items.push(ActivityItem {
                id: format!("agent:{}", id),
                kind: "agent".into(),
                project_id: pid,
                agent_id: Some(id), // Agent is its own agent? Or None? Activity is "Agent Created".
                title: format!("Agent Created: {}", name),
                description: Some(desc),
                metadata: None,
                created_at: inception_ts,
            });
        }

        // 4. Sort and Limit
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items.truncate(limit as usize);

        Ok(items)
    }
}
