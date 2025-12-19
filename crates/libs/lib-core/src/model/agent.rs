//! Agent management for AI coding agents.
//!
//! This module provides the BMC (Backend Model Controller) for managing AI agents
//! within projects. Each agent represents an autonomous AI coding assistant.
//!
//! # Data Model
//!
//! - **Agent**: The main entity representing a registered AI agent
//! - **AgentForCreate**: Input data for agent registration
//! - **AgentProfileUpdate**: Partial update for agent profile fields
//!
//! # Example
//!
//! ```no_run
//! use lib_core::model::agent::{AgentBmc, AgentForCreate};
//! use lib_core::model::ModelManager;
//! use lib_core::ctx::Ctx;
//!
//! # async fn example() -> lib_core::Result<()> {
//! let mm = ModelManager::new().await?;
//! let ctx = Ctx::root_ctx();
//!
//! // Register a new agent
//! let agent_data = AgentForCreate {
//!     project_id: 1,
//!     name: "worker-1".to_string(),
//!     program: "claude-code".to_string(),
//!     model: "claude-sonnet-4-20250514".to_string(),
//!     task_description: "Implement user auth".to_string(),
//! };
//! let agent_id = AgentBmc::create(&ctx, &mm, agent_data).await?;
//!
//! // Retrieve the agent
//! let agent = AgentBmc::get(&ctx, &mm, agent_id).await?;
//! println!("Agent {} registered", agent.name);
//! # Ok(())
//! # }
//! ```

use crate::Result;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::store::git_store;
use crate::utils::mistake_detection::suggest_similar;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A registered AI coding agent.
///
/// Agents are AI assistants that work on tasks within a project. Each agent
/// has a unique name within its project and maintains profile information
/// including capabilities and policies.
///
/// # Fields
///
/// - `id` - Database primary key
/// - `project_id` - Foreign key to the parent project
/// - `name` - Unique agent name within the project (e.g., "worker-1")
/// - `program` - Agent runtime (e.g., "claude-code", "cursor", "aider")
/// - `model` - AI model identifier (e.g., "claude-sonnet-4-20250514")
/// - `task_description` - Current task being worked on
/// - `inception_ts` - When the agent was first registered
/// - `last_active_ts` - Last activity timestamp
/// - `attachments_policy` - How agent handles file attachments
/// - `contact_policy` - Agent communication preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub program: String,
    pub model: String,
    pub task_description: String,
    pub inception_ts: NaiveDateTime,
    pub last_active_ts: NaiveDateTime,
    pub attachments_policy: String,
    pub contact_policy: String,
}

/// Input data for creating a new agent.
///
/// All fields are required for agent registration.
///
/// # Fields
///
/// - `project_id` - ID of the project to register the agent in
/// - `name` - Unique name for the agent within the project
/// - `program` - Runtime environment (e.g., "claude-code")
/// - `model` - AI model to use (e.g., "claude-sonnet-4-20250514")
/// - `task_description` - Initial task description
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AgentForCreate {
    /// Project database ID.
    pub project_id: i64,
    /// Unique agent name within the project.
    pub name: String,
    /// Agent runtime program.
    pub program: String,
    /// AI model identifier.
    pub model: String,
    /// Description of the agent's current task.
    pub task_description: String,
}

/// Backend Model Controller for Agent operations.
///
/// Provides stateless methods for agent lifecycle management including
/// registration, retrieval, and profile updates.
pub struct AgentBmc;

