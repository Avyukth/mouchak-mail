
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
    pub async fn create(_ctx: &crate::Ctx, mm: &ModelManager, slug: &str, human_key: &str) -> Result<i64> {
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
}
