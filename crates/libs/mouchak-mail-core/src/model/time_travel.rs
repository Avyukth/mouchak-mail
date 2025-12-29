//! Time Travel - Historical Inbox Snapshots
//!
//! Provides functionality to retrieve inbox state at any point in time
//! using git history. This enables:
//! - Viewing what an agent's inbox looked like at a specific timestamp
//! - Debugging message delivery issues
//! - Auditing message flow historically
//!
//! # Example
//!
//! ```no_run
//! use mouchak_mail_core::model::time_travel::{TimeTravelBmc, parse_timestamp};
//! use chrono::{Utc, Duration};
//!
//! # async fn example() -> mouchak_mail_core::Result<()> {
//! # let mm = todo!();
//! # let ctx = todo!();
//! // Get inbox from 1 hour ago
//! let one_hour_ago = Utc::now() - Duration::hours(1);
//! let snapshot = TimeTravelBmc::inbox_at(&ctx, &mm, "my-project", "agent-1", one_hour_ago).await?;
//! # Ok(())
//! # }
//! ```

use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::store::git_store;
use crate::{Error, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A historical message snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMessage {
    pub id: i64,
    pub sender_name: String,
    pub recipient_names: Vec<String>,
    pub subject: String,
    pub body_md: String,
    pub created_ts: String,
    pub read_ts: Option<String>,
    pub thread_id: Option<String>,
    pub importance: String,
}

/// Result of a time travel query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTravelSnapshot {
    /// The project slug.
    pub project_slug: String,
    /// The agent name.
    pub agent_name: String,
    /// The requested timestamp (normalized to UTC).
    pub requested_at: DateTime<Utc>,
    /// The actual commit timestamp used.
    pub snapshot_at: DateTime<Utc>,
    /// Messages in the inbox at that time.
    pub messages: Vec<HistoricalMessage>,
    /// Whether this is an exact match or interpolated.
    pub is_exact: bool,
}

/// Parse a timestamp string into a UTC DateTime.
///
/// Supports multiple formats:
/// - RFC 3339: `2024-01-15T10:30:00Z`
/// - RFC 3339 with offset: `2024-01-15T10:30:00+05:30`
/// - ISO 8601: `2024-01-15T10:30:00`
/// - Date only: `2024-01-15`
/// - Unix epoch: `1705312200`
///
/// # Arguments
///
/// * `s` - The timestamp string to parse
///
/// # Returns
///
/// A UTC DateTime, or an error if parsing fails.
pub fn parse_timestamp(s: &str) -> Result<DateTime<Utc>> {
    let s = s.trim();

    // Try RFC 3339 with timezone
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try RFC 3339 with explicit offset patterns
    if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%:z") {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try ISO 8601 without timezone (assume UTC)
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&ndt));
    }

    // Try ISO 8601 with milliseconds
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(Utc.from_utc_datetime(&ndt));
    }

    // Try date only (midnight UTC)
    if let Ok(ndt) = NaiveDateTime::parse_from_str(&format!("{}T00:00:00", s), "%Y-%m-%dT%H:%M:%S")
    {
        return Ok(Utc.from_utc_datetime(&ndt));
    }

    // Try Unix epoch (seconds)
    if let Ok(epoch) = s.parse::<i64>() {
        if let Some(dt) = DateTime::from_timestamp(epoch, 0) {
            return Ok(dt);
        }
    }

    Err(Error::InvalidInput(format!(
        "Invalid timestamp format: {}",
        s
    )))
}

/// Time Travel Backend Model Controller.
pub struct TimeTravelBmc;

impl TimeTravelBmc {
    /// Get the inbox state for an agent at a specific point in time.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Request context
    /// * `mm` - Model manager
    /// * `project_slug` - Project identifier
    /// * `agent_name` - Agent name
    /// * `at_time` - The timestamp to query
    ///
    /// # Returns
    ///
    /// A snapshot of the inbox at the requested time.
    pub async fn inbox_at(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_slug: &str,
        agent_name: &str,
        at_time: DateTime<Utc>,
    ) -> Result<TimeTravelSnapshot> {
        // Validate inputs for XSS prevention
        Self::validate_slug(project_slug)?;
        Self::validate_agent_name(agent_name)?;

        let repo = mm.get_repo().await?;
        let repo_guard = repo.lock().await;

        // Find commit at or before the requested time
        let unix_time = at_time.timestamp();
        let commit_oid = git_store::find_commit_before(&repo_guard, unix_time)?
            .ok_or_else(|| Error::NotFound)?;

        let commit_time = git_store::get_commit_time(&repo_guard, commit_oid)?;
        let snapshot_at = DateTime::from_timestamp(commit_time, 0)
            .ok_or_else(|| Error::InvalidInput("Invalid commit timestamp".to_string()))?;

        // Build path to messages directory for this project
        let messages_dir = PathBuf::from(project_slug).join("messages");

        // Get all message files at that commit
        let files = match git_store::list_files_at_commit(&repo_guard, commit_oid, &messages_dir) {
            Ok(f) => f,
            Err(Error::Git2(_)) => vec![], // Directory doesn't exist = no messages
            Err(e) => return Err(e),
        };

        // Parse messages and filter by agent
        let mut messages = Vec::new();
        for (_filename, content) in files {
            if let Ok(msg) = serde_json::from_str::<HistoricalMessage>(&content) {
                // Check if agent is a recipient
                if msg.recipient_names.contains(&agent_name.to_string()) {
                    messages.push(msg);
                }
            }
        }

        // Sort by created_ts descending (newest first)
        messages.sort_by(|a, b| b.created_ts.cmp(&a.created_ts));

        Ok(TimeTravelSnapshot {
            project_slug: project_slug.to_string(),
            agent_name: agent_name.to_string(),
            requested_at: at_time,
            snapshot_at,
            messages,
            is_exact: commit_time == unix_time,
        })
    }

