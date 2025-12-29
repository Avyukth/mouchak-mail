//! CI/CD build slot coordination for agents.
//!
//! This module provides exclusive lock management for CI/CD build slots,
//! preventing concurrent builds that could conflict. Slots use TTL-based
//! expiration with optional renewal.
//!
//! # Coordination Pattern
//!
//! 1. Agent acquires slot with TTL
//! 2. Agent periodically renews while building
//! 3. Agent releases slot when done (or slot expires)
//!
//! # Example
//!
//! ```no_run
//! use mouchak_mail_core::model::build_slot::{BuildSlotBmc, BuildSlotForCreate};
//! use mouchak_mail_core::model::ModelManager;
//! use mouchak_mail_core::ctx::Ctx;
//!
//! # async fn example() -> mouchak_mail_core::Result<()> {
//! let mm = ModelManager::new(std::sync::Arc::new(mouchak_mail_common::config::AppConfig::default())).await?;
//! let ctx = Ctx::root_ctx();
//!
//! // Acquire a build slot
//! let slot = BuildSlotForCreate {
//!     project_id: 1,
//!     agent_id: 1,
//!     slot_name: "ci-build".to_string(),
//!     ttl_seconds: 3600,
//! };
//! let slot_id = BuildSlotBmc::acquire(&ctx, &mm, slot).await?;
//!
//! // ... perform build ...
//!
//! // Release when done
//! BuildSlotBmc::release(&ctx, &mm, slot_id).await?;
//! # Ok(())
//! # }
//! ```

use crate::Result;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// An acquired build slot reservation.
///
/// Represents exclusive access to a named build slot for a specific agent.
/// Slots expire automatically after TTL unless renewed or released.
///
/// # Fields
///
/// - `id` - Database primary key
/// - `project_id` - Associated project
/// - `agent_id` - Agent holding the slot
/// - `slot_name` - Named slot (e.g., "ci-build", "deploy-prod")
/// - `created_ts` - When slot was acquired
/// - `expires_ts` - When slot will auto-expire
/// - `released_ts` - When slot was released (None if active)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSlot {
    /// Database primary key.
    pub id: i64,
    /// Associated project ID.
    pub project_id: i64,
    /// Agent holding the slot.
    pub agent_id: i64,
    /// Named slot identifier.
    pub slot_name: String,
    /// Acquisition timestamp.
    pub created_ts: NaiveDateTime,
    /// Expiration timestamp.
    pub expires_ts: NaiveDateTime,
    /// Release timestamp (None if active).
    pub released_ts: Option<NaiveDateTime>,
}

/// Input data for acquiring a build slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSlotForCreate {
    /// Project to acquire slot in.
    pub project_id: i64,
    /// Agent requesting the slot.
    pub agent_id: i64,
    /// Named slot to acquire.
    pub slot_name: String,
    /// Time-to-live in seconds.
    pub ttl_seconds: i64,
}

/// Backend Model Controller for Build Slot operations.
///
/// Manages CI/CD build slot coordination to prevent concurrent builds.
/// Uses TTL-based expiration with renewal support.
pub struct BuildSlotBmc;

impl BuildSlotBmc {
    /// Acquires a build slot for exclusive use.
    ///
    /// This method:
    /// 1. Checks if slot is currently held by another agent
    /// 2. Creates new slot reservation with TTL
    /// 3. Returns slot ID for renewal/release
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `slot_c` - Slot parameters (name, agent, TTL)
    ///
    /// # Returns
    /// The build slot ID
    ///
    /// # Errors
    /// Returns error if slot is already held by another agent
    ///
    /// # Example
    /// ```no_run
    /// # use mouchak_mail_core::model::build_slot::*;
    /// # use mouchak_mail_core::model::ModelManager;
    /// # use mouchak_mail_core::ctx::Ctx;
    /// # async fn example(mm: &ModelManager) {
    /// let ctx = Ctx::root_ctx();
    /// let slot = BuildSlotForCreate {
    ///     project_id: 1,
    ///     agent_id: 1,
    ///     slot_name: "ci-build".to_string(),
    ///     ttl_seconds: 3600, // 1 hour
    /// };
    /// let id = BuildSlotBmc::acquire(&ctx, mm, slot).await.unwrap();
    /// # }
    /// ```
    pub async fn acquire(_ctx: &Ctx, mm: &ModelManager, slot_c: BuildSlotForCreate) -> Result<i64> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let expires = now + chrono::Duration::seconds(slot_c.ttl_seconds);
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let expires_str = expires.format("%Y-%m-%d %H:%M:%S").to_string();

