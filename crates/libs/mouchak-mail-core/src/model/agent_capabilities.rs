use crate::Result;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::utils::{parse_timestamp, parse_timestamp_opt};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

// ============================================================================
// Capability Constants
// ============================================================================

/// Capability to send messages to other agents
pub const CAP_SEND_MESSAGE: &str = "send_message";

/// Capability to fetch/check inbox
pub const CAP_FETCH_INBOX: &str = "fetch_inbox";

/// Capability to reserve file paths
pub const CAP_FILE_RESERVATION: &str = "file_reservation_paths";

/// Capability to acknowledge messages
pub const CAP_ACKNOWLEDGE_MESSAGE: &str = "acknowledge_message";

/// Default capabilities granted to new agents
pub const DEFAULT_CAPABILITIES: &[&str] = &[
    CAP_SEND_MESSAGE,
    CAP_FETCH_INBOX,
    CAP_FILE_RESERVATION,
    CAP_ACKNOWLEDGE_MESSAGE,
];

/// A capability granted to an agent.
///
/// Capabilities control what actions an agent can perform in the system.
/// Most capabilities are granted at creation time.
///
/// # Fields
///
/// - `id` - Database primary key
/// - `agent_id` - Agent holding the capability
/// - `capability` - Capability string (e.g., "send_message")
/// - `granted_at` - When it was granted
/// - `granted_by` - Agent ID who granted it (None = system)
/// - `expires_at` - Optional expiration time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub id: i64,
    pub agent_id: i64,
    pub capability: String,
    pub granted_at: NaiveDateTime,
    pub granted_by: Option<i64>,
    pub expires_at: Option<NaiveDateTime>,
}

/// Input to grant a new capability.
///
/// # Fields
///
/// - `agent_id` - Target agent
/// - `capability` - Capability string
/// - `granted_by` - Granting agent ID (optional)
/// - `expires_at` - Expiration (optional)
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AgentCapabilityForCreate {
    pub agent_id: i64,
    pub capability: String,
    pub granted_by: Option<i64>,
    pub expires_at: Option<NaiveDateTime>,
}

pub struct AgentCapabilityBmc;

impl AgentCapabilityBmc {
    pub async fn create(
        _ctx: &Ctx,
        mm: &ModelManager,
        capability_c: AgentCapabilityForCreate,
    ) -> Result<i64> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let expires_at_str = capability_c
            .expires_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        let stmt = db.prepare(
            r#"
            INSERT INTO agent_capabilities (agent_id, capability, granted_at, granted_by, expires_at)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#
        ).await?;

        let mut rows = stmt
            .query((
                capability_c.agent_id,
                capability_c.capability,
                now_str,
                capability_c.granted_by,
                expires_at_str,
            ))
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(row.get(0)?)
        } else {
            Err(crate::Error::InvalidInput(
                "Failed to create agent capability".into(),
            ))
        }
    }

    /// List all non-expired capabilities for an agent
    pub async fn list_for_agent(
        _ctx: &Ctx,
        mm: &ModelManager,
        agent_id: i64,
    ) -> Result<Vec<AgentCapability>> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let stmt = db
            .prepare(
                r#"
            SELECT id, agent_id, capability, granted_at, granted_by, expires_at
            FROM agent_capabilities
            WHERE agent_id = ?
            AND (expires_at IS NULL OR expires_at > ?)
            ORDER BY capability ASC
            "#,
            )
            .await?;
        let mut rows = stmt.query((agent_id, now_str)).await?;

        let mut capabilities = Vec::new();
        while let Some(row) = rows.next().await? {
            let granted_at_str: String = row.get(3)?;
            let granted_at = parse_timestamp(&granted_at_str, "agent_capability.granted_at");

            let granted_by: Option<i64> = row.get(4)?;

            let expires_at: Option<String> = row.get(5)?;
            let expires_at = parse_timestamp_opt(expires_at, "agent_capability.expires_at");

            capabilities.push(AgentCapability {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                capability: row.get(2)?,
                granted_at,
                granted_by,
                expires_at,
            });
        }
        Ok(capabilities)
    }

    pub async fn revoke(_ctx: &Ctx, mm: &ModelManager, id: i64) -> Result<()> {
        let db = mm.db();
        let stmt = db
            .prepare("DELETE FROM agent_capabilities WHERE id = ?")
            .await?;
        stmt.execute([id]).await?;
        Ok(())
    }

    /// Check if an agent has a specific capability (non-expired)
    pub async fn check(
        _ctx: &Ctx,
        mm: &ModelManager,
        agent_id: i64,
        capability: &str,
    ) -> Result<bool> {
        let db = mm.db();
        let now = chrono::Utc::now().naive_utc();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let stmt = db
            .prepare(
                r#"SELECT COUNT(*) FROM agent_capabilities
                   WHERE agent_id = ? AND capability = ?
                   AND (expires_at IS NULL OR expires_at > ?)"#,
            )
            .await?;
        let mut rows = stmt.query((agent_id, capability, now_str)).await?;

        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    /// Grant default capabilities to a newly registered agent.
    ///
    /// This grants: send_message, fetch_inbox, file_reservation_paths, acknowledge_message
    ///
    /// # Arguments
    /// * `ctx` - Context
    /// * `mm` - Model manager
    /// * `agent_id` - The agent to grant capabilities to
    ///
    /// # Returns
    /// Number of capabilities granted
    pub async fn grant_defaults(ctx: &Ctx, mm: &ModelManager, agent_id: i64) -> Result<usize> {
        let mut granted = 0;
        for cap in DEFAULT_CAPABILITIES {
            let cap_c = AgentCapabilityForCreate {
                agent_id,
                capability: (*cap).to_string(),
                granted_by: None,
                expires_at: None,
            };
            Self::create(ctx, mm, cap_c).await?;
            granted += 1;
        }
        Ok(granted)
    }
}
