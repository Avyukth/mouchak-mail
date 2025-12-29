//! Project management for multi-agent workspaces.
//!
//! Projects are the top-level organizational unit in Mouchak Mail.
//! Each project contains agents, messages, and configuration for a
//! collaborative AI coding workspace.
//!
//! # Data Model
//!
//! - **Project**: Main entity with slug identifier and human-readable name
//! - **ProjectBmc**: BMC with CRUD operations and Git archive management
//!
//! # Git Archive
//!
//! Each project maintains a Git archive for audit logging:
//! - `projects/{slug}/agents/` - Agent profiles
//! - `projects/{slug}/messages/` - Message history
//! - `projects/{slug}/mailbox.json` - Full mailbox export
//!
//! # Example
//!
//! ```no_run
//! use mouchak_mail_core::model::project::ProjectBmc;
//! use mouchak_mail_core::model::ModelManager;
//! use mouchak_mail_core::ctx::Ctx;
//!
//! # async fn example() -> mouchak_mail_core::Result<()> {
//! let mm = ModelManager::new(std::sync::Arc::new(mouchak_mail_common::config::AppConfig::default())).await?;
//! let ctx = Ctx::root_ctx();
//!
//! // Create a new project
//! let id = ProjectBmc::create(&ctx, &mm, "my-app", "My Application").await?;
//!
//! // Retrieve by slug
//! let project = ProjectBmc::get_by_slug(&ctx, &mm, "my-app").await?;
//! println!("Project: {}", project.human_key);
//! # Ok(())
//! # }
//! ```

use crate::Result;
use crate::model::ModelManager;
use crate::store::git_store;
use crate::types::ProjectId;
use crate::utils::mistake_detection::suggest_similar;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A project workspace for AI agents.
///
/// Projects provide isolation and organization for multi-agent collaboration.
/// Each project has a unique slug (URL-safe) and human-readable key.
///
/// # Fields
///
/// - `id` - Database primary key (strongly typed as [`ProjectId`])
/// - `slug` - URL-safe identifier (e.g., "my-project")
/// - `human_key` - Human-readable name (e.g., "My Project")
/// - `created_at` - Timestamp of project creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Database primary key (strongly typed).
    pub id: ProjectId,
    /// URL-safe project identifier.
    pub slug: String,
    /// Human-readable project name.
    pub human_key: String,
    /// Project creation timestamp.
    pub created_at: NaiveDateTime,
}

/// Backend Model Controller for Project operations.
///
/// Manages projects which are the top-level organizational unit for agents and messages.
pub struct ProjectBmc;

