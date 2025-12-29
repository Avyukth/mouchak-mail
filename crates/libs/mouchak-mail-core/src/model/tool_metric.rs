//! MCP tool usage metrics and analytics.
//!
//! This module tracks every MCP tool invocation for performance monitoring,
//! debugging, and usage analytics. Metrics include timing, status, and error
//! information.
//!
//! # Use Cases
//!
//! - **Performance monitoring**: Track tool duration and identify slow tools
//! - **Error tracking**: Monitor error rates and error codes
//! - **Usage analytics**: See which tools are used most frequently
//! - **Debugging**: Review tool arguments for failed invocations
//!
//! # Example
//!
//! ```no_run
//! use mouchak_mail_core::model::tool_metric::{ToolMetricBmc, ToolMetricForCreate};
//! use mouchak_mail_core::model::ModelManager;
//! use mouchak_mail_core::ctx::Ctx;
//!
//! # async fn example() -> mouchak_mail_core::Result<()> {
//! let mm = ModelManager::new(std::sync::Arc::new(mouchak_mail_common::config::AppConfig::default())).await?;
//! let ctx = Ctx::root_ctx();
//!
//! // Record a tool invocation
//! let metric = ToolMetricForCreate {
//!     project_id: Some(1),
//!     agent_id: Some(1),
//!     tool_name: "send_message".to_string(),
//!     args_json: Some(r#"{"to": "reviewer"}"#.to_string()),
//!     status: "success".to_string(),
//!     error_code: None,
//!     duration_ms: 150,
//! };
//! let id = ToolMetricBmc::create(&ctx, &mm, metric).await?;
//! # Ok(())
//! # }
//! ```

use crate::Ctx;
use crate::Result;
use crate::model::ModelManager;
use serde::{Deserialize, Serialize};

/// A recorded MCP tool invocation metric.
///
/// Captures timing, status, and context for a single tool call.
///
/// # Fields
///
/// - `id` - Database primary key
/// - `project_id` - Associated project (optional for global tools)
/// - `agent_id` - Agent that invoked the tool (optional)
/// - `tool_name` - MCP tool name (e.g., "send_message")
/// - `args_json` - JSON-serialized tool arguments
/// - `status` - Execution status ("success" or "error")
/// - `error_code` - Error code if status is "error"
/// - `duration_ms` - Execution time in milliseconds
/// - `created_at` - Timestamp of invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetric {
    /// Database primary key.
    pub id: i64,
    /// Associated project ID (optional).
    pub project_id: Option<i64>,
    /// Agent that invoked the tool (optional).
    pub agent_id: Option<i64>,
    /// MCP tool name.
    pub tool_name: String,
    /// JSON-serialized arguments.
    pub args_json: Option<String>,
    /// Execution status.
    pub status: String,
    /// Error code (if failed).
    pub error_code: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: i64,
    /// Invocation timestamp.
    pub created_at: String,
}

/// Input data for creating a tool metric record.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolMetricForCreate {
    /// Associated project ID (optional).
    pub project_id: Option<i64>,
    /// Agent that invoked the tool (optional).
    pub agent_id: Option<i64>,
    /// MCP tool name.
    pub tool_name: String,
    /// JSON-serialized arguments.
    pub args_json: Option<String>,
    /// Execution status ("success" or "error").
    pub status: String,
    /// Error code (if failed).
    pub error_code: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: i64,
}

/// Backend Model Controller for Tool Metric operations.
///
/// Tracks MCP tool invocations for analytics, debugging, and performance
/// monitoring. Each tool call is recorded with timing and status.
pub struct ToolMetricBmc;

