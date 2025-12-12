
use crate::model::ModelManager;
use crate::Result;
use crate::store::git_store;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub slug: String,
    pub human_key: String,
    pub created_at: NaiveDateTime,
}

pub struct ProjectBmc;

impl ProjectBmc {
    pub async fn create(ctx: &crate::Ctx, mm: &ModelManager, slug: &str, human_key: &str) -> Result<i64> {
        let db = mm.db();

        // Execute insert
        let stmt = db.prepare("INSERT INTO projects (slug, human_key) VALUES (?, ?) RETURNING id").await?;
        let mut rows = stmt.query([slug, human_key]).await?;
        
        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput("Failed to create project".into()));
        };

        Self::ensure_archive(mm, slug).await?;
        
        // Register built-in macros for this project
        let _ = super::macro_def::MacroDefBmc::ensure_builtin_macros(ctx, mm, id).await;

        Ok(id)
    }

    pub async fn list_all(_ctx: &crate::Ctx, mm: &ModelManager) -> Result<Vec<Project>> {
        let db = mm.db();
        let stmt = db.prepare("SELECT id, slug, human_key, created_at FROM projects ORDER BY created_at DESC").await?;
        let mut rows = stmt.query(()).await?;
        
        let mut projects = Vec::new();
        while let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            projects.push(Project {
                id: row.get(0)?,
                slug: row.get(1)?,
                human_key: row.get(2)?,
                created_at,
            });
        }
        Ok(projects)
    }

    pub async fn get_by_slug(_ctx: &crate::Ctx, mm: &ModelManager, slug: &str) -> Result<Project> {
        let db = mm.db();
        // Note: We are mapping manually because libsql doesn't have FromRow like sqlx yet
        let stmt = db.prepare("SELECT id, slug, human_key, created_at FROM projects WHERE slug = ?").await?;
        let mut rows = stmt.query([slug]).await?;
        
        if let Some(row) = rows.next().await? {
            // created_at is stored as string in SQLite by default usually or int if unix timestamp
            // Let's assume string for datetime in this migration
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default(); // Simplification for MVP

            Ok(Project {
                id: row.get(0)?,
                slug: row.get(1)?,
                human_key: row.get(2)?,
                created_at,
            })
        } else {
            Err(crate::Error::ProjectNotFound(format!("Slug: {}", slug)))
        }
    }

    pub async fn get_by_human_key(_ctx: &crate::Ctx, mm: &ModelManager, human_key: &str) -> Result<Project> {
        let db = mm.db();
        let stmt = db.prepare("SELECT id, slug, human_key, created_at FROM projects WHERE human_key = ?").await?;
        let mut rows = stmt.query([human_key]).await?;

        if let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            Ok(Project {
                id: row.get(0)?,
                slug: row.get(1)?,
                human_key: row.get(2)?,
                created_at,
            })
        } else {
            Err(crate::Error::ProjectNotFound(format!("Human Key: {}", human_key)))
        }
    }

    /// Get project by identifier - tries slug first, then human_key.
    /// This allows APIs to accept either slug or human_key as project_key parameter.
    pub async fn get_by_identifier(ctx: &crate::Ctx, mm: &ModelManager, identifier: &str) -> Result<Project> {
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

        Err(crate::Error::ProjectNotFound(format!("Identifier: {}", identifier)))
    }

    pub async fn ensure_archive(mm: &ModelManager, slug: &str) -> Result<()> {
        let repo_root = &mm.repo_root;
        let project_root = repo_root.join("projects").join(slug);

        if !project_root.exists() {
            std::fs::create_dir_all(&project_root)?;
        }

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

    pub async fn count_messages(_ctx: &crate::Ctx, mm: &ModelManager, project_id: i64) -> Result<i64> {
        let db = mm.db();
        let stmt = db.prepare("SELECT COUNT(*) FROM messages WHERE project_id = ?").await?;
        let mut rows = stmt.query([project_id]).await?;
        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Ok(0)
        }
    }

    pub async fn get(_ctx: &crate::Ctx, mm: &ModelManager, id: i64) -> Result<Project> {
        let db = mm.db();
        let stmt = db.prepare("SELECT id, slug, human_key, created_at FROM projects WHERE id = ?").await?;
        let mut rows = stmt.query([id]).await?;

        if let Some(row) = rows.next().await? {
            let created_at_str: String = row.get(3)?;
            let created_at = NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default();

            Ok(Project {
                id: row.get(0)?,
                slug: row.get(1)?,
                human_key: row.get(2)?,
                created_at,
            })
        } else {
            Err(crate::Error::ProjectNotFound(format!("ID: {}", id)))
        }
    }

    /// List sibling projects (projects sharing at least one product)
    pub async fn list_siblings(ctx: &crate::Ctx, mm: &ModelManager, project_id: i64) -> Result<Vec<Project>> {
        // 1. Get products for this project
        let products = crate::model::product::ProductBmc::list_for_project(ctx, mm, project_id).await?;
        
        let mut sibling_ids = std::collections::HashSet::new();
        
        // 2. Get all projects for these products
        for product in products {
            let linked_ids = crate::model::product::ProductBmc::get_linked_projects(ctx, mm, product.id).await?;
            for pid in linked_ids {
                if pid != project_id {
                    sibling_ids.insert(pid);
                }
            }
        }

        // 3. Fetch project details
        let mut siblings = Vec::new();
        for pid in sibling_ids {
            // We use get which might fail if project deleted but link remains? 
            // DB constraint should prevent this, but safe to ignore error or handle
            if let Ok(proj) = Self::get(ctx, mm, pid).await {
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
    pub async fn sync_to_archive(ctx: &crate::Ctx, mm: &ModelManager, project_id: i64, message: &str) -> Result<String> {
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
        let messages = crate::model::message::MessageBmc::list_recent(ctx, mm, project_id, 1000).await?; // reasonable limit for archive?
        let mailbox_json = serde_json::to_string_pretty(&messages)?;
        let mailbox_path = project_root.join("mailbox.json");
        std::fs::write(&mailbox_path, mailbox_json)?;

        // 3. Export Agents (JSON)
        let agents = crate::model::agent::AgentBmc::list_all_for_project(ctx, mm, project_id).await?;
        let agents_json = serde_json::to_string_pretty(&agents)?;
        let agents_path = project_root.join("agents.json");
        std::fs::write(&agents_path, agents_json)?;

        // 4. Commit using git_store
        // We need to pass paths relative to the repo root
        // Since we are inside repo_root, relative path is `projects/{slug}/mailbox.json`
        let relative_mailbox = Path::new("projects").join(&project.slug).join("mailbox.json");
        let relative_agents = Path::new("projects").join(&project.slug).join("agents.json");
        
        // We open the repo at mm.repo_root
        let repo = git_store::open_repo(repo_root)?;
        
        let oid = git_store::commit_paths(
            &repo,
            &[relative_mailbox, relative_agents],
            message,
            "mcp-bot",
            "mcp-bot@localhost"
        )?;

        Ok(oid.to_string())
    }
}
