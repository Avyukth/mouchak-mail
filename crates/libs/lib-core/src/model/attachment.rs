use crate::model::ModelManager;
use crate::{Ctx, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)] 
// Note: We don't have db_macro::FromRow, we use manual implementation usually or sqlx (now libsql).
// I will implement from_row manually as seen in other files.
pub struct Attachment {
    pub id: i64,
    pub project_id: i64,
    pub filename: String,
    pub stored_path: String,
    pub media_type: String,
    pub size_bytes: i64,
    pub created_ts: String,
}

#[derive(Deserialize)]
pub struct AttachmentForCreate {
    pub project_id: i64,
    pub filename: String,
    pub stored_path: String,
    pub media_type: String,
    pub size_bytes: i64,
}

pub struct AttachmentBmc;



impl AttachmentBmc {
    pub async fn create(
        _ctx: &Ctx,
        mm: &ModelManager,
        attachment_c: AttachmentForCreate,
    ) -> Result<i64> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let created_ts = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db.prepare(
            "INSERT INTO attachments (project_id, filename, stored_path, media_type, size_bytes, created_ts) VALUES (?, ?, ?, ?, ?, ?) RETURNING id"
        ).await?;
        
        let mut rows = stmt.query((
            attachment_c.project_id,
            attachment_c.filename,
            attachment_c.stored_path,
            attachment_c.media_type,
            attachment_c.size_bytes,
            created_ts,
        )).await?;

        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Err(crate::Error::InvalidInput("Failed to create attachment".into()))
        }
    }

    pub async fn get(
        _ctx: &Ctx,
        mm: &ModelManager,
        id: i64,
    ) -> Result<Attachment> {
        let db = mm.db();
        let stmt = db.prepare("SELECT id, project_id, filename, stored_path, media_type, size_bytes, created_ts FROM attachments WHERE id = ?").await?;
        let mut rows = stmt.query([id]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Self::from_row(row)?)
        } else {
            Err(crate::Error::NotFound)
        }
    }

    pub async fn list_by_project(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
    ) -> Result<Vec<Attachment>> {
        let db = mm.db();
        let stmt = db.prepare("SELECT id, project_id, filename, stored_path, media_type, size_bytes, created_ts FROM attachments WHERE project_id = ? ORDER BY id DESC").await?;
        let mut rows = stmt.query([project_id]).await?;
        
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
            filename: row.get(2)?,
            stored_path: row.get(3)?,
            media_type: row.get(4)?,
            size_bytes: row.get(5)?,
            created_ts: row.get(6)?,
        })
    }
}
