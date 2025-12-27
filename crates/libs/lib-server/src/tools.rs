use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
use lib_core::{self, Ctx};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::AppState;

// --- health_check ---
#[derive(Serialize)]
pub struct HealthCheckResponse {
    status: String,
    timestamp: String,
}

pub async fn health_check(_state: State<AppState>) -> crate::error::Result<Response> {
    Ok(Json(HealthCheckResponse {
        status: "ok".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    })
    .into_response())
}

// --- readiness_check ---
#[derive(Serialize)]
pub struct ReadinessResponse {
    status: &'static str,
    version: &'static str,
    uptime_seconds: u64,
    checks: ReadinessChecks,
}

#[derive(Serialize)]
pub struct ReadinessChecks {
    database: DatabaseCheckResult,
}

#[derive(Serialize)]
pub struct DatabaseCheckResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Readiness probe - checks if the service can handle requests
/// Returns 200 OK when ready, 503 Service Unavailable when not ready
pub async fn readiness_check(State(app_state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();

    // Check database connectivity
    let db_check = match app_state.mm.health_check().await {
        Ok(true) => DatabaseCheckResult {
            ok: true,
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Ok(false) => DatabaseCheckResult {
            ok: false,
            latency_ms: None,
            error: Some("Database query returned no rows".to_string()),
        },
        Err(e) => DatabaseCheckResult {
            ok: false,
            latency_ms: None,
            error: Some(e.to_string()),
        },
    };

    let all_ok = db_check.ok;
    let status = if all_ok { "ready" } else { "not_ready" };
    let http_status = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = ReadinessResponse {
        status,
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: app_state.start_time.elapsed().as_secs(),
        checks: ReadinessChecks { database: db_check },
    };

    (http_status, Json(response))
}

// --- ensure_project ---
#[derive(Deserialize)]
pub struct EnsureProjectPayload {
    /// Human-readable project name (e.g., "My Project")
    pub human_key: String,
}

#[derive(Serialize)]
pub struct EnsureProjectResponse {
    pub id: i64,
    pub slug: String,
    pub human_key: String,
}

pub async fn ensure_project(
    State(app_state): State<AppState>,
    Json(payload): Json<EnsureProjectPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        match lib_core::model::project::ProjectBmc::get_by_human_key(&ctx, mm, &payload.human_key)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                if let lib_core::Error::ProjectNotFound { .. } = e {
                    let mcp_config = lib_common::config::McpConfig::from_env();
                    let slug = lib_core::utils::compute_project_slug(
                        &payload.human_key,
                        mcp_config.project_identity_mode,
                        &mcp_config.project_identity_remote,
                    );
                    let _id = lib_core::model::project::ProjectBmc::create(
                        &ctx,
                        mm,
                        &slug,
                        &payload.human_key,
                    )
                    .await?;
                    lib_core::model::project::ProjectBmc::get_by_human_key(
                        &ctx,
                        mm,
                        &payload.human_key,
                    )
                    .await?
                } else {
                    return Err(e.into());
                }
            }
        };

    // Ensure built-in macros exist
    lib_core::model::macro_def::MacroDefBmc::ensure_builtin_macros(&ctx, mm, project.id.get())
        .await?;

    Ok(Json(EnsureProjectResponse {
        id: project.id.get(),
        slug: project.slug,
        human_key: project.human_key,
    })
    .into_response())
}

// --- register_agent ---
#[derive(Deserialize)]
pub struct RegisterAgentPayload {
    pub project_slug: String,
    /// Agent name
    pub name: String,
    pub program: String,
    pub model: String,
    #[serde(default)]
    pub task_description: Option<String>,
}

#[derive(Serialize)]
pub struct RegisterAgentResponse {
    pub id: i64,
    pub name: String,
    /// Project ID for e2e test compatibility
    pub project_id: i64,
    pub program: String,
    pub model: String,
    pub task_description: String,
    pub inception_ts: chrono::NaiveDateTime,
    pub last_active_ts: chrono::NaiveDateTime,
}

pub async fn register_agent(
    State(app_state): State<AppState>,
    Json(payload): Json<RegisterAgentPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;

    let agent_c = lib_core::model::agent::AgentForCreate {
        project_id: project.id,
        name: payload.name.clone(),
        program: payload.program.clone(),
        model: payload.model.clone(),
        task_description: payload.task_description.clone().unwrap_or_default(),
    };

    let agent_id = lib_core::model::agent::AgentBmc::create(&ctx, mm, agent_c).await?;

    // Fetch the full agent to return
    let agent = lib_core::model::agent::AgentBmc::get(&ctx, mm, agent_id).await?;

    Ok(Json(RegisterAgentResponse {
        id: agent.id.get(),
        name: agent.name,
        project_id: project.id.get(),
        program: agent.program,
        model: agent.model,
        task_description: agent.task_description,
        inception_ts: agent.inception_ts,
        last_active_ts: agent.last_active_ts,
    })
    .into_response())
}

// --- send_message ---
#[derive(Deserialize)]
pub struct SendMessagePayload {
    pub project_slug: String,
    // Support both naming conventions for compatibility
    #[serde(alias = "from_agent_name")]
    pub sender_name: String,
    #[serde(alias = "to_agent_names")]
    pub recipient_names: Vec<String>,
    /// CC recipients (optional)
    #[serde(default)]
    pub cc_names: Option<Vec<String>>,
    /// BCC recipients (optional)
    #[serde(default)]
    pub bcc_names: Option<Vec<String>>,
    pub subject: String,
    pub body_md: String,
    pub thread_id: Option<String>,
    pub importance: Option<String>,
    /// Whether recipients must acknowledge this message (default: false)
    #[serde(default)]
    pub ack_required: bool,
}

#[derive(Serialize)]
pub struct SendMessageResponse {
    pub id: i64,
    pub project_id: i64,
    pub sender_id: i64,
    pub sender_name: String,
    pub thread_id: Option<String>,
    pub subject: String,
    pub body_md: String,
    pub importance: String,
    pub ack_required: bool,
    pub created_ts: chrono::NaiveDateTime,
}

pub async fn send_message(
    State(app_state): State<AppState>,
    Json(payload): Json<SendMessagePayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let sender =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.sender_name)
            .await?;

    // Resolve "to" recipients
    let mut recipient_ids = Vec::new();
    for name in payload.recipient_names {
        let agent =
            lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &name).await?;
        recipient_ids.push(agent.id.get());
    }

    // Resolve "cc" recipients
    let cc_ids = if let Some(cc_names) = payload.cc_names {
        let mut ids = Vec::new();
        for name in cc_names {
            let agent =
                lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &name).await?;
            ids.push(agent.id.get());
        }
        Some(ids)
    } else {
        None
    };

    // Resolve "bcc" recipients
    let bcc_ids = if let Some(bcc_names) = payload.bcc_names {
        let mut ids = Vec::new();
        for name in bcc_names {
            let agent =
                lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &name).await?;
            ids.push(agent.id.get());
        }
        Some(ids)
    } else {
        None
    };

    let msg_c = lib_core::model::message::MessageForCreate {
        project_id: project.id.get(),
        sender_id: sender.id.get(),
        recipient_ids,
        cc_ids,
        bcc_ids,
        subject: payload.subject,
        body_md: payload.body_md,
        thread_id: payload.thread_id,
        importance: payload.importance,
        ack_required: payload.ack_required,
    };

    let message_id = lib_core::model::message::MessageBmc::create(&ctx, mm, msg_c).await?;

    // Fetch the full message to return
    let message = lib_core::model::message::MessageBmc::get(&ctx, mm, message_id).await?;

    Ok(Json(SendMessageResponse {
        id: message.id,
        project_id: message.project_id,
        sender_id: message.sender_id,
        sender_name: message.sender_name,
        thread_id: message.thread_id,
        subject: message.subject,
        body_md: message.body_md,
        importance: message.importance,
        ack_required: message.ack_required,
        created_ts: message.created_ts,
    })
    .into_response())
}

// --- list_inbox ---
#[derive(Deserialize)]
pub struct ListInboxPayload {
    pub project_slug: String,
    pub agent_name: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Serialize)]
pub struct InboxMessage {
    pub id: i64,
    pub subject: String,
    pub sender_name: String,
    pub created_ts: chrono::NaiveDateTime,
}

pub async fn list_inbox(
    State(app_state): State<AppState>,
    Json(payload): Json<ListInboxPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    let messages = lib_core::model::message::MessageBmc::list_inbox_for_agent(
        &ctx,
        mm,
        project.id.get(),
        agent.id.get(),
        payload.limit,
    )
    .await?;

    let inbox_msgs: Vec<InboxMessage> = messages
        .into_iter()
        .map(|msg| InboxMessage {
            id: msg.id,
            subject: msg.subject,
            sender_name: msg.sender_name,
            created_ts: msg.created_ts,
        })
        .collect();

    Ok(Json(inbox_msgs).into_response())
}

