use crate::Result;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A defined workflow macro (sequence of tools).
///
/// Macros allow agents to execute common patterns (like "check inbox" or
/// "create feature") in a single step.
///
/// # Fields
///
/// - `id` - Database primary key
/// - `project_id` - Project context
/// - `name` - Unique macro name within project
/// - `description` - Human readable description
/// - `steps` - JSON array of tool calls or actions
/// - `created_ts` - Creation timestamp
/// - `updated_ts` - Last update timestamp
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

/// Input data for creating a new macro.
///
/// # Fields
///
/// - `project_id` - Project to attach macro to
/// - `name` - Macro name (must be unique in project)
/// - `description` - What the macro does
/// - `steps` - Worklow steps (tool calls)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroDefForCreate {
    pub project_id: i64,
    pub name: String,
    pub description: String,
    pub steps: Vec<Value>,
}

/// Backend Model Controller for Macro Definition operations.
///
/// Manages workflow macros (automated sequences of MCP tool calls).
/// Includes 5 built-in macros that are auto-registered for each project.
pub struct MacroDefBmc;

impl MacroDefBmc {
    /// Creates a new macro definition.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `macro_c` - Macro creation data (name, description, steps)
    ///
    /// # Returns
    /// The created macro's database ID
    ///
    /// # Errors
    /// Returns an error if macro name already exists in project
    ///
    /// # Example
    /// ```no_run
    /// # use mouchak_mail_core::model::macro_def::*;
    /// # use mouchak_mail_core::model::ModelManager;
    /// # use mouchak_mail_core::ctx::Ctx;
    /// # async fn example(mm: &ModelManager) {
    /// let ctx = Ctx::root_ctx();
    /// let macro_def = MacroDefForCreate {
    ///     project_id: 1,
    ///     name: "deploy".to_string(),
    ///     description: "Deploy to production".to_string(),
    ///     steps: vec![],
    /// };
    /// let id = MacroDefBmc::create(&ctx, mm, macro_def).await.unwrap();
    /// # }
    /// ```
    pub async fn create(_ctx: &Ctx, mm: &ModelManager, macro_c: MacroDefForCreate) -> Result<i64> {
        let db = mm.db();
        let steps_json = serde_json::to_string(&macro_c.steps)?;

        let stmt = db
            .prepare(
                r#"
            INSERT INTO macros (project_id, name, description, steps)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
            )
            .await?;

        let mut rows = stmt
            .query((
                macro_c.project_id,
                macro_c.name.as_str(),
                macro_c.description.as_str(),
                steps_json.as_str(),
            ))
            .await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput("Failed to create macro".into()));
        };

        Ok(id)
    }

    pub async fn get_by_name(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        name: &str,
    ) -> Result<MacroDef> {
        let db = mm.db();
        let stmt = db
            .prepare(
                r#"
            SELECT id, project_id, name, description, steps, created_ts, updated_ts
            FROM macros
            WHERE project_id = ? AND name = ?
            "#,
            )
            .await?;

        let mut rows = stmt.query((project_id, name)).await?;

        if let Some(row) = rows.next().await? {
            Ok(Self::from_row(row)?)
        } else {
            Err(crate::Error::InvalidInput(format!(
                "Macro not found: {}",
                name
            )))
        }
    }

    /// Lists all macros for a project (built-in + custom).
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `project_id` - Project database ID
    ///
    /// # Returns
    /// Vector of all macro definitions (may be empty)
    pub async fn list(_ctx: &Ctx, mm: &ModelManager, project_id: i64) -> Result<Vec<MacroDef>> {
        let db = mm.db();
        let stmt = db
            .prepare(
                r#"
            SELECT id, project_id, name, description, steps, created_ts, updated_ts
            FROM macros
            WHERE project_id = ?
            ORDER BY name ASC
            "#,
            )
            .await?;

        let mut rows = stmt.query([project_id]).await?;
        let mut macros = Vec::new();

        while let Some(row) = rows.next().await? {
            macros.push(Self::from_row(row)?);
        }
        Ok(macros)
    }

    pub async fn delete(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        name: &str,
    ) -> Result<bool> {
        let db = mm.db();
        let stmt = db
            .prepare(
                r#"
            DELETE FROM macros WHERE project_id = ? AND name = ?
            "#,
            )
            .await?;
        let affected = stmt.execute((project_id, name)).await?;
        Ok(affected > 0)
    }

    fn from_row(row: libsql::Row) -> Result<MacroDef> {
        let created_ts_str: String = row.get(5).unwrap_or_default();
        let updated_ts_str: String = row.get(6).unwrap_or_default();
        let steps_str: String = row.get(4).unwrap_or_else(|_| "[]".to_string());

        let created_ts =
            NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S").unwrap_or_default();
        let updated_ts =
            NaiveDateTime::parse_from_str(&updated_ts_str, "%Y-%m-%d %H:%M:%S").unwrap_or_default();
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

    /// Ensure all built-in macros exist for a project
    pub async fn ensure_builtin_macros(
        ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
    ) -> Result<Vec<String>> {
        let mut created = Vec::new();

        for (name, description, steps) in get_builtin_macros() {
            // Check if macro already exists
            if Self::get_by_name(ctx, mm, project_id, &name).await.is_ok() {
                continue;
            }

            // Create the macro
            let macro_c = MacroDefForCreate {
                project_id,
                name: name.clone(),
                description,
                steps,
            };

            Self::create(ctx, mm, macro_c).await?;
            created.push(name);
        }

        Ok(created)
    }
}

