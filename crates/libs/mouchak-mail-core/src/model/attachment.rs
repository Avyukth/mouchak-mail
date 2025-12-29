//! File attachment management for projects.
//!
//! This module handles file attachments that can be shared between agents
//! in a project. Attachment metadata is stored in the database while the
//! actual files are stored on disk.
//!
//! # Storage
//!
//! - **Database**: Stores metadata (filename, path, media type, size)
//! - **Disk**: Actual file content at `stored_path`
//!
//! # Example
//!
//! ```no_run
//! use mouchak_mail_core::model::attachment::{AttachmentBmc, AttachmentForCreate};
//! use mouchak_mail_core::model::ModelManager;
//! use mouchak_mail_core::ctx::Ctx;
//!
//! # async fn example() -> mouchak_mail_core::Result<()> {
//! let mm = ModelManager::new(std::sync::Arc::new(mouchak_mail_common::config::AppConfig::default())).await?;
//! let ctx = Ctx::root_ctx();
//!
//! // Create attachment record (file already written to disk)
//! let attachment = AttachmentForCreate {
//!     project_id: 1,
//!     agent_id: None,
//!     filename: "report.pdf".to_string(),
//!     stored_path: "/data/uploads/abc123.pdf".to_string(),
//!     media_type: "application/pdf".to_string(),
//!     size_bytes: 1024,
//! };
//! let id = AttachmentBmc::create(&ctx, &mm, attachment).await?;
//! # Ok(())
//! # }
//! ```