// --- list_outbox ---
#[derive(Deserialize)]
pub struct ListOutboxPayload {
    pub project_slug: String,
    pub agent_name: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

pub async fn list_outbox(
    State(app_state): State<AppState>,
    Json(payload): Json<ListOutboxPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    let messages = lib_core::model::message::MessageBmc::list_outbox_for_agent(
        &ctx,
        mm,
        project.id.get(),
        agent.id.get(),
        payload.limit,
    )
    .await?;

    let outbox_msgs: Vec<InboxMessage> = messages
        .into_iter()
        .map(|msg| InboxMessage {
            id: msg.id,
            subject: msg.subject,
            sender_name: msg.sender_name,
            created_ts: msg.created_ts,
        })
        .collect();

    Ok(Json(outbox_msgs).into_response())
}

// --- list_all_projects ---
#[derive(Serialize)]
pub struct ProjectResponse {
    pub id: i64,
    pub slug: String,
    pub human_key: String,
    pub created_at: chrono::NaiveDateTime,
}

pub async fn list_all_projects(
    State(app_state): State<AppState>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let projects = lib_core::model::project::ProjectBmc::list_all(&ctx, mm).await?;

    let project_responses: Vec<ProjectResponse> = projects
        .into_iter()
        .map(|p| ProjectResponse {
            id: p.id.get(),
            slug: p.slug,
            human_key: p.human_key,
            created_at: p.created_at,
        })
        .collect();

    Ok(Json(project_responses).into_response())
}

// --- delete_project ---
#[derive(Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

pub async fn delete_project(
    State(app_state): State<AppState>,
    Path(project_slug): Path<String>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &project_slug).await?;

    lib_core::model::project::ProjectBmc::delete(&ctx, mm, project.id).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: format!("Project '{}' deleted successfully", project_slug),
    })
    .into_response())
}

// --- delete_agent ---
pub async fn delete_agent(
    State(app_state): State<AppState>,
    Path((project_slug, agent_name)): Path<(String, String)>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &project_slug).await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &agent_name).await?;

    lib_core::model::agent::AgentBmc::delete(&ctx, mm, agent.id).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: format!("Agent '{}' deleted successfully", agent_name),
    })
    .into_response())
}

// --- list_all_agents_for_project ---
// Keep for backwards compatibility with JSON body requests
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ListAgentsPayload {
    pub project_slug: String,
}

#[derive(Serialize)]
pub struct AgentResponse {
    pub id: i64,
    pub name: String,
    pub program: String,
    pub model: String,
    pub task_description: String,
    pub inception_ts: chrono::NaiveDateTime,
    pub last_active_ts: chrono::NaiveDateTime,
}

pub async fn list_all_agents_for_project(
    State(app_state): State<AppState>,
    Path(project_slug): Path<String>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &project_slug).await?;
    let agents =
        lib_core::model::agent::AgentBmc::list_all_for_project(&ctx, mm, project.id).await?;

    let agent_responses: Vec<AgentResponse> = agents
        .into_iter()
        .map(|a| AgentResponse {
            id: a.id.get(),
            name: a.name,
            program: a.program,
            model: a.model,
            task_description: a.task_description,
            inception_ts: a.inception_ts,
            last_active_ts: a.last_active_ts,
        })
        .collect();

    Ok(Json(agent_responses).into_response())
}

// --- get_message ---
// Keep for backwards compatibility
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct GetMessagePayload {
    pub message_id: i64,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub id: i64,
    pub project_id: i64,
    pub sender_id: i64,
    pub sender_name: String,
    pub thread_id: Option<String>,
    pub subject: String,
    pub body_md: String,
    pub importance: String,
    pub ack_required: bool,
    pub created_ts: chrono::NaiveDateTime,
    pub attachments: Vec<serde_json::Value>,
    pub recipients: Vec<String>,
}

pub async fn get_message(
    State(app_state): State<AppState>,
    Path(message_id): Path<i64>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let message = lib_core::model::message::MessageBmc::get(&ctx, mm, message_id).await?;
    let recipients = lib_core::model::message::MessageBmc::get_recipients(&ctx, mm, message_id)
        .await
        .unwrap_or_default();

    Ok(Json(MessageResponse {
        id: message.id,
        project_id: message.project_id,
        sender_id: message.sender_id,
        sender_name: message.sender_name,
        thread_id: message.thread_id,
        subject: message.subject,
        body_md: message.body_md,
        importance: message.importance,
        ack_required: message.ack_required,
        created_ts: message.created_ts,
        attachments: message.attachments,
        recipients,
    })
    .into_response())
}

// --- file_reservation_paths ---
#[derive(Deserialize)]
pub struct FileReservationPathsPayload {
    pub project_slug: String,
    pub agent_name: String,
    pub paths: Vec<String>,
    #[serde(default = "default_exclusive")]
    pub exclusive: bool,
    pub reason: Option<String>,
    pub ttl_seconds: Option<i64>,
}

fn default_exclusive() -> bool {
    true
}

#[derive(Serialize)]
pub struct FileReservationGranted {
    pub id: i64,
    pub path_pattern: String,
    pub exclusive: bool,
    pub reason: String,
    pub expires_ts: String,
}