        // Check if slot is already held
        let stmt = db
            .prepare(
                r#"
            SELECT id FROM build_slots
            WHERE project_id = ? AND slot_name = ? AND released_ts IS NULL AND expires_ts > ?
            "#,
            )
            .await?;
        let mut rows = stmt
            .query((
                slot_c.project_id,
                slot_c.slot_name.as_str(),
                now_str.as_str(),
            ))
            .await?;

        if rows.next().await?.is_some() {
            return Err(crate::Error::InvalidInput("Build slot already held".into()));
        }

        let stmt = db
            .prepare(
                r#"
            INSERT INTO build_slots (project_id, agent_id, slot_name, expires_ts)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
            )
            .await?;

        let mut rows = stmt
            .query((
                slot_c.project_id,
                slot_c.agent_id,
                slot_c.slot_name.as_str(),
                expires_str.as_str(),
            ))
            .await?;

        let id = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            return Err(crate::Error::InvalidInput(
                "Failed to acquire build slot".into(),
            ));
        };

        Ok(id)
    }

    /// Renews (extends) a build slot's TTL.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `slot_id` - Slot to renew
    /// * `ttl_seconds` - New TTL duration
    ///
    /// # Returns
    /// New expiration timestamp
    ///
    /// # Errors
    /// Returns error if slot doesn't exist or is already released
    pub async fn renew(
        _ctx: &Ctx,
        mm: &ModelManager,
        slot_id: i64,
        ttl_seconds: i64,
    ) -> Result<NaiveDateTime> {
        let db = mm.db();
        let new_expires = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl_seconds);
        let expires_str = new_expires.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db
            .prepare(
                r#"
            UPDATE build_slots SET expires_ts = ? WHERE id = ? AND released_ts IS NULL
            "#,
            )
            .await?;
        stmt.execute((expires_str.as_str(), slot_id)).await?;
        Ok(new_expires)
    }

    /// Releases a build slot.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `slot_id` - Slot ID to release
    ///
    /// # Errors
    /// Returns error if slot doesn't exist
    pub async fn release(_ctx: &Ctx, mm: &ModelManager, slot_id: i64) -> Result<()> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db
            .prepare(
                r#"
            UPDATE build_slots SET released_ts = ? WHERE id = ? AND released_ts IS NULL
            "#,
            )
            .await?;
        stmt.execute((now_str.as_str(), slot_id)).await?;
        Ok(())
    }

    /// Lists all active (non-released, non-expired) build slots for a project.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `project_id` - Project database ID
    ///
    /// # Returns
    /// Vector of active build slots (may be empty)
    pub async fn list_active(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: i64,
    ) -> Result<Vec<BuildSlot>> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let stmt = db
            .prepare(
                r#"
            SELECT id, project_id, agent_id, slot_name, created_ts, expires_ts, released_ts
            FROM build_slots
            WHERE project_id = ? AND released_ts IS NULL AND expires_ts > ?
            ORDER BY created_ts DESC
            "#,
            )
            .await?;

        let mut rows = stmt.query((project_id, now_str.as_str())).await?;
        let mut slots = Vec::new();

        while let Some(row) = rows.next().await? {
            slots.push(Self::from_row(row)?);
        }
        Ok(slots)
    }

    fn from_row(row: libsql::Row) -> Result<BuildSlot> {
        let created_ts_str: String = row.get(4).unwrap_or_default();
        let expires_ts_str: String = row.get(5).unwrap_or_default();
        let released_ts_str: Option<String> = row.get(6).unwrap_or_default();

        let created_ts =
            NaiveDateTime::parse_from_str(&created_ts_str, "%Y-%m-%d %H:%M:%S").unwrap_or_default();
        let expires_ts =
            NaiveDateTime::parse_from_str(&expires_ts_str, "%Y-%m-%d %H:%M:%S").unwrap_or_default();
        let released_ts = released_ts_str
            .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok());

        Ok(BuildSlot {
            id: row.get(0)?,
            project_id: row.get(1)?,
            agent_id: row.get(2)?,
            slot_name: row.get(3)?,
            created_ts,
            expires_ts,
            released_ts,
        })
    }
}
