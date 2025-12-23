//! Unified Inbox HTTP handler
//!
//! Provides Gmail-style view of ALL messages across ALL projects.

use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use lib_core::Ctx;
use lib_core::model::message::{ImportanceFilter, MessageBmc};
use serde::{Deserialize, Serialize};

use crate::AppState;

/// Query parameters for unified inbox endpoint
#[derive(Debug, Deserialize)]
pub struct UnifiedInboxParams {
    /// Filter by importance: "high", "normal", or omit for all
    pub importance: Option<String>,
    /// Maximum messages to return (default: 50, max: 200)
    pub limit: Option<i32>,
}

/// Single message in unified inbox response
#[derive(Debug, Serialize)]
pub struct UnifiedInboxMessage {
    pub id: i64,
    pub project_id: i64,
    pub project_slug: String,
    pub sender_id: i64,
    pub sender_name: String,
    pub subject: String,
    pub importance: String,
    pub created_ts: chrono::NaiveDateTime,
    pub thread_id: Option<String>,
}

/// Response wrapper for unified inbox
#[derive(Debug, Serialize)]
pub struct UnifiedInboxResponse {
    pub messages: Vec<UnifiedInboxMessage>,
    pub total_count: usize,
}

/// GET /api/unified-inbox
///
/// Returns messages from all projects, optionally filtered by importance.
pub async fn unified_inbox_json(
    State(app_state): State<AppState>,
    Query(params): Query<UnifiedInboxParams>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let importance = ImportanceFilter::from_str_opt(params.importance.as_deref());
    let limit = params.limit.unwrap_or(50).clamp(1, 200);

    let items = MessageBmc::list_unified_inbox(&ctx, mm, importance, limit).await?;

    let messages: Vec<UnifiedInboxMessage> = items
        .into_iter()
        .map(|m| UnifiedInboxMessage {
            id: m.id,
            project_id: m.project_id,
            project_slug: m.project_slug,
            sender_id: m.sender_id,
            sender_name: m.sender_name,
            subject: m.subject,
            importance: m.importance,
            created_ts: m.created_ts,
            thread_id: m.thread_id,
        })
        .collect();

    let response = UnifiedInboxResponse {
        total_count: messages.len(),
        messages,
    };

    Ok(Json(response).into_response())
}