/// Get the 5 built-in macro definitions
fn get_builtin_macros() -> Vec<(String, String, Vec<Value>)> {
    vec![
        // 1. start_session: Register agent and check inbox
        (
            "start_session".to_string(),
            "Register an agent in the project and immediately check their inbox for pending messages.".to_string(),
            vec![
                serde_json::json!({
                    "tool": "register_agent",
                    "description": "Register the agent in the project",
                    "params": ["project_slug", "name", "program", "model", "task_description"]
                }),
                serde_json::json!({
                    "tool": "check_inbox",
                    "description": "Check for any pending messages",
                    "params": ["project_slug", "agent_name", "limit"]
                }),
            ],
        ),
        // 2. prepare_thread: Create a thread and reserve files
        (
            "prepare_thread".to_string(),
            "Send an initial message to create a thread and reserve files for the work.".to_string(),
            vec![
                serde_json::json!({
                    "tool": "send_message",
                    "description": "Send initial message to create thread",
                    "params": ["project_slug", "sender_name", "recipient_names", "subject", "body_md"]
                }),
                serde_json::json!({
                    "tool": "reserve_file",
                    "description": "Reserve files for exclusive editing",
                    "params": ["project_slug", "agent_name", "path_pattern", "reason", "ttl_minutes"]
                }),
            ],
        ),
        // 3. file_reservation_cycle: Reserve, work, release
        (
            "file_reservation_cycle".to_string(),
            "Complete file reservation workflow: reserve files, do work, then release them.".to_string(),
            vec![
                serde_json::json!({
                    "tool": "reserve_file",
                    "description": "Reserve files for exclusive editing",
                    "params": ["project_slug", "agent_name", "path_pattern", "reason", "ttl_minutes"]
                }),
                serde_json::json!({
                    "action": "user_work",
                    "description": "Perform the actual file editing work"
                }),
                serde_json::json!({
                    "tool": "release_reservation",
                    "description": "Release the file reservation when done",
                    "params": ["reservation_id"]
                }),
            ],
        ),
        // 4. contact_handshake: Cross-project contact setup
        (
            "contact_handshake".to_string(),
            "Establish a bidirectional contact between two agents across projects.".to_string(),
            vec![
                serde_json::json!({
                    "tool": "request_contact",
                    "description": "Request contact from agent A to agent B",
                    "params": ["from_project_slug", "from_agent_name", "to_project_slug", "to_agent_name", "reason"]
                }),
                serde_json::json!({
                    "tool": "respond_contact",
                    "description": "Accept the contact request (by agent B)",
                    "params": ["link_id", "accept"]
                }),
            ],
        ),
        // 5. broadcast_message: Send to multiple agents
        (
            "broadcast_message".to_string(),
            "Send a message to multiple agents in the project at once.".to_string(),
            vec![
                serde_json::json!({
                    "tool": "list_agents",
                    "description": "List all agents to identify broadcast targets",
                    "params": ["project_slug"]
                }),
                serde_json::json!({
                    "tool": "send_message",
                    "description": "Send message to all target agents",
                    "params": ["project_slug", "sender_name", "recipient_names", "subject", "body_md", "importance"]
                }),
            ],
        ),
    ]
}