    /// Validate project slug for XSS prevention.
    fn validate_slug(slug: &str) -> Result<()> {
        if slug.is_empty() {
            return Err(Error::InvalidInput(
                "Project slug cannot be empty".to_string(),
            ));
        }

        // Check for dangerous characters
        if slug.contains('<')
            || slug.contains('>')
            || slug.contains('"')
            || slug.contains('\'')
            || slug.contains('&')
            || slug.contains('\0')
        {
            return Err(Error::InvalidInput(
                "Project slug contains invalid characters".to_string(),
            ));
        }

        // Check for path traversal
        if slug.contains("..") || slug.starts_with('/') || slug.starts_with('\\') {
            return Err(Error::InvalidInput(
                "Project slug contains path traversal".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate agent name for XSS prevention.
    fn validate_agent_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(Error::InvalidInput(
                "Agent name cannot be empty".to_string(),
            ));
        }

        // Check for dangerous characters
        if name.contains('<')
            || name.contains('>')
            || name.contains('"')
            || name.contains('\'')
            || name.contains('&')
            || name.contains('\0')
        {
            return Err(Error::InvalidInput(
                "Agent name contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }

    /// List all projects that have historical data.
    pub async fn list_projects(_ctx: &Ctx, mm: &ModelManager) -> Result<Vec<String>> {
        let repo = mm.get_repo().await?;
        let repo_guard = repo.lock().await;

        // Get HEAD commit
        let head = match repo_guard.head() {
            Ok(h) => h,
            Err(_) => return Ok(vec![]), // No commits yet
        };

        let tree = head.peel_to_tree()?;
        let mut projects = Vec::new();

        for entry in tree.iter() {
            if entry.kind() == Some(git2::ObjectType::Tree) {
                if let Some(name) = entry.name() {
                    // Each top-level directory is a project
                    projects.push(name.to_string());
                }
            }
        }

        projects.sort();
        Ok(projects)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_parse_timestamp_rfc3339() {
        let dt = parse_timestamp("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_parse_timestamp_with_offset() {
        let dt = parse_timestamp("2024-01-15T10:30:00+05:30").unwrap();
        // Should be converted to UTC
        assert_eq!(dt.hour(), 5); // 10:30 IST = 05:00 UTC
    }

    #[test]
    fn test_parse_timestamp_negative_offset() {
        let dt = parse_timestamp("2024-01-15T10:30:00-05:00").unwrap();
        assert_eq!(dt.hour(), 15); // 10:30 EST = 15:30 UTC
    }

    #[test]
    fn test_parse_timestamp_naive() {
        let dt = parse_timestamp("2024-01-15T10:30:00").unwrap();
        assert_eq!(dt.hour(), 10); // Assumed UTC
    }

    #[test]
    fn test_parse_timestamp_date_only() {
        let dt = parse_timestamp("2024-01-15").unwrap();
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
    }

    #[test]
    fn test_parse_timestamp_epoch() {
        // 2024-01-15 00:00:00 UTC
        let dt = parse_timestamp("1705276800").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        assert!(parse_timestamp("not-a-date").is_err());
        assert!(parse_timestamp("").is_err());
        assert!(parse_timestamp("2024-13-45").is_err());
    }

    #[test]
    fn test_validate_slug_valid() {
        assert!(TimeTravelBmc::validate_slug("my-project").is_ok());
        assert!(TimeTravelBmc::validate_slug("project_123").is_ok());
        assert!(TimeTravelBmc::validate_slug("Project.Name").is_ok());
    }

    #[test]
    fn test_validate_slug_xss() {
        assert!(TimeTravelBmc::validate_slug("<script>").is_err());
        assert!(TimeTravelBmc::validate_slug("project&name").is_err());
        assert!(TimeTravelBmc::validate_slug("\"quoted\"").is_err());
    }

    #[test]
    fn test_validate_slug_path_traversal() {
        assert!(TimeTravelBmc::validate_slug("../etc/passwd").is_err());
        assert!(TimeTravelBmc::validate_slug("/absolute/path").is_err());
    }

    #[test]
    fn test_validate_agent_name_valid() {
        assert!(TimeTravelBmc::validate_agent_name("agent-1").is_ok());
        assert!(TimeTravelBmc::validate_agent_name("Claude_Code").is_ok());
    }

    #[test]
    fn test_validate_agent_name_xss() {
        assert!(TimeTravelBmc::validate_agent_name("<script>alert(1)</script>").is_err());
        assert!(TimeTravelBmc::validate_agent_name("agent'--").is_err());
    }
}
