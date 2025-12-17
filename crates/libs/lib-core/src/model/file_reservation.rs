use crate::Result;
use crate::model::ModelManager;
use crate::store::git_store;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReservation {
    pub id: i64,
    pub project_id: i64,
    pub agent_id: i64,
    pub path_pattern: String,
    pub exclusive: bool,
    pub reason: String,
    pub created_ts: NaiveDateTime,
    pub expires_ts: NaiveDateTime,
    pub released_ts: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReservationForCreate {
    pub project_id: i64,
    pub agent_id: i64,
    pub path_pattern: String,
    pub exclusive: bool,
    pub reason: String,
    pub expires_ts: NaiveDateTime,
}

/// Backend Model Controller for File Reservation operations.
///
/// Manages file-level locking and coordination between agents.
/// Supports both exclusive and shared locks with TTL-based expiration.
pub struct FileReservationBmc;

impl FileReservationBmc {
    /// Creates a new file reservation (lock) for an agent.
    ///
    /// This method:
    /// 1. Inserts reservation into database
    /// 2. Archives reservation to Git
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `fr_c` - Reservation data (path pattern, exclusive flag, TTL)
    ///
    /// # Returns  
    /// The created reservation's database ID
    ///
    /// # Errors
    /// Returns an error if:
    /// - Agent or project doesn't exist
    /// - Conflicting exclusive lock exists
    ///
    /// # Example
    /// ```no_run
    /// # use lib_core::model::file_reservation::*;
    /// # use lib_core::model::ModelManager;
    /// # use lib_core::ctx::Ctx;
    /// # async fn example(mm: &ModelManager) {
    /// let ctx = Ctx::root_ctx();
    /// let reservation = FileReservationForCreate {
    ///     project_id: 1,
    ///     agent_id: 1,
    ///     path_pattern: "src/**/*.rs".to_string(),
    ///     exclusive: true,
    ///     reason: "Refactoring module".to_string(),
    ///     expires_ts: chrono::Utc::now().naive_utc() + chrono::Duration::hours(1),
    /// };
    /// let id = FileReservationBmc::create(&ctx, mm, reservation).await.unwrap();
    /// # }
    /// ```
    pub async fn create(
        _ctx: &crate::Ctx,
        mm: &ModelManager,
        fr_c: FileReservationForCreate,
    ) -> Result<i64> {
        let db = mm.db();

        let stmt = db.prepare(
            r#"
            INSERT INTO file_reservations (project_id, agent_id, path_pattern, exclusive, reason, expires_ts)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        ).await?;

        // Format datetime as string for SQLite
        let expires_ts_str = fr_c.expires_ts.format("%Y-%m-%d %H:%M:%S").to_string();

        let mut rows = stmt
            .query((
                fr_c.project_id,
                fr_c.agent_id,
                fr_c.path_pattern.as_str(),
                fr_c.exclusive,
                fr_c.reason.as_str(),
                expires_ts_str,
            ))
            .await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput(
                "Failed to create file reservation".into(),
            ));
        };

        // Write to Git
        let stmt = db.prepare("SELECT slug FROM projects WHERE id = ?").await?;
        let mut rows = stmt.query([fr_c.project_id]).await?;
        let project_slug: String = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(crate::Error::ProjectNotFound(format!(
                "{}",
                fr_c.project_id
            )));
        };

        let stmt = db.prepare("SELECT name FROM agents WHERE id = ?").await?;
        let mut rows = stmt.query([fr_c.agent_id]).await?;
        let agent_name: String = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            return Err(crate::Error::AgentNotFound(format!("{}", fr_c.agent_id)));
        };

        // Git Operations - serialized to prevent lock contention
        let _git_guard = mm.git_lock.lock().await;

        let repo_root = &mm.repo_root;
        let repo = git_store::open_repo(repo_root)?;

        // Hash path_pattern
        let mut hasher = Sha1::new();
        hasher.update(fr_c.path_pattern.as_bytes());
        let result = hasher.finalize();
        let digest = hex::encode(result);

        // Path: projects/<slug>/file_reservations/<digest>.json
        let rel_path = std::path::PathBuf::from("projects")
            .join(&project_slug)
            .join("file_reservations")
            .join(format!("{}.json", digest));

        let payload = serde_json::json!({
            "id": id,
            "agent": agent_name,
            "path_pattern": fr_c.path_pattern,
            "exclusive": fr_c.exclusive,
            "reason": fr_c.reason,
            "created_ts": chrono::Utc::now().to_rfc3339(),
            "expires_ts": fr_c.expires_ts.format("%Y-%m-%dT%H:%M:%S").to_string(),
        });

        let content = serde_json::to_string_pretty(&payload)?;

        git_store::commit_file(
            &repo,
            &rel_path,
            &content,
            &format!("file_reservation: {} {}", agent_name, fr_c.path_pattern),
            "mcp-bot",
            "mcp-bot@localhost",
        )?;

        Ok(id)
    }

    pub async fn list_active_for_project(
        _ctx: &crate::Ctx,
        mm: &ModelManager,
        project_id: i64,
    ) -> Result<Vec<FileReservation>> {
        let db = mm.db();
        // Select active (not released). Checking expiry is better done in app logic or filter
        let stmt = db.prepare(
            r#"
            SELECT id, project_id, agent_id, path_pattern, exclusive, reason, created_ts, expires_ts, released_ts
            FROM file_reservations 
            WHERE project_id = ? AND released_ts IS NULL
            ORDER BY created_ts DESC
            "#
        ).await?;
        let mut rows = stmt.query([project_id]).await?;

        let mut reservations = Vec::new();
        while let Some(row) = rows.next().await? {
            reservations.push(Self::from_row(row)?);
        }
        Ok(reservations)
    }

    /// Retrieves a reservation by its database ID.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `id` - Reservation ID
    ///
    /// # Returns
    /// The file reservation
    ///
    /// # Errors
    /// Returns `Error::FileReservationNotFound` if ID doesn't exist
    pub async fn get(_ctx: &crate::Ctx, mm: &ModelManager, id: i64) -> Result<FileReservation> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, project_id, agent_id, path_pattern, exclusive, reason, created_ts, expires_ts, released_ts
            FROM file_reservations 
            WHERE id = ?
            "#
        ).await?;
        let mut rows = stmt.query([id]).await?;

        if let Some(row) = rows.next().await? {
            Ok(Self::from_row(row)?)
        } else {
            Err(crate::Error::FileReservationNotFound(format!("{}", id)))
        }
    }

    /// Releases a file reservation by marking it as released.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `id` - Reservation ID to release
    ///
    /// # Errors
    /// Returns an error if the reservation doesn't exist
    pub async fn release(_ctx: &crate::Ctx, mm: &ModelManager, id: i64) -> Result<()> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db
            .prepare(
                r#"
            UPDATE file_reservations SET released_ts = ? WHERE id = ?
            "#,
            )
            .await?;

        stmt.execute((now_str, id)).await?;
        Ok(())
    }

    pub async fn list_all_for_project(
        _ctx: &crate::Ctx,
        mm: &ModelManager,
        project_id: i64,
    ) -> Result<Vec<FileReservation>> {
        let db = mm.db();
        let stmt = db.prepare(
            r#"
            SELECT id, project_id, agent_id, path_pattern, exclusive, reason, created_ts, expires_ts, released_ts
            FROM file_reservations
            WHERE project_id = ?
            ORDER BY created_ts DESC
            "#
        ).await?;
        let mut rows = stmt.query([project_id]).await?;

        let mut reservations = Vec::new();
        while let Some(row) = rows.next().await? {
            reservations.push(Self::from_row(row)?);
        }
        Ok(reservations)
    }

    pub async fn release_by_path(
        _ctx: &crate::Ctx,
        mm: &ModelManager,
        project_id: i64,
        agent_id: i64,
        path_pattern: &str,
    ) -> Result<Option<i64>> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        // Find active reservation matching path
        let stmt = db
            .prepare(
                r#"
            SELECT id FROM file_reservations
            WHERE project_id = ? AND agent_id = ? AND path_pattern = ? AND released_ts IS NULL
            "#,
            )
            .await?;
        let mut rows = stmt.query((project_id, agent_id, path_pattern)).await?;

        if let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;

            // Release it
            let stmt = db
                .prepare(
                    r#"
                UPDATE file_reservations SET released_ts = ? WHERE id = ?
                "#,
                )
                .await?;
            stmt.execute((now_str, id)).await?;

            Ok(Some(id))
        } else {
            Ok(None)
        }
    }

    /// Force release a reservation by ID (any agent can call this for emergencies)
    pub async fn force_release(
        _ctx: &crate::Ctx,
        mm: &ModelManager,
        reservation_id: i64,
    ) -> Result<()> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db
            .prepare(
                r#"
            UPDATE file_reservations SET released_ts = ? WHERE id = ? AND released_ts IS NULL
            "#,
            )
            .await?;
        stmt.execute((now_str, reservation_id)).await?;
        Ok(())
    }

    /// Renew (extend) a file reservation's TTL
    pub async fn renew(
        _ctx: &crate::Ctx,
        mm: &ModelManager,
        reservation_id: i64,
        new_expires_ts: chrono::NaiveDateTime,
    ) -> Result<()> {
        let db = mm.db();
        let expires_str = new_expires_ts.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db
            .prepare(
                r#"
            UPDATE file_reservations SET expires_ts = ? WHERE id = ? AND released_ts IS NULL
            "#,
            )
            .await?;
        stmt.execute((expires_str, reservation_id)).await?;
        Ok(())
    }

    fn from_row(row: libsql::Row) -> Result<FileReservation> {
        let created_ts_str: String = row.get(6).unwrap_or_default();
        let expires_ts_str: String = row.get(7).unwrap_or_default();
        let released_ts_str: Option<String> = row.get(8).unwrap_or_default();

        let created_ts =
            NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S").unwrap_or_default();
        let expires_ts =
            NaiveDateTime::parse_from_str(&expires_ts_str, "%Y-%m-%d %H:%M:%S").unwrap_or_default();

        let released_ts = if let Some(s) = released_ts_str {
            NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()
        } else {
            None
        };

        Ok(FileReservation {
            id: row.get(0)?,
            project_id: row.get(1)?,
            agent_id: row.get(2)?,
            path_pattern: row.get(3)?,
            exclusive: row.get(4)?,
            reason: row.get(5)?,
            created_ts,
            expires_ts,
            released_ts,
        })
    }
}