#[derive(Serialize)]
pub struct FileReservationConflict {
    pub path_pattern: String,
    pub exclusive: bool,
    pub expires_ts: String,
    pub conflict_type: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct FileReservationPathsResponse {
    pub granted: Vec<FileReservationGranted>,
    pub conflicts: Vec<FileReservationConflict>,
}

fn find_path_conflicts(
    path: &str,
    agent_id: i64,
    request_exclusive: bool,
    active_reservations: &[lib_core::model::file_reservation::FileReservation],
) -> Vec<FileReservationConflict> {
    active_reservations
        .iter()
        .filter(|res| {
            res.agent_id.get() != agent_id
                && (res.exclusive || request_exclusive)
                && res.path_pattern == path
        })
        .map(|res| FileReservationConflict {
            path_pattern: res.path_pattern.clone(),
            exclusive: res.exclusive,
            expires_ts: res.expires_ts.format("%Y-%m-%dT%H:%M:%S").to_string(),
            conflict_type: "FILE_RESERVATION_CONFLICT".to_string(),
            message: format!(
                "Conflict with reservation held by agent ID {}",
                res.agent_id.get()
            ),
        })
        .collect()
}

pub async fn file_reservation_paths(
    State(app_state): State<AppState>,
    Json(payload): Json<FileReservationPathsPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    let active_reservations =
        FileReservationBmc::list_active_for_project(&ctx, mm, project.id).await?;

    let ttl = payload.ttl_seconds.unwrap_or(3600);
    let now = chrono::Utc::now().naive_utc();
    let expires_ts = now + chrono::Duration::seconds(ttl);

    let mut granted = Vec::new();
    let mut conflicts = Vec::new();

    for path in payload.paths {
        conflicts.extend(find_path_conflicts(
            &path,
            agent.id.get(),
            payload.exclusive,
            &active_reservations,
        ));

        let fr_c = FileReservationForCreate {
            project_id: project.id,
            agent_id: agent.id,
            path_pattern: path.clone(),
            exclusive: payload.exclusive,
            reason: payload.reason.clone().unwrap_or_default(),
            expires_ts,
        };

        let id = FileReservationBmc::create(&ctx, mm, fr_c).await?;

        granted.push(FileReservationGranted {
            id,
            path_pattern: path,
            exclusive: payload.exclusive,
            reason: payload.reason.clone().unwrap_or_default(),
            expires_ts: expires_ts.format("%Y-%m-%dT%H:%M:%S").to_string(),
        });
    }

    Ok(Json(FileReservationPathsResponse { granted, conflicts }).into_response())
}

// --- create_agent_identity ---
const ADJECTIVES: &[&str] = &[
    "Blue", "Green", "Red", "Golden", "Silver", "Crystal", "Dark", "Bright", "Swift", "Calm",
    "Bold", "Wise", "Noble", "Grand", "Mystic", "Ancient", "Lunar", "Solar", "Azure", "Coral",
    "Amber", "Jade", "Onyx", "Pearl", "Scarlet", "Violet", "Copper", "Bronze", "Iron", "Steel",
    "Frost", "Storm",
];

const NOUNS: &[&str] = &[
    "Mountain", "Castle", "River", "Forest", "Valley", "Harbor", "Tower", "Bridge", "Falcon",
    "Phoenix", "Dragon", "Wolf", "Eagle", "Lion", "Hawk", "Owl", "Oak", "Pine", "Willow", "Cedar",
    "Maple", "Birch", "Ash", "Elm", "Stone", "Crystal", "Star", "Moon", "Sun", "Cloud", "Wind",
    "Thunder",
];

fn generate_random_names(
    existing: &std::collections::HashSet<String>,
    count: usize,
) -> Vec<String> {
    let mut names = Vec::new();
    let mut rng_seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;

    let mut next_rand = || {
        rng_seed = rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
        rng_seed
    };

    for _ in 0..count * 2 {
        let adj_idx = next_rand() % ADJECTIVES.len();
        let noun_idx = next_rand() % NOUNS.len();
        let name = format!("{}{}", ADJECTIVES[adj_idx], NOUNS[noun_idx]);

        if !existing.contains(&name) && !names.contains(&name) {
            names.push(name);
            if names.len() >= count {
                break;
            }
        }
    }
    names
}

fn find_hint_match(
    hint: &str,
    existing: &std::collections::HashSet<String>,
    alternatives: &[String],
) -> Option<String> {
    let hint_lower = hint.to_lowercase();
    for adj in ADJECTIVES {
        if !adj.to_lowercase().contains(&hint_lower) {
            continue;
        }
        for noun in NOUNS {
            let name = format!("{adj}{noun}");
            if !existing.contains(&name) && !alternatives.contains(&name) {
                return Some(name);
            }
        }
    }
    None
}

#[derive(Deserialize)]
pub struct CreateAgentIdentityPayload {
    pub project_slug: String,
    #[serde(default)]
    pub hint: Option<String>,
}

#[derive(Serialize)]
pub struct CreateAgentIdentityResponse {
    pub suggested_name: String,
    pub alternatives: Vec<String>,
}

pub async fn create_agent_identity(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateAgentIdentityPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;

    let existing_agents =
        lib_core::model::agent::AgentBmc::list_all_for_project(&ctx, mm, project.id).await?;
    let existing_names: std::collections::HashSet<String> =
        existing_agents.iter().map(|a| a.name.clone()).collect();

    let mut alternatives = generate_random_names(&existing_names, 5);

    if let Some(hint) = &payload.hint {
        if let Some(hint_name) = find_hint_match(hint, &existing_names, &alternatives) {
            alternatives.insert(0, hint_name);
        }
    }

    let suggested_name = alternatives
        .first()
        .cloned()
        .unwrap_or_else(|| "Agent1".to_string());

    Ok(Json(CreateAgentIdentityResponse {
        suggested_name,
        alternatives,
    })
    .into_response())
}

// --- whois ---
#[derive(Deserialize)]
pub struct WhoisPayload {
    pub project_slug: String,
    pub agent_name: String,
}

#[derive(Serialize)]
pub struct WhoisResponse {
    pub id: i64,
    pub name: String,
    pub program: String,
    pub model: String,
    pub task_description: String,
    pub inception_ts: chrono::NaiveDateTime,
    pub last_active_ts: chrono::NaiveDateTime,
    pub attachments_policy: String,
    pub contact_policy: String,
    pub project_slug: String,
    pub project_human_key: String,
}

pub async fn whois(
    State(app_state): State<AppState>,
    Json(payload): Json<WhoisPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    Ok(Json(WhoisResponse {
        id: agent.id.get(),
        name: agent.name,
        program: agent.program,
        model: agent.model,
        task_description: agent.task_description,
        inception_ts: agent.inception_ts,
        last_active_ts: agent.last_active_ts,
        attachments_policy: agent.attachments_policy,
        contact_policy: agent.contact_policy,
        project_slug: project.slug,
        project_human_key: project.human_key,
    })
    .into_response())
}

// --- list_file_reservations ---
#[derive(Deserialize)]
pub struct ListFileReservationsPayload {
    pub project_slug: String,
    #[serde(default)]
    pub agent_name: Option<String>,
    #[serde(default)]
    pub active_only: Option<bool>,
}

#[derive(Serialize)]
pub struct FileReservationResponse {
    pub id: i64,
    pub agent_id: i64,
    pub agent_name: String,
    pub path_pattern: String,
    pub exclusive: bool,
    pub reason: String,
    pub created_ts: String,
    pub expires_ts: String,
    pub is_active: bool,
}

pub async fn list_file_reservations(
    State(app_state): State<AppState>,
    Json(payload): Json<ListFileReservationsPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;

    let reservations = if payload.active_only.unwrap_or(true) {
        FileReservationBmc::list_active_for_project(&ctx, mm, project.id).await?
    } else {
        FileReservationBmc::list_all_for_project(&ctx, mm, project.id.get()).await?
    };

    let now = chrono::Utc::now().naive_utc();
    let mut responses = Vec::new();

    for res in reservations {
        // Filter by agent if specified
        if let Some(ref agent_name) = payload.agent_name {
            let agent = lib_core::model::agent::AgentBmc::get(&ctx, mm, res.agent_id).await?;
            if &agent.name != agent_name {
                continue;
            }
        }

        // Get agent name for response
        let agent = lib_core::model::agent::AgentBmc::get(&ctx, mm, res.agent_id).await?;

        responses.push(FileReservationResponse {
            id: res.id,
            agent_id: res.agent_id.get(),
            agent_name: agent.name,
            path_pattern: res.path_pattern,
            exclusive: res.exclusive,
            reason: res.reason,
            created_ts: res.created_ts.format("%Y-%m-%dT%H:%M:%S").to_string(),
            expires_ts: res.expires_ts.format("%Y-%m-%dT%H:%M:%S").to_string(),
            is_active: res.expires_ts > now,
        });
    }

    Ok(Json(responses).into_response())
}

// --- list_all_locks ---
// Returns all active file reservations across all projects (for web UI dashboard)
#[derive(Serialize)]
pub struct LockResponse {
    pub id: i64,
    pub project_id: i64,
    pub project_slug: String,
    pub agent_id: i64,
    pub agent_name: String,
    pub path_pattern: String,
    pub exclusive: bool,
    pub reason: String,
    pub created_ts: String,
    pub expires_ts: String,
    pub is_expired: bool,
}

pub async fn list_all_locks(State(app_state): State<AppState>) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let reservations = FileReservationBmc::list_all_active(&ctx, mm).await?;
    let now = chrono::Utc::now().naive_utc();
    let mut responses = Vec::new();

    for res in reservations {
        // Get project slug
        let project = lib_core::model::project::ProjectBmc::get(&ctx, mm, res.project_id)
            .await
            .ok();
        let project_slug = project
            .map(|p| p.slug)
            .unwrap_or_else(|| "unknown".to_string());

        // Get agent name
        let agent = lib_core::model::agent::AgentBmc::get(&ctx, mm, res.agent_id)
            .await
            .ok();
        let agent_name = agent
            .map(|a| a.name)
            .unwrap_or_else(|| "unknown".to_string());

        responses.push(LockResponse {
            id: res.id,
            project_id: res.project_id.get(),
            project_slug,
            agent_id: res.agent_id.get(),
            agent_name,
            path_pattern: res.path_pattern,
            exclusive: res.exclusive,
            reason: res.reason,
            created_ts: res.created_ts.format("%Y-%m-%dT%H:%M:%S").to_string(),
            expires_ts: res.expires_ts.format("%Y-%m-%dT%H:%M:%S").to_string(),
            is_expired: res.expires_ts < now,
        });
    }

    Ok(Json(responses).into_response())
}

// --- release_file_reservation ---
#[derive(Deserialize)]
pub struct ReleaseFileReservationPayload {
    pub project_slug: String,
    pub agent_name: String,
    pub paths: Vec<String>,
}

#[derive(Serialize)]
pub struct ReleaseFileReservationResponse {
    pub released_count: usize,
    pub released_ids: Vec<i64>,
}

pub async fn release_file_reservation(
    State(app_state): State<AppState>,
    Json(payload): Json<ReleaseFileReservationPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    let mut released_ids = Vec::new();

    for path in &payload.paths {
        if let Some(id) =
            FileReservationBmc::release_by_path(&ctx, mm, project.id.get(), agent.id.get(), path)
                .await?
        {
            released_ids.push(id);
        }
    }

    Ok(Json(ReleaseFileReservationResponse {
        released_count: released_ids.len(),
        released_ids,
    })
    .into_response())
}

// --- get_thread ---
#[derive(Deserialize)]
pub struct GetThreadPayload {
    pub project_slug: String,
    pub thread_id: String,
}

pub async fn get_thread(
    State(app_state): State<AppState>,
    Json(payload): Json<GetThreadPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let messages = lib_core::model::message::MessageBmc::list_by_thread(
        &ctx,
        mm,
        project.id.get(),
        &payload.thread_id,
    )
    .await?;

    let mut responses: Vec<MessageResponse> = Vec::with_capacity(messages.len());
    for msg in messages {
        let recipients =
            lib_core::model::message::MessageBmc::get_recipients(&ctx, mm, msg.id).await?;
        responses.push(MessageResponse {
            id: msg.id,
            project_id: msg.project_id,
            sender_id: msg.sender_id,
            sender_name: msg.sender_name,
            thread_id: msg.thread_id,
            subject: msg.subject,
            body_md: msg.body_md,
            importance: msg.importance,
            ack_required: msg.ack_required,
            created_ts: msg.created_ts,
            attachments: msg.attachments,
            recipients,
        });
    }

    Ok(Json(responses).into_response())
}

// --- reply_message ---
#[derive(Deserialize)]
pub struct ReplyMessagePayload {
    pub project_slug: String,
    pub sender_name: String,
    pub message_id: i64,
    pub body_md: String,
    pub importance: Option<String>,
}

pub async fn reply_message(
    State(app_state): State<AppState>,
    Json(payload): Json<ReplyMessagePayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let sender =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.sender_name)
            .await?;

    // Get original message to extract thread_id and original sender as recipient
    let original_msg =
        lib_core::model::message::MessageBmc::get(&ctx, mm, payload.message_id).await?;

    // Reply goes to the original sender
    let recipient_ids = vec![original_msg.sender_id];

    // Use existing thread_id or create reference to original
    let thread_id = original_msg.thread_id.clone();

    // Create subject with Re: prefix if not already present
    let subject = if original_msg.subject.starts_with("Re: ") {
        original_msg.subject.clone()
    } else {
        format!("Re: {}", original_msg.subject)
    };

    let msg_c = lib_core::model::message::MessageForCreate {
        project_id: project.id.get(),
        sender_id: sender.id.get(),
        recipient_ids,
        cc_ids: None,
        bcc_ids: None,
        subject,
        body_md: payload.body_md,
        thread_id,
        importance: payload.importance,
        ack_required: false, // Replies don't require ack by default
    };

    let message_id = lib_core::model::message::MessageBmc::create(&ctx, mm, msg_c).await?;

    // Fetch the full message to return
    let message = lib_core::model::message::MessageBmc::get(&ctx, mm, message_id).await?;

    Ok(Json(SendMessageResponse {
        id: message.id,
        project_id: message.project_id,
        sender_id: message.sender_id,
        sender_name: message.sender_name,
        thread_id: message.thread_id,
        subject: message.subject,
        body_md: message.body_md,
        importance: message.importance,
        ack_required: message.ack_required,
        created_ts: message.created_ts,
    })
    .into_response())
}