impl AgentBmc {
    /// Creates a new agent in the specified project.
    ///
    /// This method:
    /// 1. Inserts the agent into the database
    /// 2. Creates a profile.json file in the Git archive
    ///
    /// # Arguments
    /// * `_ctx` - Request context (unused currently)
    /// * `mm` - ModelManager providing database and Git access
    /// * `agent_c` - Agent creation data
    ///
    /// # Returns
    /// The created agent's database ID
    ///
    /// # Errors
    /// Returns an error if:
    /// - Agent name already exists in the project
    /// - Project ID is invalid
    /// - Git operations fail
    ///
    /// # Example
    /// ```no_run
    /// # use lib_core::model::agent::{AgentBmc, AgentForCreate};
    /// # use lib_core::model::ModelManager;
    /// # use lib_core::ctx::Ctx;
    /// # async fn example(mm: &ModelManager) {
    /// let ctx = Ctx::root_ctx();
    /// let agent = AgentForCreate {
    ///     project_id: 1,
    ///     name: "claude-1".to_string(),
    ///     program: "claude-code".to_string(),
    ///     model: "claude-3.5-sonnet".to_string(),
    ///     task_description: "Implement feature X".to_string(),
    /// };
    /// let id = AgentBmc::create(&ctx, mm, agent).await.unwrap();
    /// # }
    /// ```
    pub async fn create(_ctx: &Ctx, mm: &ModelManager, agent_c: AgentForCreate) -> Result<i64> {
        let db = mm.db();

        // 1. Insert into DB
        let stmt = db
            .prepare(
                r#"
            INSERT INTO agents (project_id, name, program, model, task_description)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
            )
            .await?;

        let mut rows = stmt
            .query((
                agent_c.project_id,
                agent_c.name.as_str(),
                agent_c.program.as_str(),
                agent_c.model.as_str(),
                agent_c.task_description.as_str(),
            ))
            .await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput("Failed to create agent".into()));
        };

        // 2. Write profile to Git
        let stmt = db.prepare("SELECT slug FROM projects WHERE id = ?").await?;
        let mut rows = stmt.query([agent_c.project_id]).await?;

        let project_slug: String = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(crate::Error::project_not_found(format!(
                "ID: {}",
                agent_c.project_id
            )));
        };

        // Git Operations - serialized to prevent lock contention
        let _git_guard = mm.git_lock.lock().await;

        // Use cached repository to prevent FD exhaustion
        let repo_arc = mm.get_repo().await?;
        let repo = repo_arc.lock().await;

        let agent_dir = PathBuf::from("projects")
            .join(&project_slug)
            .join("agents")
            .join(&agent_c.name);

        // File path relative to repo root
        let profile_rel_path = agent_dir.join("profile.json");
        let profile_json = serde_json::to_string_pretty(&agent_c)?;

        git_store::commit_file(
            &repo,
            &profile_rel_path,
            &profile_json,
            &format!("agent: profile {}", agent_c.name),
            "mcp-bot",
            "mcp-bot@localhost",
        )?;

        Ok(id)
    }

    /// Retrieves an agent by its database ID.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager providing database access
    /// * `id` - Agent database ID
    ///
    /// # Returns
    /// The agent data including all fields
    ///
    /// # Errors
    /// Returns `Error::AgentNotFound` if the ID doesn't exist
    pub async fn get(_ctx: &Ctx, mm: &ModelManager, id: i64) -> Result<Agent> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, project_id, name, program, model, task_description, inception_ts, last_active_ts, attachments_policy, contact_policy
            FROM agents WHERE id = ?
            "#
        ).await?;
        let mut rows = stmt.query([id]).await?;

        if let Some(row) = rows.next().await? {
            // Column indices: 0=id, 1=project_id, 2=name, 3=program, 4=model,
            //                 5=task_description, 6=inception_ts, 7=last_active_ts,
            //                 8=attachments_policy, 9=contact_policy
            let inception_ts_str: String = row.get(6)?;
            let inception_ts =
                NaiveDateTime::parse_from_str(&inception_ts_str, "%Y-%m-%d %H:%M:%S")
                    .unwrap_or_default();
            let last_active_ts_str: String = row.get(7)?;
            let last_active_ts =
                NaiveDateTime::parse_from_str(&last_active_ts_str, "%Y-%m-%d %H:%M:%S")
                    .unwrap_or_default();

            Ok(Agent {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                program: row.get(3)?,
                model: row.get(4)?,
                task_description: row.get(5)?,
                inception_ts,
                last_active_ts,
                attachments_policy: row.get(8)?,
                contact_policy: row.get(9)?,
            })
        } else {
            Err(crate::Error::agent_not_found(format!("ID: {}", id)))
        }
    }

    /// Retrieves an agent by name within a project.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager providing database access
    /// * `project_id` - Project database ID
    /// * `name` - Agent name (unique within project)
    ///
    /// # Returns
    /// The agent data
    ///
    /// # Errors
    /// Returns `Error::AgentNotFound` if no agent with that name exists in the project
    pub async fn get_by_name(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        name: &str,
    ) -> Result<Agent> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, project_id, name, program, model, task_description, inception_ts, last_active_ts, attachments_policy, contact_policy
            FROM agents WHERE project_id = ? AND name = ?
            "#
        ).await?;
        let mut rows = stmt.query((project_id, name)).await?;

        if let Some(row) = rows.next().await? {
            // Column indices: 0=id, 1=project_id, 2=name, 3=program, 4=model,
            //                 5=task_description, 6=inception_ts, 7=last_active_ts,
            //                 8=attachments_policy, 9=contact_policy
            let inception_ts_str: String = row.get(6)?;
            let inception_ts =
                NaiveDateTime::parse_from_str(&inception_ts_str, "%Y-%m-%d %H:%M:%S")
                    .unwrap_or_default();
            let last_active_ts_str: String = row.get(7)?;
            let last_active_ts =
                NaiveDateTime::parse_from_str(&last_active_ts_str, "%Y-%m-%d %H:%M:%S")
                    .unwrap_or_default();

            Ok(Agent {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                program: row.get(3)?,
                model: row.get(4)?,
                task_description: row.get(5)?,
                inception_ts,
                last_active_ts,
                attachments_policy: row.get(8)?,
                contact_policy: row.get(9)?,
            })
        } else {
            // Fetch all agent names in this project for suggestions
            let stmt = db
                .prepare("SELECT name FROM agents WHERE project_id = ?")
                .await?;
            let mut rows = stmt.query([project_id]).await?;
            let mut all_names: Vec<String> = Vec::new();
            while let Some(row) = rows.next().await? {
                all_names.push(row.get(0)?);
            }

            // Find similar names using Levenshtein distance
            let name_refs: Vec<&str> = all_names.iter().map(|s| s.as_str()).collect();
            let similar = suggest_similar(name, &name_refs, 3);
            let suggestions: Vec<String> = similar.into_iter().map(|s| s.to_string()).collect();

            Err(crate::Error::agent_not_found_with_suggestions(
                format!("Name: {} in Project ID: {}", name, project_id),
                suggestions,
            ))
        }
    }

    /// Check if an active reviewer agent exists for a project.
    ///
    /// Used by workers to determine if they should send [COMPLETION] to a reviewer
    /// or directly to a human (self-review mode).
    ///
    /// An agent is considered "active" if its `last_active_ts` is within the
    /// specified `stale_threshold`. If `None` is provided for threshold,
    /// defaults to 1 hour.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager providing database access
    /// * `project_id` - Project database ID
    /// * `stale_threshold` - Optional duration; agents inactive longer are considered stale
    ///
    /// # Returns
    /// `Some(Agent)` if an active agent named "reviewer" exists, `None` otherwise
    pub async fn check_reviewer_exists(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        stale_threshold: Option<std::time::Duration>,
    ) -> Result<Option<Agent>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, project_id, name, program, model, task_description, inception_ts, last_active_ts, attachments_policy, contact_policy
            FROM agents WHERE project_id = ? AND LOWER(name) = 'reviewer'
            "#
        ).await?;
        let mut rows = stmt.query([project_id]).await?;

        if let Some(row) = rows.next().await? {
            let inception_ts_str: String = row.get(6)?;
            let inception_ts =
                NaiveDateTime::parse_from_str(&inception_ts_str, "%Y-%m-%d %H:%M:%S")
                    .unwrap_or_default();
            let last_active_ts_str: String = row.get(7)?;
            let last_active_ts =
                NaiveDateTime::parse_from_str(&last_active_ts_str, "%Y-%m-%d %H:%M:%S")
                    .unwrap_or_default();

            // Check if agent is stale (inactive beyond threshold)
            let threshold = stale_threshold.unwrap_or(std::time::Duration::from_secs(3600)); // 1 hour default
            let now = chrono::Utc::now().naive_utc();
            let threshold_chrono =
                chrono::Duration::from_std(threshold).unwrap_or(chrono::Duration::hours(1));
            let cutoff = now - threshold_chrono;

            if last_active_ts < cutoff {
                // Agent is stale, treat as non-existent
                return Ok(None);
            }

            Ok(Some(Agent {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                program: row.get(3)?,
                model: row.get(4)?,
                task_description: row.get(5)?,
                inception_ts,
                last_active_ts,
                attachments_policy: row.get(8)?,
                contact_policy: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Lists all agents in a project, ordered by name.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager providing database access
    /// * `project_id` - Project database ID
    ///
    /// # Returns
    /// Vector of all agents in the project (may be empty)
    pub async fn list_all_for_project(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
    ) -> Result<Vec<Agent>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, project_id, name, program, model, task_description, inception_ts, last_active_ts, attachments_policy, contact_policy
            FROM agents WHERE project_id = ? ORDER BY name ASC
            "#
        ).await?;
        let mut rows = stmt.query([project_id]).await?;

        let mut agents = Vec::new();
        while let Some(row) = rows.next().await? {
            // Column indices: 0=id, 1=project_id, 2=name, 3=program, 4=model,
            //                 5=task_description, 6=inception_ts, 7=last_active_ts,
            //                 8=attachments_policy, 9=contact_policy
            let inception_ts_str: String = row.get(6)?;
            let inception_ts =
                NaiveDateTime::parse_from_str(&inception_ts_str, "%Y-%m-%d %H:%M:%S")
                    .unwrap_or_default();
            let last_active_ts_str: String = row.get(7)?;
            let last_active_ts =
                NaiveDateTime::parse_from_str(&last_active_ts_str, "%Y-%m-%d %H:%M:%S")
                    .unwrap_or_default();

            agents.push(Agent {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                program: row.get(3)?,
                model: row.get(4)?,
                task_description: row.get(5)?,
                inception_ts,
                last_active_ts,
                attachments_policy: row.get(8)?,
                contact_policy: row.get(9)?,
            });
        }
        Ok(agents)
    }

    /// Counts the total messages sent by an agent.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager providing database access
    /// * `agent_id` - Agent database ID
    ///
    /// # Returns
    /// The count of messages where this agent is the sender.
    pub async fn count_messages_sent(_ctx: &Ctx, mm: &ModelManager, agent_id: i64) -> Result<i64> {
        let db = mm.db();
        let stmt = db
            .prepare("SELECT COUNT(*) FROM messages WHERE sender_id = ?")
            .await?;
        let mut rows = stmt.query([agent_id]).await?;
        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Ok(0)
        }
    }

    /// Counts the total messages received by an agent.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager providing database access
    /// * `agent_id` - Agent database ID
    ///
    /// # Returns
    /// The count of messages where this agent is a recipient.
    pub async fn count_messages_received(
        _ctx: &Ctx,
        mm: &ModelManager,
        agent_id: i64,
    ) -> Result<i64> {
        let db = mm.db();
        let stmt = db
            .prepare("SELECT COUNT(*) FROM message_recipients WHERE agent_id = ?")
            .await?;
        let mut rows = stmt.query([agent_id]).await?;
        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Ok(0)
        }
    }

    /// Updates an agent's profile fields.
    ///
    /// Only non-None fields in the update struct are modified.
    /// Automatically updates the last_active_ts timestamp.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager providing database access
    /// * `agent_id` - Agent database ID
    /// * `update` - Profile fields to update (partial updates)
    ///
    /// # Errors
    /// Returns an error if the agent ID doesn't exist
    pub async fn update_profile(
        _ctx: &Ctx,
        mm: &ModelManager,
        agent_id: i64,
        update: AgentProfileUpdate,
    ) -> Result<()> {
        let db = mm.db();

        if let Some(task_description) = update.task_description {
            let stmt = db
                .prepare("UPDATE agents SET task_description = ? WHERE id = ?")
                .await?;
            stmt.execute((task_description, agent_id)).await?;
        }

        if let Some(attachments_policy) = update.attachments_policy {
            let stmt = db
                .prepare("UPDATE agents SET attachments_policy = ? WHERE id = ?")
                .await?;
            stmt.execute((attachments_policy, agent_id)).await?;
        }

        if let Some(contact_policy) = update.contact_policy {
            let stmt = db
                .prepare("UPDATE agents SET contact_policy = ? WHERE id = ?")
                .await?;
            stmt.execute((contact_policy, agent_id)).await?;
        }

        // Update last_active_ts
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let stmt = db
            .prepare("UPDATE agents SET last_active_ts = ? WHERE id = ?")
            .await?;
        stmt.execute((now_str, agent_id)).await?;

        Ok(())
    }
}

/// Partial update for agent profile fields.
///
/// Only non-None fields will be updated. This allows updating
/// individual fields without affecting others.
///
/// # Example
///
/// ```
/// use lib_core::model::agent::AgentProfileUpdate;
///
/// // Update only the task description
/// let update = AgentProfileUpdate {
///     task_description: Some("New task".to_string()),
///     ..Default::default()
/// };
/// ```
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AgentProfileUpdate {
    /// New task description (if updating).
    pub task_description: Option<String>,
    /// New attachments policy (if updating).
    pub attachments_policy: Option<String>,
    /// New contact policy (if updating).
    pub contact_policy: Option<String>,
}