impl ProjectBmc {
    /// Creates a new project with Git archive initialization.
    ///
    /// This method:
    /// 1. Inserts project into database
    /// 2. Creates project directory structure in Git
    /// 3. Initializes README.md and .gitkeep files
    ///
    /// # Arguments
    /// * `ctx` - Request context
    /// * `mm` - ModelManager providing database and Git access
    /// * `slug` - URL-safe project identifier (lowercase, hyphenated)
    /// * `human_key` - Human-friendly project identifier
    ///
    /// # Returns
    /// The created project's database ID
    ///
    /// # Errors
    /// Returns an error if project slug already exists
    ///
    /// # Example
    /// ```no_run
    /// # use mouchak_mail_core::model::project::ProjectBmc;
    /// # use mouchak_mail_core::model::ModelManager;
    /// # use mouchak_mail_core::ctx::Ctx;
    /// # async fn example(mm: &ModelManager) {
    /// let ctx = Ctx::root_ctx();
    /// let id = ProjectBmc::create(&ctx, mm, "my-project", "My Project").await.unwrap();
    /// # }
    /// ```
    pub async fn create(
        ctx: &crate::Ctx,
        mm: &ModelManager,
        slug: &str,
        human_key: &str,
    ) -> Result<ProjectId> {
        let db = mm.db();

        // Execute insert
        let stmt = db
            .prepare("INSERT INTO projects (slug, human_key) VALUES (?, ?) RETURNING id")
            .await?;
        let mut rows = stmt.query([slug, human_key]).await?;

        let id: i64 = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput(
                "Failed to create project".into(),
            ));
        };

        Self::ensure_archive(mm, slug).await?;

        // Register built-in macros for this project
        let _ = super::macro_def::MacroDefBmc::ensure_builtin_macros(ctx, mm, id).await;

        Ok(ProjectId::new(id))
    }

    /// Lists all projects ordered by creation time (newest first).
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager providing database access
    ///
    /// # Returns
    /// Vector of all projects (may be empty)
    pub async fn list_all(_ctx: &crate::Ctx, mm: &ModelManager) -> Result<Vec<Project>> {
        let db = mm.db();
        let stmt = db
            .prepare(
                "SELECT id, slug, human_key, created_at FROM projects ORDER BY created_at DESC",
            )
            .await?;
        let mut rows = stmt.query(()).await?;

        let mut projects = Vec::new();
        while let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            projects.push(Project {
                id: ProjectId::new(row.get(0)?),
                slug: row.get(1)?,
                human_key: row.get(2)?,
                created_at,
            });
        }
        Ok(projects)
    }

    /// Retrieves a project by its slug (URL-safe identifier).
    ///
    /// # Arguments
    /// * `_ctx` - Request context  
    /// * `mm` - ModelManager
    /// * `slug` - Project slug (e.g., "my-project")
    ///
    /// # Returns
    /// The project data
    ///
    /// # Errors
    /// Returns `Error::ProjectNotFound` if slug doesn't exist
    pub async fn get_by_slug(_ctx: &crate::Ctx, mm: &ModelManager, slug: &str) -> Result<Project> {
        let db = mm.db();
        // Note: We are mapping manually because libsql doesn't have FromRow like sqlx yet
        let stmt = db
            .prepare("SELECT id, slug, human_key, created_at FROM projects WHERE slug = ?")
            .await?;
        let mut rows = stmt.query([slug]).await?;

        if let Some(row) = rows.next().await? {
            // created_at is stored as string in SQLite by default usually or int if unix timestamp
            // Let's assume string for datetime in this migration
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default(); // Simplification for MVP

            Ok(Project {
                id: ProjectId::new(row.get(0)?),
                slug: row.get(1)?,
                human_key: row.get(2)?,
                created_at,
            })
        } else {
            // Fetch all project slugs for suggestions
            let stmt = db.prepare("SELECT slug FROM projects").await?;
            let mut rows = stmt.query(()).await?;
            let mut all_slugs: Vec<String> = Vec::new();
            while let Some(row) = rows.next().await? {
                all_slugs.push(row.get(0)?);
            }

            let slug_refs: Vec<&str> = all_slugs.iter().map(|s| s.as_str()).collect();
            let similar = suggest_similar(slug, &slug_refs, 3);
            let suggestions: Vec<String> = similar.into_iter().map(|s| s.to_string()).collect();

            Err(crate::Error::project_not_found_with_suggestions(
                format!("Slug: {}", slug),
                suggestions,
            ))
        }
    }

    /// Retrieves a project by its human-readable key.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `human_key` - Human-readable project name
    ///
    /// # Returns
    /// The project data
    ///
    /// # Errors
    /// Returns `Error::ProjectNotFound` if human_key doesn't exist
    pub async fn get_by_human_key(
        _ctx: &crate::Ctx,
        mm: &ModelManager,
        human_key: &str,
    ) -> Result<Project> {
        let db = mm.db();
        let stmt = db
            .prepare("SELECT id, slug, human_key, created_at FROM projects WHERE human_key = ?")
            .await?;
        let mut rows = stmt.query([human_key]).await?;

        if let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            Ok(Project {
                id: ProjectId::new(row.get(0)?),
                slug: row.get(1)?,
                human_key: row.get(2)?,
                created_at,
            })
        } else {
            // Fetch all human_keys for suggestions
            let stmt = db.prepare("SELECT human_key FROM projects").await?;
            let mut rows = stmt.query(()).await?;
            let mut all_keys: Vec<String> = Vec::new();
            while let Some(row) = rows.next().await? {
                all_keys.push(row.get(0)?);
            }

            let key_refs: Vec<&str> = all_keys.iter().map(|s| s.as_str()).collect();
            let similar = suggest_similar(human_key, &key_refs, 3);
            let suggestions: Vec<String> = similar.into_iter().map(|s| s.to_string()).collect();

            Err(crate::Error::project_not_found_with_suggestions(
                format!("Human Key: {}", human_key),
                suggestions,
            ))
        }
    }

    /// Get project by identifier - tries slug first, then human_key.
    ///
    /// This is a convenience method that allows APIs to accept either slug
    /// or human_key as the project identifier parameter.
    ///
    /// # Arguments
    /// * `ctx` - Request context
    /// * `mm` - ModelManager
    /// * `identifier` - Either a slug or human_key
    ///
    /// # Returns
    /// The matching project
    ///
    /// # Errors
    /// Returns `Error::ProjectNotFound` if no match for either slug or human_key
    pub async fn get_by_identifier(
        ctx: &crate::Ctx,
        mm: &ModelManager,
        identifier: &str,
    ) -> Result<Project> {
        // First try by slug
        if let Ok(project) = Self::get_by_slug(ctx, mm, identifier).await {
            return Ok(project);
        }

        // Then try by human_key
        if let Ok(project) = Self::get_by_human_key(ctx, mm, identifier).await {
            return Ok(project);
        }

        // Finally, try slugified version of the identifier as slug
        let slugified = crate::utils::slugify(identifier);
        if let Ok(project) = Self::get_by_slug(ctx, mm, &slugified).await {
            return Ok(project);
        }

        // Fetch both slugs and human_keys for suggestions
        let db = mm.db();
        let stmt = db.prepare("SELECT slug, human_key FROM projects").await?;
        let mut rows = stmt.query(()).await?;
        let mut all_identifiers: Vec<String> = Vec::new();
        while let Some(row) = rows.next().await? {
            let slug: String = row.get(0)?;
            let human_key: String = row.get(1)?;
            all_identifiers.push(slug);
            all_identifiers.push(human_key);
        }

        let id_refs: Vec<&str> = all_identifiers.iter().map(|s| s.as_str()).collect();
        let similar = suggest_similar(identifier, &id_refs, 3);
        let suggestions: Vec<String> = similar.into_iter().map(|s| s.to_string()).collect();

        Err(crate::Error::project_not_found_with_suggestions(
            format!("Identifier: {}", identifier),
            suggestions,
        ))
    }

    /// Ensures the Git archive directory structure exists for a project.
    ///
    /// Creates the project directory and initializes `.gitattributes` if needed.
    /// This is called automatically during project creation.
    ///
    /// # Arguments
    /// * `mm` - ModelManager providing Git access
    /// * `slug` - Project slug for directory naming
    pub async fn ensure_archive(mm: &ModelManager, slug: &str) -> Result<()> {
        let repo_root = &mm.repo_root;
        let project_root = repo_root.join("projects").join(slug);

        if !project_root.exists() {
            std::fs::create_dir_all(&project_root)?;
        }

        // Git Operations - serialized to prevent lock contention
        let _git_guard = mm.git_lock.lock().await;

        // Initialize or open repo at repo_root (not project_root, as per python code single global archive repo)
        // Wait, python code says "single global archive repo" at settings.storage.root
        // and "Project-specific root inside the single global archive repo".
        // So we init the repo at mm.repo_root once.

        let repo = git_store::init_or_open_repo(repo_root)?;

        // Check if .gitattributes exists, if not create and commit
        let attributes_path = ".gitattributes";
        if git_store::read_file_content(&repo, attributes_path).is_err() {
            git_store::commit_file(
                &repo,
                Path::new(attributes_path),
                "*.json text\n*.md text\n",
                "chore: initialize archive",
                "mcp-bot",
                "mcp-bot@localhost",
            )?;
        }

        Ok(())
    }

    /// Counts total messages in a project.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `project_id` - Project database ID
    ///
    /// # Returns
    /// The total message count for the project.
    pub async fn count_messages(
        _ctx: &crate::Ctx,
        mm: &ModelManager,
        project_id: ProjectId,
    ) -> Result<i64> {
        let db = mm.db();
        let stmt = db
            .prepare("SELECT COUNT(*) FROM messages WHERE project_id = ?")
            .await?;
        let mut rows = stmt.query([project_id.get()]).await?;
        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Ok(0)
        }
    }

    /// Retrieves a project by its database ID.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `id` - Project database ID
    ///
    /// # Returns
    /// The project data
    ///
    /// # Errors
    /// Returns `Error::ProjectNotFound` if ID doesn't exist
    pub async fn get(_ctx: &crate::Ctx, mm: &ModelManager, id: ProjectId) -> Result<Project> {
        let db = mm.db();
        let stmt = db
            .prepare("SELECT id, slug, human_key, created_at FROM projects WHERE id = ?")
            .await?;
        let mut rows = stmt.query([id.get()]).await?;

        if let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            Ok(Project {
                id: ProjectId::new(row.get(0)?),
                slug: row.get(1)?,
                human_key: row.get(2)?,
                created_at,
            })
        } else {
            Err(crate::Error::project_not_found(format!("ID: {}", id.get())))
        }
    }

    /// List sibling projects (projects sharing at least one product)
    pub async fn list_siblings(
        ctx: &crate::Ctx,
        mm: &ModelManager,
        project_id: ProjectId,
    ) -> Result<Vec<Project>> {
        // 1. Get products for this project
        let products =
            crate::model::product::ProductBmc::list_for_project(ctx, mm, project_id.get()).await?;

        let mut sibling_ids = std::collections::HashSet::new();

        // 2. Get all projects for these products
        for product in products {
            let linked_ids =
                crate::model::product::ProductBmc::get_linked_projects(ctx, mm, product.id).await?;
            for pid in linked_ids {
                if pid != project_id.get() {
                    sibling_ids.insert(pid);
                }
            }
        }

        // 3. Fetch project details
        let mut siblings = Vec::new();
        for pid in sibling_ids {
            // We use get which might fail if project deleted but link remains?
            // DB constraint should prevent this, but safe to ignore error or handle
            if let Ok(proj) = Self::get(ctx, mm, ProjectId::new(pid)).await {
                siblings.push(proj);
            }
        }

        // Sort by slug for consistency
        siblings.sort_by(|a, b| a.slug.cmp(&b.slug));

        Ok(siblings)
    }

    /// Sync project state to git archive.
    ///
    /// Writes project data (mailbox, etc.) to files in the repo and commits them.
    pub async fn sync_to_archive(
        ctx: &crate::Ctx,
        mm: &ModelManager,
        project_id: ProjectId,
        message: &str,
    ) -> Result<String> {
        // 1. Get project
        let project = Self::get(ctx, mm, project_id).await?;
        let repo_root = &mm.repo_root;
        let project_root = repo_root.join("projects").join(&project.slug);

        // Ensure directory exists (it should, but just in case)
        if !project_root.exists() {
            std::fs::create_dir_all(&project_root)?;
        }

        // 2. Export Mailbox (JSON)
        // reuse ExportBmc logic but specifically for archive
        let messages =
            crate::model::message::MessageBmc::list_recent(ctx, mm, project_id, 1000).await?; // reasonable limit for archive?
        let mailbox_json = serde_json::to_string_pretty(&messages)?;
        let mailbox_path = project_root.join("mailbox.json");
        std::fs::write(&mailbox_path, mailbox_json)?;

        // 3. Export Agents (JSON)
        let agents =
            crate::model::agent::AgentBmc::list_all_for_project(ctx, mm, project_id).await?;
        let agents_json = serde_json::to_string_pretty(&agents)?;
        let agents_path = project_root.join("agents.json");
        std::fs::write(&agents_path, agents_json)?;

        // 4. Commit using git_store - serialized to prevent lock contention
        let _git_guard = mm.git_lock.lock().await;

        // We need to pass paths relative to the repo root
        // Since we are inside repo_root, relative path is `projects/{slug}/mailbox.json`
        let relative_mailbox = Path::new("projects")
            .join(&project.slug)
            .join("mailbox.json");
        let relative_agents = Path::new("projects")
            .join(&project.slug)
            .join("agents.json");

        // Use cached repository to prevent FD exhaustion
        let repo_arc = mm.get_repo().await?;
        let repo = repo_arc.lock().await;

        let oid = git_store::commit_paths(
            &repo,
            &[relative_mailbox, relative_agents],
            message,
            "mcp-bot",
            "mcp-bot@localhost",
        )?;

        Ok(oid.to_string())
    }

    /// Deletes a project and all related data (cascade delete).
    ///
    /// This method performs a cascading delete in the correct order to respect
    /// foreign key constraints. SQLite does not enforce FK cascades by default,
    /// so we manually delete in dependency order:
    ///
    /// 1. message_recipients (references messages and agents)
    /// 2. messages_fts (FTS5 virtual table, synced with messages)
    /// 3. messages (references project and sender agent)
    /// 4. file_reservations, build_slots, macros, overseer_messages
    /// 5. agent_links (references agents)
    /// 6. project_sibling_suggestions
    /// 7. agents (references project)
    /// 8. product_project_links
    /// 9. project itself
    ///
    /// Also removes the project directory from the Git archive.
    ///
    /// # Arguments
    /// * `ctx` - Request context
    /// * `mm` - ModelManager providing database and Git access
    /// * `project_id` - The project database ID to delete
    ///
    /// # Returns
    /// `Ok(())` on successful deletion
    ///
    /// # Errors
    /// Returns an error if:
    /// - Project ID doesn't exist
    /// - Database operations fail
    ///
    /// # Example
    /// ```no_run
    /// # use mouchak_mail_core::model::project::ProjectBmc;
    /// # use mouchak_mail_core::model::ModelManager;
    /// # use mouchak_mail_core::ctx::Ctx;
    /// # use mouchak_mail_core::types::ProjectId;
    /// # async fn example(mm: &ModelManager) {
    /// let ctx = Ctx::root_ctx();
    /// ProjectBmc::delete(&ctx, mm, ProjectId::new(42)).await.unwrap();
    /// # }
    /// ```
    pub async fn delete(ctx: &crate::Ctx, mm: &ModelManager, project_id: ProjectId) -> Result<()> {
        let db = mm.db();
        let pid = project_id.get();

        // First, verify project exists and get slug for git cleanup
        let project = Self::get(ctx, mm, project_id).await?;
        let project_slug = project.slug.clone();

        // Get all agent IDs for this project (needed for message_recipients cleanup)
        let agents = super::agent::AgentBmc::list_all_for_project(ctx, mm, project_id).await?;
        let agent_ids: Vec<i64> = agents.iter().map(|a| a.id.get()).collect();

        // 1. Delete message_recipients for messages in this project
        // (message_recipients references both messages and agents)
        let stmt = db
            .prepare(
                r#"
                DELETE FROM message_recipients
                WHERE message_id IN (SELECT id FROM messages WHERE project_id = ?)
                "#,
            )
            .await?;
        stmt.execute([pid]).await?;

        // 2. Delete messages (FTS5 trigger handles messages_fts automatically)
        let stmt = db
            .prepare("DELETE FROM messages WHERE project_id = ?")
            .await?;
        stmt.execute([pid]).await?;

        // 3. Delete file_reservations
        let stmt = db
            .prepare("DELETE FROM file_reservations WHERE project_id = ?")
            .await?;
        stmt.execute([pid]).await?;

        // 4. Delete build_slots
        let stmt = db
            .prepare("DELETE FROM build_slots WHERE project_id = ?")
            .await?;
        stmt.execute([pid]).await?;

        // 5. Delete macros
        let stmt = db
            .prepare("DELETE FROM macros WHERE project_id = ?")
            .await?;
        stmt.execute([pid]).await?;

        // 6. Delete overseer_messages
        let stmt = db
            .prepare("DELETE FROM overseer_messages WHERE project_id = ?")
            .await?;
        stmt.execute([pid]).await?;

        // 7. Delete agent_links
        if !agent_ids.is_empty() {
            let placeholders = agent_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                "DELETE FROM agent_links WHERE a_agent_id IN ({}) OR b_agent_id IN ({})",
                placeholders, placeholders
            );
            let stmt = db.prepare(&sql).await?;
            let params: Vec<libsql::Value> = agent_ids
                .iter()
                .chain(agent_ids.iter())
                .map(|id| libsql::Value::Integer(*id))
                .collect();
            stmt.execute(params).await?;
        }

        // 8. Delete project_sibling_suggestions
        let stmt = db
            .prepare("DELETE FROM project_sibling_suggestions WHERE project_a_id = ? OR project_b_id = ?")
            .await?;
        stmt.execute([pid, pid]).await?;

        // 9. Delete agents
        let stmt = db
            .prepare("DELETE FROM agents WHERE project_id = ?")
            .await?;
        stmt.execute([pid]).await?;

        // 10. Delete product_project_links
        let stmt = db
            .prepare("DELETE FROM product_project_links WHERE project_id = ?")
            .await?;
        stmt.execute([pid]).await?;

        // 11. Delete the project itself
        let stmt = db.prepare("DELETE FROM projects WHERE id = ?").await?;
        stmt.execute([pid]).await?;

        // 12. Clean up Git archive
        let project_dir = mm.repo_root.join("projects").join(&project_slug);
        if project_dir.exists() {
            std::fs::remove_dir_all(&project_dir)?;

            let _git_guard = mm.git_lock.lock().await;
            let repo_arc = mm.get_repo().await?;
            let repo = repo_arc.lock().await;

            let relative_path = Path::new("projects").join(&project_slug);
            git_store::commit_deletion(
                &repo,
                &relative_path,
                &format!("chore: delete project {}", project_slug),
                "mcp-bot",
                "mcp-bot@localhost",
            )?;
        }

        Ok(())
    }

    /// Adopts (merges) artifacts from one project to another.
    ///
    /// Moves all agents and messages from `from_project_id` to `to_project_id`.
    /// Note: This may fail if there are naming conflicts (e.g. agent name exists in dest).
    pub async fn adopt(
        ctx: &crate::Ctx,
        mm: &ModelManager,
        from_project_id: ProjectId,
        to_project_id: ProjectId,
    ) -> Result<()> {
        let db = mm.db();
        let from_pid = from_project_id.get();
        let to_pid = to_project_id.get();

        // 1. Move Agents
        // We use execute directly
        // Note: Using prepare_cached or prepare
        let stmt = db
            .prepare("UPDATE agents SET project_id = ? WHERE project_id = ?")
            .await?;
        stmt.execute([to_pid, from_pid]).await?;

        // 2. Move Messages
        let stmt = db
            .prepare("UPDATE messages SET project_id = ? WHERE project_id = ?")
            .await?;
        stmt.execute([to_pid, from_pid]).await?;

        // 3. Move other entities (Optional but recommended for full adopt)
        // File Reservations
        let stmt = db
            .prepare("UPDATE file_reservations SET project_id = ? WHERE project_id = ?")
            .await?;
        stmt.execute([to_pid, from_pid]).await?;

        // Build Slots
        let stmt = db
            .prepare("UPDATE build_slots SET project_id = ? WHERE project_id = ?")
            .await?;
        stmt.execute([to_pid, from_pid]).await?;

        // 4. Sync destination to archive
        Self::sync_to_archive(ctx, mm, to_project_id, "Adopted artifacts").await?;

        Ok(())
    }
}