impl ToolMetricBmc {
    /// Records a new tool metric entry.
    ///
    /// Called automatically by the tool execution layer to track
    /// every MCP tool invocation.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `metric_c` - Metric data (tool name, duration, status)
    ///
    /// # Returns
    /// The metric record's database ID
    pub async fn create(
        _ctx: &Ctx,
        mm: &ModelManager,
        metric_c: ToolMetricForCreate,
    ) -> Result<i64> {
        let db = mm.db();
        let created_at = chrono::Utc::now().naive_utc().to_string();

        let stmt = db.prepare(
            r#"
            INSERT INTO tool_metrics (project_id, agent_id, tool_name, args_json, status, error_code, duration_ms, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        ).await?;

        let params: Vec<libsql::Value> = vec![
            metric_c.project_id.into(),
            metric_c.agent_id.into(),
            metric_c.tool_name.into(),
            metric_c.args_json.into(),
            metric_c.status.into(),
            metric_c.error_code.into(),
            metric_c.duration_ms.into(),
            created_at.into(),
        ];
        let mut rows = stmt.query(params).await?;

        let row = rows.next().await?.ok_or(crate::Error::NotFound)?;
        let id: i64 = row.get(0)?;
        Ok(id)
    }

    /// Lists recent tool metrics.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `project_id` - Optional project filter (None = all projects)
    /// * `limit` - Maximum number of records
    ///
    /// # Returns
    /// Vector of metrics (newest first)
    pub async fn list_recent(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: Option<i64>,
        limit: i64,
    ) -> Result<Vec<ToolMetric>> {
        let db = mm.db();

        let mut metrics = Vec::new();

        if let Some(pid) = project_id {
            let stmt = db.prepare("SELECT id, project_id, agent_id, tool_name, args_json, status, error_code, duration_ms, created_at FROM tool_metrics WHERE project_id = ? ORDER BY created_at DESC LIMIT ?").await?;
            let mut rows = stmt
                .query(vec![pid.into(), limit.into()] as Vec<libsql::Value>)
                .await?;
            while let Some(row) = rows.next().await? {
                metrics.push(Self::row_to_metric(&row)?);
            }
        } else {
            let stmt = db.prepare("SELECT id, project_id, agent_id, tool_name, args_json, status, error_code, duration_ms, created_at FROM tool_metrics ORDER BY created_at DESC LIMIT ?").await?;
            let mut rows = stmt.query(vec![limit.into()] as Vec<libsql::Value>).await?;
            while let Some(row) = rows.next().await? {
                metrics.push(Self::row_to_metric(&row)?);
            }
        }

        Ok(metrics)
    }

    fn row_to_metric(row: &libsql::Row) -> Result<ToolMetric> {
        Ok(ToolMetric {
            id: row.get(0)?,
            project_id: row.get(1)?,
            agent_id: row.get(2)?,
            tool_name: row.get(3)?,
            args_json: row.get(4)?,
            status: row.get(5)?,
            error_code: row.get(6)?,
            duration_ms: row.get(7)?,
            created_at: row.get(8)?,
        })
    }

    /// Gets aggregated statistics for tool usage.
    ///
    /// Provides per-tool counts, average duration, and error rates.
    ///
    /// # Arguments
    /// * `_ctx` - Request context
    /// * `mm` - ModelManager
    /// * `project_id` - Optional project filter
    ///
    /// # Returns
    /// Vector of per-tool statistics
    pub async fn get_stats(
        _ctx: &Ctx,
        mm: &ModelManager,
        project_id: Option<i64>,
    ) -> Result<Vec<ToolStat>> {
        let db = mm.db();

        let mut stats = Vec::new();

        if let Some(pid) = project_id {
            let sql = r#"
                SELECT 
                    tool_name, 
                    COUNT(*) as count, 
                    AVG(duration_ms) as avg_duration_ms,
                    SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as error_count
                FROM tool_metrics
                WHERE project_id = ?
                GROUP BY tool_name ORDER BY count DESC
            "#;
            let stmt = db.prepare(sql).await?;
            let mut rows = stmt.query(vec![pid.into()] as Vec<libsql::Value>).await?;
            while let Some(row) = rows.next().await? {
                stats.push(Self::row_to_stat(&row)?);
            }
        } else {
            let sql = r#"
                SELECT 
                    tool_name, 
                    COUNT(*) as count, 
                    AVG(duration_ms) as avg_duration_ms,
                    SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as error_count
                FROM tool_metrics
                GROUP BY tool_name ORDER BY count DESC
            "#;
            let stmt = db.prepare(sql).await?;
            let mut rows = stmt.query(()).await?;
            while let Some(row) = rows.next().await? {
                stats.push(Self::row_to_stat(&row)?);
            }
        }

        Ok(stats)
    }

    fn row_to_stat(row: &libsql::Row) -> Result<ToolStat> {
        Ok(ToolStat {
            tool_name: row.get(0)?,
            count: row.get(1)?,
            avg_duration_ms: row.get(2)?,
            error_count: row.get(3)?,
        })
    }
}

/// Aggregated statistics for a single tool.
///
/// Provides summary metrics including usage count, average duration,
/// and error rate for a specific tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolStat {
    /// Tool name.
    pub tool_name: String,
    /// Total invocation count.
    pub count: i64,
    /// Average execution duration in milliseconds.
    pub avg_duration_ms: f64,
    /// Count of failed invocations.
    pub error_count: i64,
}