// --- search_messages ---
#[derive(Deserialize)]
pub struct SearchMessagesPayload {
    pub project_slug: String,
    pub query: String,
    #[serde(default = "default_search_limit")]
    pub limit: i64,
}

fn default_search_limit() -> i64 {
    50
}

#[derive(Serialize)]
pub struct SearchMessageResult {
    pub id: i64,
    pub subject: String,
    pub sender_name: String,
    pub thread_id: Option<String>,
    pub body_md: String,
    pub importance: String,
    pub created_ts: chrono::NaiveDateTime,
}

#[derive(Serialize)]
pub struct SearchMessagesResponse {
    pub query: String,
    pub results: Vec<SearchMessageResult>,
    pub count: usize,
}

pub async fn search_messages(
    State(app_state): State<AppState>,
    Json(payload): Json<SearchMessagesPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;

    let messages = lib_core::model::message::MessageBmc::search(
        &ctx,
        mm,
        project.id.get(),
        &payload.query,
        payload.limit,
    )
    .await?;

    let results: Vec<SearchMessageResult> = messages
        .into_iter()
        .map(|msg| SearchMessageResult {
            id: msg.id,
            subject: msg.subject,
            sender_name: msg.sender_name,
            thread_id: msg.thread_id,
            body_md: msg.body_md,
            importance: msg.importance,
            created_ts: msg.created_ts,
        })
        .collect();

    let count = results.len();

    Ok(Json(SearchMessagesResponse {
        query: payload.query,
        results,
        count,
    })
    .into_response())
}

// --- force_release_reservation ---
#[derive(Deserialize)]
pub struct ForceReleaseReservationPayload {
    pub reservation_id: i64,
}

#[derive(Serialize)]
pub struct ForceReleaseReservationResponse {
    pub released: bool,
    pub reservation_id: i64,
}

pub async fn force_release_reservation(
    State(app_state): State<AppState>,
    Json(payload): Json<ForceReleaseReservationPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    FileReservationBmc::force_release(&ctx, mm, payload.reservation_id).await?;

    Ok(Json(ForceReleaseReservationResponse {
        released: true,
        reservation_id: payload.reservation_id,
    })
    .into_response())
}

// --- renew_file_reservation ---
#[derive(Deserialize)]
pub struct RenewFileReservationPayload {
    pub reservation_id: i64,
    pub ttl_seconds: Option<i64>,
}

#[derive(Serialize)]
pub struct RenewFileReservationResponse {
    pub renewed: bool,
    pub reservation_id: i64,
    pub new_expires_ts: String,
}

pub async fn renew_file_reservation(
    State(app_state): State<AppState>,
    Json(payload): Json<RenewFileReservationPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let ttl = payload.ttl_seconds.unwrap_or(3600);
    let new_expires = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(ttl);

    FileReservationBmc::renew(&ctx, mm, payload.reservation_id, new_expires).await?;

    Ok(Json(RenewFileReservationResponse {
        renewed: true,
        reservation_id: payload.reservation_id,
        new_expires_ts: new_expires.format("%Y-%m-%dT%H:%M:%S").to_string(),
    })
    .into_response())
}

// --- get_project_info ---
#[derive(Deserialize)]
pub struct GetProjectInfoPayload {
    pub project_slug: String,
}

#[derive(Serialize)]
pub struct ProjectInfoResponse {
    pub id: i64,
    pub slug: String,
    pub human_key: String,
    pub created_at: chrono::NaiveDateTime,
    pub agent_count: usize,
    pub message_count: usize,
}

pub async fn get_project_info(
    State(app_state): State<AppState>,
    Json(payload): Json<GetProjectInfoPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;

    // Count agents
    let agents =
        lib_core::model::agent::AgentBmc::list_all_for_project(&ctx, mm, project.id).await?;
    let agent_count = agents.len();

    // Count messages
    let message_count =
        lib_core::model::project::ProjectBmc::count_messages(&ctx, mm, project.id).await?;

    Ok(Json(ProjectInfoResponse {
        id: project.id.get(),
        slug: project.slug,
        human_key: project.human_key,
        created_at: project.created_at,
        agent_count,
        message_count: message_count as usize,
    })
    .into_response())
}

#[derive(Deserialize)]
pub struct GetQuotaStatusPayload {
    pub project_slug: String,
    pub agent_name: Option<String>,
}

#[derive(Serialize)]
pub struct QuotaStatusResponse {
    pub project_slug: String,
    pub quota_enabled: bool,
    pub attachments_limit_bytes: i64,
    pub attachments_usage_bytes: i64,
    pub inbox_limit_count: i64,
    pub agent_inbox_usage: Option<i64>,
}

pub async fn get_quota_status(
    State(app_state): State<AppState>,
    Json(payload): Json<GetQuotaStatusPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;
    let config = &mm.app_config.quota;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;

    let attachments_usage = lib_core::model::attachment::AttachmentBmc::get_total_project_usage(
        &ctx,
        mm,
        project.id.get(),
    )
    .await?;

    let mut agent_usage = None;
    if let Some(agent_name) = &payload.agent_name {
        let agent =
            lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, agent_name).await?;
        let count =
            lib_core::model::message::MessageBmc::get_inbox_count(&ctx, mm, agent.id.get()).await?;
        agent_usage = Some(count);
    }

    Ok(Json(QuotaStatusResponse {
        project_slug: project.slug,
        quota_enabled: config.enabled,
        attachments_limit_bytes: config.attachments_limit_bytes as i64,
        attachments_usage_bytes: attachments_usage,
        inbox_limit_count: config.inbox_limit_count as i64,
        agent_inbox_usage: agent_usage,
    })
    .into_response())
}

// --- get_agent_profile ---
// Extended profile info compared to basic whois
#[derive(Serialize)]
pub struct AgentProfileResponse {
    pub id: i64,
    pub name: String,
    pub program: String,
    pub model: String,
    pub task_description: String,
    pub inception_ts: chrono::NaiveDateTime,
    pub last_active_ts: chrono::NaiveDateTime,
    pub attachments_policy: String,
    pub contact_policy: String,
    pub project_slug: String,
    pub project_human_key: String,
    pub message_count_sent: usize,
    pub message_count_received: usize,
    pub active_reservations: usize,
}

pub async fn get_agent_profile(
    State(app_state): State<AppState>,
    Json(payload): Json<WhoisPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    // Count sent and received messages
    let sent_count =
        lib_core::model::agent::AgentBmc::count_messages_sent(&ctx, mm, agent.id).await?;
    let received_count =
        lib_core::model::agent::AgentBmc::count_messages_received(&ctx, mm, agent.id).await?;

    // Count active reservations
    let reservations = FileReservationBmc::list_active_for_project(&ctx, mm, project.id).await?;
    let active_reservations = reservations
        .iter()
        .filter(|r| r.agent_id == agent.id)
        .count();

    Ok(Json(AgentProfileResponse {
        id: agent.id.get(),
        name: agent.name,
        program: agent.program,
        model: agent.model,
        task_description: agent.task_description,
        inception_ts: agent.inception_ts,
        last_active_ts: agent.last_active_ts,
        attachments_policy: agent.attachments_policy,
        contact_policy: agent.contact_policy,
        project_slug: project.slug,
        project_human_key: project.human_key,
        message_count_sent: sent_count as usize,
        message_count_received: received_count as usize,
        active_reservations,
    })
    .into_response())
}

// --- mark_message_read ---
#[derive(Deserialize)]
pub struct MarkMessageReadPayload {
    pub project_slug: String,
    pub agent_name: String,
    pub message_id: i64,
}

#[derive(Serialize)]
pub struct MarkMessageReadResponse {
    pub marked: bool,
    pub message_id: i64,
}

pub async fn mark_message_read(
    State(app_state): State<AppState>,
    Json(payload): Json<MarkMessageReadPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    lib_core::model::message::MessageBmc::mark_read(&ctx, mm, payload.message_id, agent.id.get())
        .await?;

    Ok(Json(MarkMessageReadResponse {
        marked: true,
        message_id: payload.message_id,
    })
    .into_response())
}

// --- acknowledge_message ---
#[derive(Deserialize)]
pub struct AcknowledgeMessagePayload {
    pub project_slug: String,
    pub agent_name: String,
    pub message_id: i64,
}

#[derive(Serialize)]
pub struct AcknowledgeMessageResponse {
    pub acknowledged: bool,
    pub message_id: i64,
}

pub async fn acknowledge_message(
    State(app_state): State<AppState>,
    Json(payload): Json<AcknowledgeMessagePayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    lib_core::model::message::MessageBmc::acknowledge(&ctx, mm, payload.message_id, agent.id.get())
        .await?;

    Ok(Json(AcknowledgeMessageResponse {
        acknowledged: true,
        message_id: payload.message_id,
    })
    .into_response())
}