use crate::model::ModelManager;
use crate::{Ctx, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// File attachment metadata.
///
/// Represents a file that has been uploaded and stored. The actual file
/// content is stored at `stored_path` on disk.
///
/// # Fields
///
/// - `id` - Database primary key
/// - `project_id` - Associated project
/// - `agent_id` - Optional agent that uploaded the file
/// - `filename` - Original filename
/// - `stored_path` - Path to file on disk
/// - `media_type` - MIME type (e.g., "application/pdf")
/// - `size_bytes` - File size in bytes
/// - `created_ts` - Upload timestamp
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Attachment {
    /// Database primary key.
    pub id: i64,
    /// Associated project ID.
    pub project_id: i64,
    /// Optional agent ID that uploaded the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<i64>,
    /// Original filename.
    pub filename: String,
    /// Path to file on disk.
    pub stored_path: String,
    /// MIME type.
    pub media_type: String,
    /// File size in bytes.
    pub size_bytes: i64,
    /// Upload timestamp.
    pub created_ts: String,
}

/// Input data for creating an attachment record.
///
/// The file must already be written to disk at `stored_path` before
/// creating the database record.
#[derive(Deserialize)]
pub struct AttachmentForCreate {
    /// Project to associate with.
    pub project_id: i64,
    /// Optional agent that uploaded the file.
    #[serde(default)]
    pub agent_id: Option<i64>,
    /// Original filename.
    pub filename: String,
    /// Path where file is stored.
    pub stored_path: String,
    /// MIME type of the file.
    pub media_type: String,
    /// File size in bytes.
    pub size_bytes: i64,
}

/// Backend Model Controller for Attachment operations.
///
/// Manages file attachments associated with projects. Files are stored
/// on disk and metadata is tracked in the database.
pub struct AttachmentBmc;

impl AttachmentBmc {
    /// Creates a new attachment record.
    ///
    /// **Note**: This only creates the database record. The actual file
    /// must be written to disk separately (typically by the API layer).
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `attachment_c` - Attachment metadata
    ///
    /// # Returns
    /// The created attachment's database ID
    pub async fn create(
        _ctx: &Ctx,
        mm: &ModelManager,
        attachment_c: AttachmentForCreate,
    ) -> Result<i64> {
        // Enforce Quota
        if mm.app_config.quota.enabled {
            let limit = mm.app_config.quota.attachments_limit_bytes as i64;
            if limit > 0 {
                let current_usage =
                    Self::get_total_project_usage(_ctx, mm, attachment_c.project_id).await?;
                if current_usage + attachment_c.size_bytes > limit {
                    return Err(crate::Error::QuotaExceeded(format!(
                        "Attachments limit reached. Current: {} bytes, New: {} bytes, Limit: {} bytes",
                        current_usage, attachment_c.size_bytes, limit
                    )));
                }
            }
        }

        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let created_ts = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db.prepare(
            "INSERT INTO attachments (project_id, agent_id, filename, stored_path, media_type, size_bytes, created_ts) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id"
        ).await?;

        let mut rows = stmt
            .query((
                attachment_c.project_id,
                attachment_c.agent_id,
                attachment_c.filename,
                attachment_c.stored_path,
                attachment_c.media_type,
                attachment_c.size_bytes,
                created_ts,
            ))
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Err(crate::Error::InvalidInput(
                "Failed to create attachment".into(),
            ))
        }
    }

    /// Retrieves an attachment by its database ID.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `id` - Attachment ID
    ///
    /// # Returns
    /// The attachment metadata
    ///
    /// # Errors
    /// Returns `Error::NotFound` if attachment doesn't exist
    pub async fn get(_ctx: &Ctx, mm: &ModelManager, id: i64) -> Result<Attachment> {
        let db = mm.db();
        let stmt = db.prepare("SELECT id, project_id, agent_id, filename, stored_path, media_type, size_bytes, created_ts FROM attachments WHERE id = ?").await?;
        let mut rows = stmt.query([id]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Self::from_row(row)?)
        } else {
            Err(crate::Error::NotFound)
        }
    }

    /// Lists all attachments for a project.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `project_id` - Project database ID
    ///
    /// # Returns
    /// Vector of attachments (newest first)
    pub async fn list_by_project(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
    ) -> Result<Vec<Attachment>> {
        let db = mm.db();
        let stmt = db.prepare("SELECT id, project_id, agent_id, filename, stored_path, media_type, size_bytes, created_ts FROM attachments WHERE project_id = ? ORDER BY id DESC").await?;
        let mut rows = stmt.query([project_id]).await?;

        let mut res = Vec::new();
        while let Some(row) = rows.next().await? {
            res.push(Self::from_row(row)?);
        }
        Ok(res)
    }

    /// Lists attachments for a project, optionally filtered by agent.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `project_id` - Project database ID
    /// * `agent_id` - Optional agent ID to filter by
    ///
    /// # Returns
    /// Vector of attachments (newest first)
    pub async fn list_by_project_and_agent(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
        agent_id: Option<i64>,
    ) -> Result<Vec<Attachment>> {
        let db = mm.db();

        let (sql, params): (&str, Vec<i64>) = match agent_id {
            Some(aid) => (
                "SELECT id, project_id, agent_id, filename, stored_path, media_type, size_bytes, created_ts FROM attachments WHERE project_id = ? AND agent_id = ? ORDER BY id DESC",
                vec![project_id, aid],
            ),
            None => (
                "SELECT id, project_id, agent_id, filename, stored_path, media_type, size_bytes, created_ts FROM attachments WHERE project_id = ? ORDER BY id DESC",
                vec![project_id],
            ),
        };

        let stmt = db.prepare(sql).await?;
        let mut rows = match agent_id {
            Some(_) => stmt.query([params[0], params[1]]).await?,
            None => stmt.query([params[0]]).await?,
        };

        let mut res = Vec::new();
        while let Some(row) = rows.next().await? {
            res.push(Self::from_row(row)?);
        }
        Ok(res)
    }

    fn from_row(row: libsql::Row) -> Result<Attachment> {
        Ok(Attachment {
            id: row.get(0)?,
            project_id: row.get(1)?,
            agent_id: row.get(2)?,
            filename: row.get(3)?,
            stored_path: row.get(4)?,
            media_type: row.get(5)?,
            size_bytes: row.get(6)?,
            created_ts: row.get(7)?,
        })
    }

    /// Calculate total attachment size in bytes for a project.
    pub async fn get_total_project_usage(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
    ) -> Result<i64> {
        let db = mm.db();
        // COALESCE ensures we get 0 instead of NULL if no rows match
        let stmt = db
            .prepare("SELECT COALESCE(SUM(size_bytes), 0) FROM attachments WHERE project_id = ?")
            .await?;
        let mut rows = stmt.query([project_id]).await?;

        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_has_agent_id_field() {
        // Verify the Attachment struct has agent_id field
        let attachment = Attachment {
            id: 1,
            project_id: 1,
            agent_id: Some(42),
            filename: "test.pdf".to_string(),
            stored_path: "/tmp/test.pdf".to_string(),
            media_type: "application/pdf".to_string(),
            size_bytes: 1024,
            created_ts: "2024-01-01 00:00:00".to_string(),
        };
        assert_eq!(attachment.agent_id, Some(42));
    }

    #[test]
    fn test_attachment_agent_id_can_be_none() {
        let attachment = Attachment {
            id: 1,
            project_id: 1,
            agent_id: None,
            filename: "test.pdf".to_string(),
            stored_path: "/tmp/test.pdf".to_string(),
            media_type: "application/pdf".to_string(),
            size_bytes: 1024,
            created_ts: "2024-01-01 00:00:00".to_string(),
        };
        assert!(attachment.agent_id.is_none());
    }

    #[test]
    fn test_attachment_for_create_has_agent_id() {
        let create = AttachmentForCreate {
            project_id: 1,
            agent_id: Some(42),
            filename: "test.pdf".to_string(),
            stored_path: "/tmp/test.pdf".to_string(),
            media_type: "application/pdf".to_string(),
            size_bytes: 1024,
        };
        assert_eq!(create.agent_id, Some(42));
    }

    #[test]
    fn test_attachment_for_create_agent_id_defaults_to_none() {
        // Verify serde default works
        let json = r#"{"project_id":1,"filename":"test.pdf","stored_path":"/tmp/test.pdf","media_type":"application/pdf","size_bytes":1024}"#;
        let create: AttachmentForCreate = serde_json::from_str(json).unwrap();
        assert!(create.agent_id.is_none());
    }
}