// --- list_threads ---
#[derive(Deserialize)]
pub struct ListThreadsPayload {
    pub project_slug: String,
    #[serde(default = "default_threads_limit")]
    pub limit: i64,
}

fn default_threads_limit() -> i64 {
    50
}

#[derive(Serialize)]
pub struct ThreadSummaryResponse {
    pub thread_id: String,
    pub subject: String,
    pub message_count: usize,
    pub last_message_ts: chrono::NaiveDateTime,
}

pub async fn list_threads(
    State(app_state): State<AppState>,
    Json(payload): Json<ListThreadsPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let threads = lib_core::model::message::MessageBmc::list_threads(
        &ctx,
        mm,
        project.id.get(),
        payload.limit,
    )
    .await?;

    let responses: Vec<ThreadSummaryResponse> = threads
        .into_iter()
        .map(|t| ThreadSummaryResponse {
            thread_id: t.thread_id,
            subject: t.subject,
            message_count: t.message_count,
            last_message_ts: t.last_message_ts,
        })
        .collect();

    Ok(Json(responses).into_response())
}

// --- update_agent_profile ---
#[derive(Deserialize)]
pub struct UpdateAgentProfilePayload {
    pub project_slug: String,
    pub agent_name: String,
    pub task_description: Option<String>,
    pub attachments_policy: Option<String>,
    pub contact_policy: Option<String>,
}

#[derive(Serialize)]
pub struct UpdateAgentProfileResponse {
    pub updated: bool,
    pub agent_name: String,
}

pub async fn update_agent_profile(
    State(app_state): State<AppState>,
    Json(payload): Json<UpdateAgentProfilePayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    let update = lib_core::model::agent::AgentProfileUpdate {
        task_description: payload.task_description,
        attachments_policy: payload.attachments_policy,
        contact_policy: payload.contact_policy,
    };

    lib_core::model::agent::AgentBmc::update_profile(&ctx, mm, agent.id, update).await?;

    Ok(Json(UpdateAgentProfileResponse {
        updated: true,
        agent_name: payload.agent_name,
    })
    .into_response())
}

// --- request_contact ---
#[derive(Deserialize)]
pub struct RequestContactPayload {
    pub from_project_slug: String,
    pub from_agent_name: String,
    pub to_project_slug: String,
    pub to_agent_name: String,
    pub reason: String,
}

#[derive(Serialize)]
pub struct RequestContactResponse {
    pub link_id: i64,
    pub status: String,
}

pub async fn request_contact(
    State(app_state): State<AppState>,
    Json(payload): Json<RequestContactPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let from_project = lib_core::model::project::ProjectBmc::get_by_identifier(
        &ctx,
        mm,
        &payload.from_project_slug,
    )
    .await?;
    let from_agent = lib_core::model::agent::AgentBmc::get_by_name(
        &ctx,
        mm,
        from_project.id,
        &payload.from_agent_name,
    )
    .await?;

    let to_project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.to_project_slug)
            .await?;
    let to_agent = lib_core::model::agent::AgentBmc::get_by_name(
        &ctx,
        mm,
        to_project.id,
        &payload.to_agent_name,
    )
    .await?;

    let link_c = lib_core::model::agent_link::AgentLinkForCreate {
        a_project_id: from_project.id.get(),
        a_agent_id: from_agent.id.get(),
        b_project_id: to_project.id.get(),
        b_agent_id: to_agent.id.get(),
        reason: payload.reason,
    };

    let link_id =
        lib_core::model::agent_link::AgentLinkBmc::request_contact(&ctx, mm, link_c).await?;

    Ok(Json(RequestContactResponse {
        link_id,
        status: "pending".to_string(),
    })
    .into_response())
}

// --- respond_contact ---
#[derive(Deserialize)]
pub struct RespondContactPayload {
    pub link_id: i64,
    pub accept: bool,
}

#[derive(Serialize)]
pub struct RespondContactResponse {
    pub link_id: i64,
    pub status: String,
}

pub async fn respond_contact(
    State(app_state): State<AppState>,
    Json(payload): Json<RespondContactPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    lib_core::model::agent_link::AgentLinkBmc::respond_contact(
        &ctx,
        mm,
        payload.link_id,
        payload.accept,
    )
    .await?;

    Ok(Json(RespondContactResponse {
        link_id: payload.link_id,
        status: if payload.accept {
            "accepted"
        } else {
            "rejected"
        }
        .to_string(),
    })
    .into_response())
}

// --- list_contacts ---
#[derive(Deserialize)]
pub struct ListContactsPayload {
    pub project_slug: String,
    pub agent_name: String,
}

#[derive(Serialize)]
pub struct ContactResponse {
    pub id: i64,
    pub other_project_id: i64,
    pub other_agent_id: i64,
    pub status: String,
    pub reason: String,
    pub created_ts: chrono::NaiveDateTime,
}

pub async fn list_contacts(
    State(app_state): State<AppState>,
    Json(payload): Json<ListContactsPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    let links = lib_core::model::agent_link::AgentLinkBmc::list_contacts(
        &ctx,
        mm,
        project.id.get(),
        agent.id.get(),
    )
    .await?;

    let responses: Vec<ContactResponse> = links
        .into_iter()
        .map(|link| {
            // Determine which side is the "other" party
            let (other_project_id, other_agent_id) = if link.a_agent_id == agent.id.get() {
                (link.b_project_id, link.b_agent_id)
            } else {
                (link.a_project_id, link.a_agent_id)
            };
            ContactResponse {
                id: link.id,
                other_project_id,
                other_agent_id,
                status: link.status,
                reason: link.reason,
                created_ts: link.created_ts,
            }
        })
        .collect();

    Ok(Json(responses).into_response())
}

// --- set_contact_policy ---
// This reuses update_agent_profile with just contact_policy field
#[derive(Deserialize)]
pub struct SetContactPolicyPayload {
    pub project_slug: String,
    pub agent_name: String,
    pub contact_policy: String, // "auto", "manual", "deny"
}

#[derive(Serialize)]
pub struct SetContactPolicyResponse {
    pub updated: bool,
    pub contact_policy: String,
}

pub async fn set_contact_policy(
    State(app_state): State<AppState>,
    Json(payload): Json<SetContactPolicyPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    let update = lib_core::model::agent::AgentProfileUpdate {
        task_description: None,
        attachments_policy: None,
        contact_policy: Some(payload.contact_policy.clone()),
    };

    lib_core::model::agent::AgentBmc::update_profile(&ctx, mm, agent.id, update).await?;

    Ok(Json(SetContactPolicyResponse {
        updated: true,
        contact_policy: payload.contact_policy,
    })
    .into_response())
}

// --- acquire_build_slot ---
#[derive(Deserialize)]
pub struct AcquireBuildSlotPayload {
    pub project_slug: String,
    pub agent_name: String,
    pub slot_name: String,
    #[serde(default = "default_build_slot_ttl")]
    pub ttl_seconds: i64,
}

fn default_build_slot_ttl() -> i64 {
    1800 // 30 minutes default
}

#[derive(Serialize)]
pub struct AcquireBuildSlotResponse {
    pub slot_id: i64,
    pub slot_name: String,
    pub expires_ts: String,
}

pub async fn acquire_build_slot(
    State(app_state): State<AppState>,
    Json(payload): Json<AcquireBuildSlotPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    let slot_c = lib_core::model::build_slot::BuildSlotForCreate {
        project_id: project.id.get(),
        agent_id: agent.id.get(),
        slot_name: payload.slot_name.clone(),
        ttl_seconds: payload.ttl_seconds,
    };

    let slot_id = lib_core::model::build_slot::BuildSlotBmc::acquire(&ctx, mm, slot_c).await?;
    let expires = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(payload.ttl_seconds);

    Ok(Json(AcquireBuildSlotResponse {
        slot_id,
        slot_name: payload.slot_name,
        expires_ts: expires.format("%Y-%m-%dT%H:%M:%S").to_string(),
    })
    .into_response())
}

// --- renew_build_slot ---
#[derive(Deserialize)]
pub struct RenewBuildSlotPayload {
    pub slot_id: i64,
    #[serde(default = "default_build_slot_ttl")]
    pub ttl_seconds: i64,
}

#[derive(Serialize)]
pub struct RenewBuildSlotResponse {
    pub renewed: bool,
    pub slot_id: i64,
    pub new_expires_ts: String,
}

pub async fn renew_build_slot(
    State(app_state): State<AppState>,
    Json(payload): Json<RenewBuildSlotPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let new_expires = lib_core::model::build_slot::BuildSlotBmc::renew(
        &ctx,
        mm,
        payload.slot_id,
        payload.ttl_seconds,
    )
    .await?;

    Ok(Json(RenewBuildSlotResponse {
        renewed: true,
        slot_id: payload.slot_id,
        new_expires_ts: new_expires.format("%Y-%m-%dT%H:%M:%S").to_string(),
    })
    .into_response())
}

// --- release_build_slot ---
#[derive(Deserialize)]
pub struct ReleaseBuildSlotPayload {
    pub slot_id: i64,
}

#[derive(Serialize)]
pub struct ReleaseBuildSlotResponse {
    pub released: bool,
    pub slot_id: i64,
}

pub async fn release_build_slot(
    State(app_state): State<AppState>,
    Json(payload): Json<ReleaseBuildSlotPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    lib_core::model::build_slot::BuildSlotBmc::release(&ctx, mm, payload.slot_id).await?;

    Ok(Json(ReleaseBuildSlotResponse {
        released: true,
        slot_id: payload.slot_id,
    })
    .into_response())
}

// --- send_overseer_message ---
#[derive(Deserialize)]
pub struct SendOverseerMessagePayload {
    pub project_slug: String,
    pub agent_name: String,
    pub subject: String,
    pub body_md: String,
    #[serde(default)]
    pub importance: Option<String>,
}

#[derive(Serialize)]
pub struct SendOverseerMessageResponse {
    pub sent: bool,
    pub message_id: i64,
}

pub async fn send_overseer_message(
    State(app_state): State<AppState>,
    Json(payload): Json<SendOverseerMessagePayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    let msg_c = lib_core::model::overseer_message::OverseerMessageForCreate {
        project_id: project.id.get(),
        sender_id: agent.id.get(),
        subject: payload.subject,
        body_md: payload.body_md,
        importance: payload.importance.unwrap_or_else(|| "normal".to_string()),
    };

    let message_id =
        lib_core::model::overseer_message::OverseerMessageBmc::create(&ctx, mm, msg_c).await?;

    Ok(Json(SendOverseerMessageResponse {
        sent: true,
        message_id,
    })
    .into_response())
}

// --- list_macros ---
#[derive(Deserialize)]
pub struct ListMacrosPayload {
    pub project_slug: String,
}

#[derive(Serialize)]
pub struct MacroResponse {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub step_count: usize,
}

pub async fn list_macros(
    State(app_state): State<AppState>,
    Json(payload): Json<ListMacrosPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let macros = lib_core::model::macro_def::MacroDefBmc::list(&ctx, mm, project.id.get()).await?;

    let responses: Vec<MacroResponse> = macros
        .into_iter()
        .map(|m| MacroResponse {
            id: m.id,
            name: m.name,
            description: m.description,
            step_count: m.steps.len(),
        })
        .collect();

    Ok(Json(responses).into_response())
}

// --- register_macro ---
#[derive(Deserialize)]
pub struct RegisterMacroPayload {
    pub project_slug: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct RegisterMacroResponse {
    pub macro_id: i64,
    pub name: String,
}

pub async fn register_macro(
    State(app_state): State<AppState>,
    Json(payload): Json<RegisterMacroPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;

    let macro_c = lib_core::model::macro_def::MacroDefForCreate {
        project_id: project.id.get(),
        name: payload.name.clone(),
        description: payload.description,
        steps: payload.steps,
    };

    let macro_id = lib_core::model::macro_def::MacroDefBmc::create(&ctx, mm, macro_c).await?;

    Ok(Json(RegisterMacroResponse {
        macro_id,
        name: payload.name,
    })
    .into_response())
}

// --- unregister_macro ---
#[derive(Deserialize)]
pub struct UnregisterMacroPayload {
    pub project_slug: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct UnregisterMacroResponse {
    pub deleted: bool,
    pub name: String,
}

pub async fn unregister_macro(
    State(app_state): State<AppState>,
    Json(payload): Json<UnregisterMacroPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let deleted =
        lib_core::model::macro_def::MacroDefBmc::delete(&ctx, mm, project.id.get(), &payload.name)
            .await?;

    Ok(Json(UnregisterMacroResponse {
        deleted,
        name: payload.name,
    })
    .into_response())
}

// --- invoke_macro ---
#[derive(Deserialize)]
pub struct InvokeMacroPayload {
    pub project_slug: String,
    pub name: String,
    #[allow(dead_code)]
    pub params: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct InvokeMacroResponse {
    pub name: String,
    pub steps: Vec<serde_json::Value>,
    pub message: String,
}

pub async fn invoke_macro(
    State(app_state): State<AppState>,
    Json(payload): Json<InvokeMacroPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let macro_def = lib_core::model::macro_def::MacroDefBmc::get_by_name(
        &ctx,
        mm,
        project.id.get(),
        &payload.name,
    )
    .await?;

    // Return the steps - actual execution is client-side
    let step_count = macro_def.steps.len();
    Ok(Json(InvokeMacroResponse {
        name: macro_def.name,
        steps: macro_def.steps,
        message: format!(
            "Macro '{}' has {} steps to execute",
            payload.name, step_count
        ),
    })
    .into_response())
}

// ===========================================================================
// Convenience Macros (Python-compatible pre-built macros)
// ===========================================================================

// --- macro_start_session ---
// Combines: register_agent + file_reservation_paths
#[derive(Deserialize)]
pub struct MacroStartSessionPayload {
    pub project_slug: String,
    pub name: String,
    pub model: String,
    pub program: String,
    pub patterns: Vec<String>,
    #[serde(default = "default_reservation_ttl")]
    pub ttl_seconds: i64,
}

fn default_reservation_ttl() -> i64 {
    3600
}

#[derive(Serialize)]
pub struct MacroStartSessionResponse {
    pub agent_id: i64,
    pub agent_name: String,
    pub reservation_ids: Vec<i64>,
    pub message: String,
}

pub async fn macro_start_session(
    State(app_state): State<AppState>,
    Json(payload): Json<MacroStartSessionPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    // Step 1: Ensure project exists
    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;

    // Step 2: Register agent
    let agent_c = lib_core::model::agent::AgentForCreate {
        project_id: project.id,
        name: payload.name.clone(),
        model: payload.model,
        program: payload.program,
        task_description: String::new(),
    };
    let agent_id = lib_core::model::agent::AgentBmc::create(&ctx, mm, agent_c).await?;

    // Step 3: Create file reservations
    let mut reservation_ids = Vec::new();
    for pattern in &payload.patterns {
        let res_c = lib_core::model::file_reservation::FileReservationForCreate {
            project_id: project.id,
            agent_id,
            path_pattern: pattern.clone(),
            exclusive: true,
            reason: "Session start".to_string(),
            expires_ts: chrono::Utc::now().naive_utc()
                + chrono::Duration::seconds(payload.ttl_seconds),
        };
        let res_id =
            lib_core::model::file_reservation::FileReservationBmc::create(&ctx, mm, res_c).await?;
        reservation_ids.push(res_id);
    }

    Ok(Json(MacroStartSessionResponse {
        agent_id: agent_id.get(),
        agent_name: payload.name,
        reservation_ids,
        message: "Session started: agent registered and files reserved".to_string(),
    })
    .into_response())
}

// --- macro_file_reservation_cycle ---
// Reserve or release files
#[derive(Deserialize)]
pub struct MacroFileReservationCyclePayload {
    pub project_slug: String,
    pub agent_name: String,
    pub patterns: Vec<String>,
    pub action: String, // "reserve" or "release"
    #[serde(default = "default_reservation_ttl")]
    pub ttl_seconds: i64,
}

#[derive(Serialize)]
pub struct MacroFileReservationCycleResponse {
    pub action: String,
    pub affected_count: usize,
    pub ids: Vec<i64>,
}

pub async fn macro_file_reservation_cycle(
    State(app_state): State<AppState>,
    Json(payload): Json<MacroFileReservationCyclePayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let agent =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name)
            .await?;

    let mut ids = Vec::new();

    if payload.action == "reserve" {
        for pattern in &payload.patterns {
            let res_c = lib_core::model::file_reservation::FileReservationForCreate {
                project_id: project.id,
                agent_id: agent.id,
                path_pattern: pattern.clone(),
                exclusive: true,
                reason: "Reservation cycle".to_string(),
                expires_ts: chrono::Utc::now().naive_utc()
                    + chrono::Duration::seconds(payload.ttl_seconds),
            };
            let res_id =
                lib_core::model::file_reservation::FileReservationBmc::create(&ctx, mm, res_c)
                    .await?;
            ids.push(res_id);
        }
    } else if payload.action == "release" {
        for pattern in &payload.patterns {
            if let Some(released_id) =
                lib_core::model::file_reservation::FileReservationBmc::release_by_path(
                    &ctx,
                    mm,
                    project.id.get(),
                    agent.id.get(),
                    pattern,
                )
                .await?
            {
                ids.push(released_id);
            }
        }
    }

    Ok(Json(MacroFileReservationCycleResponse {
        action: payload.action,
        affected_count: ids.len(),
        ids,
    })
    .into_response())
}

// --- macro_contact_handshake ---
// Create bidirectional contact between two agents
#[derive(Deserialize)]
pub struct MacroContactHandshakePayload {
    pub project_slug: String,
    pub requester: String,
    pub target: String,
}

#[derive(Serialize)]
pub struct MacroContactHandshakeResponse {
    pub contacts_created: i32,
    pub link_ids: Vec<i64>,
}

pub async fn macro_contact_handshake(
    State(app_state): State<AppState>,
    Json(payload): Json<MacroContactHandshakePayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let requester =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.requester)
            .await?;
    let target =
        lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.target)
            .await?;

    // Create bidirectional contact: request + auto-accept
    let link_c = lib_core::model::agent_link::AgentLinkForCreate {
        a_project_id: project.id.get(),
        a_agent_id: requester.id.get(),
        b_project_id: project.id.get(),
        b_agent_id: target.id.get(),
        reason: "Handshake macro".to_string(),
    };

    let link_id =
        lib_core::model::agent_link::AgentLinkBmc::request_contact(&ctx, mm, link_c).await?;
    // Auto-accept the contact request
    lib_core::model::agent_link::AgentLinkBmc::respond_contact(&ctx, mm, link_id, true).await?;

    Ok(Json(MacroContactHandshakeResponse {
        contacts_created: 1,
        link_ids: vec![link_id],
    })
    .into_response())
}

// --- summarize_thread ---
// Note: Real summarization would use LLM, this returns a simple summary
#[derive(Deserialize)]
pub struct SummarizeThreadPayload {
    pub project_slug: String,
    pub thread_id: String,
    #[serde(default = "default_per_thread_limit")]
    pub per_thread_limit: i64,
    #[serde(default)]
    pub no_llm: bool,
}

fn default_per_thread_limit() -> i64 {
    100
}

#[derive(Serialize)]
pub struct SummarizeThreadResponse {
    pub thread_id: String,
    pub message_count: usize,
    pub participants: Vec<String>,
    pub subject: String,
    pub summary: String,
}

// Helper to call OpenAI API
async fn call_openai_summarize(
    messages: &[lib_core::model::message::Message],
) -> crate::error::Result<String> {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Ok(String::new());
    }

    let prompt = messages
        .iter()
        .map(|m| format!("{}: {}", m.sender_name, m.body_md))
        .collect::<Vec<_>>()
        .join("\n\n");

    let client = reqwest::Client::new();
    let resp = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant. Summarize the following thread concisely."},
                {"role": "user", "content": prompt}
            ],
            "max_tokens": 500
        }))
        .send()
        .await
        .map_err(|e| crate::ServerError::Internal(format!("OpenAI request failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(crate::ServerError::Internal(format!(
            "OpenAI API error: {}",
            resp.status()
        )));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| {
        crate::ServerError::Internal(format!("Failed to parse OpenAI response: {}", e))
    })?;

    let summary = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("Failed to extract summary")
        .to_string();

    Ok(summary)
}

pub async fn summarize_thread(
    State(app_state): State<AppState>,
    Json(payload): Json<SummarizeThreadPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let all_messages = lib_core::model::message::MessageBmc::list_by_thread(
        &ctx,
        mm,
        project.id.get(),
        &payload.thread_id,
    )
    .await?;

    let messages: Vec<_> = all_messages
        .into_iter()
        .take(payload.per_thread_limit as usize)
        .collect();

    let mut participants: Vec<String> = messages.iter().map(|m| m.sender_name.clone()).collect();
    participants.sort();
    participants.dedup();

    let subject = messages
        .first()
        .map(|m| m.subject.clone())
        .unwrap_or_default();

    let summary = if payload.no_llm {
        format!(
            "Thread with {} messages from {} participants. Latest: {}",
            messages.len(),
            participants.len(),
            messages
                .last()
                .map(|m| m.body_md.chars().take(100).collect::<String>())
                .unwrap_or_default()
        )
    } else {
        let llm_summary = call_openai_summarize(&messages).await?;
        if !llm_summary.is_empty() {
            llm_summary
        } else {
            format!(
                "Thread with {} messages from {} participants. Latest: {}",
                messages.len(),
                participants.len(),
                messages
                    .last()
                    .map(|m| m.body_md.chars().take(100).collect::<String>())
                    .unwrap_or_default()
            )
        }
    };

    Ok(Json(SummarizeThreadResponse {
        thread_id: payload.thread_id,
        message_count: messages.len(),
        participants,
        subject,
        summary,
    })
    .into_response())
}

// --- summarize_threads (batch) ---
#[derive(Deserialize)]
pub struct SummarizeThreadsPayload {
    pub project_slug: String,
    #[serde(default = "default_threads_limit")]
    pub limit: i64,
}

#[derive(Serialize)]
pub struct ThreadSummaryBrief {
    pub thread_id: String,
    pub subject: String,
    pub message_count: usize,
    pub last_message_ts: chrono::NaiveDateTime,
}

pub async fn summarize_threads(
    State(app_state): State<AppState>,
    Json(payload): Json<SummarizeThreadsPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let threads = lib_core::model::message::MessageBmc::list_threads(
        &ctx,
        mm,
        project.id.get(),
        payload.limit,
    )
    .await?;

    let summaries: Vec<ThreadSummaryBrief> = threads
        .into_iter()
        .map(|t| ThreadSummaryBrief {
            thread_id: t.thread_id,
            subject: t.subject,
            message_count: t.message_count,
            last_message_ts: t.last_message_ts,
        })
        .collect();

    Ok(Json(summaries).into_response())
}

// --- install_precommit_guard ---
#[derive(Deserialize)]
pub struct InstallPrecommitGuardPayload {
    pub project_slug: String,
    pub target_repo_path: String,
}

#[derive(Serialize)]
pub struct InstallPrecommitGuardResponse {
    pub installed: bool,
    pub hook_path: String,
    pub message: String,
}

pub async fn install_precommit_guard(
    State(app_state): State<AppState>,
    Json(payload): Json<InstallPrecommitGuardPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    // Verify project exists
    let _project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;

    let target_path = std::path::PathBuf::from(&payload.target_repo_path);
    let hooks_dir = target_path.join(".git").join("hooks");
    let hook_path = hooks_dir.join("pre-commit");

    // Create the pre-commit hook script
    let hook_script = format!(
        r#"#!/bin/sh
# MCP Agent Mail Pre-commit Guard
# Installed for project: {}

# Check for file reservation conflicts
# This is an advisory check - can be bypassed with AGENT_MAIL_BYPASS=1

if [ -n "$AGENT_MAIL_BYPASS" ]; then
    echo "MCP Agent Mail: Bypass enabled, skipping reservation check"
    exit 0
fi

# Calls API to check file reservation conflicts
# See bd-577.9 for full implementation
echo "MCP Agent Mail: Pre-commit guard active"
exit 0
"#,
        payload.project_slug
    );

    // Ensure hooks directory exists
    if !hooks_dir.exists() {
        std::fs::create_dir_all(&hooks_dir)?;
    }

    // Write the hook
    std::fs::write(&hook_path, hook_script)?;

    // Make it executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&hook_path, perms)?;
    }

    Ok(Json(InstallPrecommitGuardResponse {
        installed: true,
        hook_path: hook_path.to_string_lossy().to_string(),
        message: format!(
            "Pre-commit guard installed for project '{}'",
            payload.project_slug
        ),
    })
    .into_response())
}

// --- uninstall_precommit_guard ---
#[derive(Deserialize)]
pub struct UninstallPrecommitGuardPayload {
    pub target_repo_path: String,
}

#[derive(Serialize)]
pub struct UninstallPrecommitGuardResponse {
    pub uninstalled: bool,
    pub message: String,
}

pub async fn uninstall_precommit_guard(
    State(_app_state): State<AppState>,
    Json(payload): Json<UninstallPrecommitGuardPayload>,
) -> crate::error::Result<Response> {
    let target_path = std::path::PathBuf::from(&payload.target_repo_path);
    let hook_path = target_path.join(".git").join("hooks").join("pre-commit");

    if hook_path.exists() {
        // Check if it's our hook before removing
        let content = std::fs::read_to_string(&hook_path)?;
        if content.contains("MCP Agent Mail Pre-commit Guard") {
            std::fs::remove_file(&hook_path)?;
            return Ok(Json(UninstallPrecommitGuardResponse {
                uninstalled: true,
                message: "Pre-commit guard removed".to_string(),
            })
            .into_response());
        } else {
            return Ok(Json(UninstallPrecommitGuardResponse {
                uninstalled: false,
                message: "Pre-commit hook exists but is not an MCP Agent Mail guard".to_string(),
            })
            .into_response());
        }
    }

    Ok(Json(UninstallPrecommitGuardResponse {
        uninstalled: false,
        message: "No pre-commit hook found".to_string(),
    })
    .into_response())
}

// --- Metrics ---

#[derive(Deserialize)]
pub struct ListMetricsParams {
    pub project_id: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_tool_metrics(
    State(state): State<AppState>,
    Query(params): Query<ListMetricsParams>,
) -> crate::error::Result<Response> {
    use lib_core::model::tool_metric::ToolMetricBmc;

    let ctx = Ctx::root_ctx();
    let limit = params.limit.unwrap_or(50);
    let metrics = ToolMetricBmc::list_recent(&ctx, &state.mm, params.project_id, limit).await?;

    Ok(Json(metrics).into_response())
}

pub async fn get_tool_stats(
    State(state): State<AppState>,
    Query(params): Query<ListMetricsParams>,
) -> crate::error::Result<Response> {
    use lib_core::model::tool_metric::ToolMetricBmc;

    let ctx = Ctx::root_ctx();
    let stats = ToolMetricBmc::get_stats(&ctx, &state.mm, params.project_id).await?;

    Ok(Json(stats).into_response())
}

// --- Activity ---

#[derive(Deserialize)]
pub struct ListActivityParams {
    pub project_id: i64,
    pub limit: Option<i64>,
}

pub async fn list_activity(
    State(state): State<AppState>,
    Query(params): Query<ListActivityParams>,
) -> crate::error::Result<Response> {
    use lib_core::model::activity::ActivityBmc;

    let ctx = Ctx::root_ctx();
    let limit = params.limit.unwrap_or(50);
    let items = ActivityBmc::list_recent(&ctx, &state.mm, params.project_id, limit).await?;

    Ok(Json(items).into_response())
}

// --- commit_archive ---
#[derive(Deserialize)]
pub struct CommitArchivePayload {
    pub project_slug: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct CommitArchiveResponse {
    pub commit_id: String,
    pub project_slug: String,
}

pub async fn commit_archive(
    State(app_state): State<AppState>,
    Json(payload): Json<CommitArchivePayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let commit_id = lib_core::model::export::ExportBmc::commit_archive(
        &ctx,
        mm,
        &payload.project_slug,
        &payload.message,
    )
    .await?;

    Ok(Json(CommitArchiveResponse {
        commit_id,
        project_slug: payload.project_slug,
    })
    .into_response())
}

// --- list_project_siblings ---
#[derive(Deserialize)]
pub struct ListProjectSiblingsPayload {
    pub project_slug: String,
}

#[derive(Serialize)]
pub struct ProjectSiblingResponse {
    pub id: i64,
    pub other_project_id: i64,
    pub score: f64,
    pub status: String,
    pub rationale: String,
}

pub async fn list_project_siblings(
    State(app_state): State<AppState>,
    Json(payload): Json<ListProjectSiblingsPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project =
        lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug)
            .await?;
    let siblings = lib_core::model::project_sibling_suggestion::ProjectSiblingSuggestionBmc::list(
        &ctx,
        mm,
        project.id.get(),
    )
    .await?;

    let responses: Vec<ProjectSiblingResponse> = siblings
        .into_iter()
        .map(|s| {
            let other_id = if s.project_a_id == project.id.get() {
                s.project_b_id
            } else {
                s.project_a_id
            };
            ProjectSiblingResponse {
                id: s.id,
                other_project_id: other_id,
                score: s.score,
                status: s.status,
                rationale: s.rationale,
            }
        })
        .collect();

    Ok(Json(responses).into_response())
}

// --- list_pending_reviews ---
// Single-call API for LLM agents to retrieve messages awaiting acknowledgment

#[derive(Deserialize)]
pub struct ListPendingReviewsQuery {
    /// Filter by project slug (optional)
    pub project: Option<String>,
    /// Filter by sender agent name (optional)
    pub sender: Option<String>,
    /// Maximum results (default: 5, max: 50)
    #[serde(default = "default_pending_reviews_limit")]
    pub limit: i64,
}

fn default_pending_reviews_limit() -> i64 {
    5
}

#[derive(Serialize)]
pub struct SenderInfo {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize)]
pub struct ProjectInfo {
    pub id: i64,
    pub slug: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct ThreadInfo {
    pub id: String,
    pub message_count: i64,
}

#[derive(Serialize, Deserialize)]
pub struct RecipientStatus {
    pub agent_id: i64,
    pub agent_name: String,
    pub recipient_type: String,
    #[serde(default)]
    pub status: String,
    pub read_ts: Option<String>,
    pub ack_ts: Option<String>,
}

#[derive(Serialize)]
pub struct PendingReview {
    pub message_id: i64,
    pub subject: String,
    pub body_md: String,
    pub importance: String,
    pub created_ts: String,
    pub attachments: Vec<serde_json::Value>,
    pub sender: SenderInfo,
    pub project: ProjectInfo,
    pub thread: Option<ThreadInfo>,
    pub recipients: Vec<RecipientStatus>,
    pub pending_count: usize,
    pub read_count: usize,
}

#[derive(Serialize)]
pub struct PendingReviewsResponse {
    pub pending_reviews: Vec<PendingReview>,
    pub total_count: usize,
}

pub async fn list_pending_reviews(
    State(app_state): State<AppState>,
    Query(params): Query<ListPendingReviewsQuery>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    // Resolve project_id from slug if provided
    let project_id = if let Some(ref slug) = params.project {
        let project =
            lib_core::model::project::ProjectBmc::get_by_identifier(&ctx, mm, slug).await?;
        Some(project.id)
    } else {
        None
    };

    // Resolve sender_id from name if provided (requires project context)
    let sender_id = if let Some(ref sender_name) = params.sender {
        if let Some(pid) = project_id {
            let agent =
                lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, pid, sender_name).await?;
            Some(agent.id)
        } else {
            // If no project specified, we can't resolve sender by name
            None
        }
    } else {
        None
    };

    let rows = lib_core::model::message::MessageBmc::list_pending_reviews(
        &ctx,
        mm,
        project_id.map(|p| p.get()),
        sender_id.map(|s| s.get()),
        params.limit,
    )
    .await?;

    // Transform rows into response structs
    let pending_reviews: Vec<PendingReview> = rows
        .into_iter()
        .map(|row| {
            // Parse recipients JSON
            let recipients: Vec<RecipientStatus> =
                serde_json::from_str(&row.recipients_json).unwrap_or_default();

            // Parse attachments
            let attachments: Vec<serde_json::Value> =
                serde_json::from_str(&row.attachments).unwrap_or_default();

            // Calculate counts
            let pending_count = recipients.iter().filter(|r| r.ack_ts.is_none()).count();
            let read_count = recipients.iter().filter(|r| r.read_ts.is_some()).count();

            // Build thread info if present
            let thread = row.thread_id.as_ref().map(|tid| ThreadInfo {
                id: tid.clone(),
                message_count: row.thread_count,
            });

            PendingReview {
                message_id: row.message_id,
                subject: row.subject,
                body_md: row.body_md,
                importance: row.importance,
                created_ts: row.created_ts.format("%Y-%m-%dT%H:%M:%S").to_string(),
                attachments,
                sender: SenderInfo {
                    id: row.sender_id,
                    name: row.sender_name,
                },
                project: ProjectInfo {
                    id: row.project_id,
                    slug: row.project_slug,
                    name: row.project_name,
                },
                thread,
                recipients,
                pending_count,
                read_count,
            }
        })
        .collect();

    let total_count = pending_reviews.len();

    Ok(Json(PendingReviewsResponse {
        pending_reviews,
        total_count,
    })
    .into_response())
}

// =============================================================================
// ARCHIVE BROWSER API
// =============================================================================

// --- list_archive_commits ---
#[derive(Deserialize)]
pub struct ListArchiveCommitsQuery {
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
    #[serde(default = "default_commits_limit")]
    pub limit: usize,
}

fn default_commits_limit() -> usize {
    50
}

pub async fn list_archive_commits(
    State(app_state): State<AppState>,
    Query(params): Query<ListArchiveCommitsQuery>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let filter = if params.author.is_some() || params.path.is_some() {
        Some(lib_core::model::archive_browser::CommitFilter {
            author: params.author,
            path: params.path,
            since: params.since.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|d| d.with_timezone(&chrono::Utc))
            }),
            until: params.until.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|d| d.with_timezone(&chrono::Utc))
            }),
            message_contains: None,
        })
    } else {
        None
    };

    let commits = lib_core::model::archive_browser::ArchiveBrowserBmc::list_commits(
        &ctx,
        mm,
        filter,
        params.limit,
    )
    .await?;

    Ok(Json(commits).into_response())
}

// --- get_archive_commit ---
pub async fn get_archive_commit(
    State(app_state): State<AppState>,
    Path(sha): Path<String>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let details =
        lib_core::model::archive_browser::ArchiveBrowserBmc::commit_details(&ctx, mm, &sha).await?;

    Ok(Json(details).into_response())
}

// --- list_archive_files ---
#[derive(Deserialize)]
pub struct ListArchiveFilesQuery {
    #[serde(default)]
    pub path: Option<String>,
}

pub async fn list_archive_files(
    State(app_state): State<AppState>,
    Path(sha): Path<String>,
    Query(params): Query<ListArchiveFilesQuery>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let dir_path = params.path.unwrap_or_default();
    let files = lib_core::model::archive_browser::ArchiveBrowserBmc::list_files_at(
        &ctx, mm, &sha, &dir_path,
    )
    .await?;

    Ok(Json(files).into_response())
}

// --- get_archive_file_content ---
#[derive(Deserialize)]
pub struct GetArchiveFileContentQuery {
    pub path: String,
}

pub async fn get_archive_file_content(
    State(app_state): State<AppState>,
    Path(sha): Path<String>,
    Query(params): Query<GetArchiveFileContentQuery>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let content = lib_core::model::archive_browser::ArchiveBrowserBmc::file_content_at(
        &ctx,
        mm,
        &sha,
        &params.path,
    )
    .await?;

    Ok(Json(content).into_response())
}

// --- get_archive_activity ---
#[derive(Deserialize)]
pub struct GetArchiveActivityQuery {
    #[serde(default = "default_since")]
    pub since: String,
    #[serde(default = "default_until")]
    pub until: String,
}

fn default_since() -> String {
    (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339()
}

fn default_until() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub async fn get_archive_activity(
    State(app_state): State<AppState>,
    Query(params): Query<GetArchiveActivityQuery>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let since = chrono::DateTime::parse_from_rfc3339(&params.since)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now() - chrono::Duration::days(30));

    let until = chrono::DateTime::parse_from_rfc3339(&params.until)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let activity = lib_core::model::archive_browser::ArchiveBrowserBmc::activity_timeline(
        &ctx, mm, since, until,
    )
    .await?;

    Ok(Json(activity).into_response())
}
