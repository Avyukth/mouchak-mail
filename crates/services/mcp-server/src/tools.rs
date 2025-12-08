use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
use lib_core::{self, Ctx};
use serde::{Deserialize, Serialize};

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
    }).into_response())
}

// --- ensure_project ---
#[derive(Deserialize)]
pub struct EnsureProjectPayload {
    pub human_key: String,
}

#[derive(Serialize)]
pub struct EnsureProjectResponse {
    pub project_id: i64,
    pub slug: String,
}

pub async fn ensure_project(State(app_state): State<AppState>, Json(payload): Json<EnsureProjectPayload>) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx(); // For now, use a root context
    let mm = &app_state.mm;

    // Call lib-core ProjectBmc to ensure project exists
    let project = match lib_core::model::project::ProjectBmc::get_by_human_key(&ctx, mm, &payload.human_key).await {
        Ok(p) => p,
        Err(e) => {
            if let lib_core::Error::ProjectNotFound(_) = e {
                // If not found, create it. Generate a slug here based on human_key.
                let slug = lib_core::utils::slugify(&payload.human_key);
                let _id = lib_core::model::project::ProjectBmc::create(&ctx, mm, &slug, &payload.human_key).await?;
                lib_core::model::project::ProjectBmc::get_by_human_key(&ctx, mm, &payload.human_key).await?
            } else {
                return Err(e.into());
            }
        }
    };

    Ok(Json(EnsureProjectResponse {
        project_id: project.id,
        slug: project.slug,
    }).into_response())
}

// --- register_agent ---
#[derive(Deserialize)]
pub struct RegisterAgentPayload {
    pub project_slug: String,
    pub name: String,
    pub program: String,
    pub model: String,
    pub task_description: String,
}

#[derive(Serialize)]
pub struct RegisterAgentResponse {
    pub agent_id: i64,
    pub name: String,
}

pub async fn register_agent(State(app_state): State<AppState>, Json(payload): Json<RegisterAgentPayload>) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;

    let agent_c = lib_core::model::agent::AgentForCreate {
        project_id: project.id,
        name: payload.name.clone(),
        program: payload.program,
        model: payload.model,
        task_description: payload.task_description,
    };

    let agent_id = lib_core::model::agent::AgentBmc::create(&ctx, mm, agent_c).await?;

    Ok(Json(RegisterAgentResponse {
        agent_id,
        name: payload.name,
    }).into_response())
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
    pub subject: String,
    pub body_md: String,
    pub thread_id: Option<String>,
    pub importance: Option<String>,
    #[serde(default)]
    pub ack_required: bool,
}

#[derive(Serialize)]
pub struct SendMessageResponse {
    pub message_id: i64,
}

pub async fn send_message(State(app_state): State<AppState>, Json(payload): Json<SendMessagePayload>) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let sender = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.sender_name).await?;

    let mut recipient_ids = Vec::new();
    for name in payload.recipient_names {
        let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &name).await?;
        recipient_ids.push(agent.id);
    }

    let msg_c = lib_core::model::message::MessageForCreate {
        project_id: project.id,
        sender_id: sender.id,
        recipient_ids,
        subject: payload.subject,
        body_md: payload.body_md,
        thread_id: payload.thread_id,
        importance: payload.importance,
    };

    let message_id = lib_core::model::message::MessageBmc::create(&ctx, mm, msg_c).await?;

    Ok(Json(SendMessageResponse { message_id }).into_response())
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

pub async fn list_inbox(State(app_state): State<AppState>, Json(payload): Json<ListInboxPayload>) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    let messages = lib_core::model::message::MessageBmc::list_inbox_for_agent(&ctx, mm, project.id, agent.id, payload.limit).await?;

    let inbox_msgs: Vec<InboxMessage> = messages.into_iter().map(|msg| InboxMessage {
        id: msg.id,
        subject: msg.subject,
        sender_name: msg.sender_name,
        created_ts: msg.created_ts,
    }).collect();

    Ok(Json(inbox_msgs).into_response())
}

// --- list_all_projects ---
#[derive(Serialize)]
pub struct ProjectResponse {
    pub id: i64,
    pub slug: String,
    pub human_key: String,
    pub created_at: chrono::NaiveDateTime,
}

pub async fn list_all_projects(State(app_state): State<AppState>) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let projects = lib_core::model::project::ProjectBmc::list_all(&ctx, mm).await?;

    let project_responses: Vec<ProjectResponse> = projects.into_iter().map(|p| ProjectResponse {
        id: p.id,
        slug: p.slug,
        human_key: p.human_key,
        created_at: p.created_at,
    }).collect();

    Ok(Json(project_responses).into_response())
}

// --- list_all_agents_for_project ---
// Keep for backwards compatibility with JSON body requests
#[derive(Deserialize)]
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &project_slug).await?;
    let agents = lib_core::model::agent::AgentBmc::list_all_for_project(&ctx, mm, project.id).await?;

    let agent_responses: Vec<AgentResponse> = agents.into_iter().map(|a| AgentResponse {
        id: a.id,
        name: a.name,
        program: a.program,
        model: a.model,
        task_description: a.task_description,
        inception_ts: a.inception_ts,
        last_active_ts: a.last_active_ts,
    }).collect();

    Ok(Json(agent_responses).into_response())
}

// --- get_message ---
// Keep for backwards compatibility
#[derive(Deserialize)]
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
}

pub async fn get_message(
    State(app_state): State<AppState>,
    Path(message_id): Path<i64>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let message = lib_core::model::message::MessageBmc::get(&ctx, mm, message_id).await?;

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
    }).into_response())
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

pub async fn file_reservation_paths(State(app_state): State<AppState>, Json(payload): Json<FileReservationPathsPayload>) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    // 1. Get active reservations
    let active_reservations = FileReservationBmc::list_active_for_project(&ctx, mm, project.id).await?;

    let mut granted = Vec::new();
    let mut conflicts = Vec::new();

    let ttl = payload.ttl_seconds.unwrap_or(3600);
    let now = chrono::Utc::now().naive_utc();
    let expires_ts = now + chrono::Duration::seconds(ttl);

    for path in payload.paths {
        // Check conflicts
        // Simple overlap check for now (exact match)
        // TODO: Implement robust glob matching using globset
        for res in &active_reservations {
            if res.agent_id != agent.id {
                if res.exclusive || payload.exclusive {
                     if res.path_pattern == path {
                         conflicts.push(FileReservationConflict {
                             path_pattern: res.path_pattern.clone(),
                             exclusive: res.exclusive,
                             expires_ts: res.expires_ts.format("%Y-%m-%dT%H:%M:%S").to_string(),
                             conflict_type: "FILE_RESERVATION_CONFLICT".to_string(),
                             message: format!("Conflict with reservation held by agent ID {}", res.agent_id),
                         });
                         // We don't break, we collect all conflicts? Or just one per path?
                         // Python collects conflicts.
                     }
                }
            }
        }

        // Advisory model: always grant
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

    Ok(Json(FileReservationPathsResponse {
        granted,
        conflicts,
    }).into_response())
}

// --- create_agent_identity ---
// Generates memorable adjective+noun names like BlueMountain, GreenCastle
const ADJECTIVES: &[&str] = &[
    "Blue", "Green", "Red", "Golden", "Silver", "Crystal", "Dark", "Bright",
    "Swift", "Calm", "Bold", "Wise", "Noble", "Grand", "Mystic", "Ancient",
    "Lunar", "Solar", "Azure", "Coral", "Amber", "Jade", "Onyx", "Pearl",
    "Scarlet", "Violet", "Copper", "Bronze", "Iron", "Steel", "Frost", "Storm",
];

const NOUNS: &[&str] = &[
    "Mountain", "Castle", "River", "Forest", "Valley", "Harbor", "Tower", "Bridge",
    "Falcon", "Phoenix", "Dragon", "Wolf", "Eagle", "Lion", "Hawk", "Owl",
    "Oak", "Pine", "Willow", "Cedar", "Maple", "Birch", "Ash", "Elm",
    "Stone", "Crystal", "Star", "Moon", "Sun", "Cloud", "Wind", "Thunder",
];

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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;

    // Get existing agent names to avoid collisions
    let existing_agents = lib_core::model::agent::AgentBmc::list_all_for_project(&ctx, mm, project.id).await?;
    let existing_names: std::collections::HashSet<String> = existing_agents.iter().map(|a| a.name.clone()).collect();

    // Generate names
    let mut alternatives = Vec::new();
    let mut rng_seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;

    // Simple pseudo-random using seed
    let mut next_rand = || {
        rng_seed = rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
        rng_seed
    };

    for _ in 0..10 {
        let adj_idx = next_rand() % ADJECTIVES.len();
        let noun_idx = next_rand() % NOUNS.len();
        let name = format!("{}{}", ADJECTIVES[adj_idx], NOUNS[noun_idx]);

        if !existing_names.contains(&name) && !alternatives.contains(&name) {
            alternatives.push(name);
            if alternatives.len() >= 5 {
                break;
            }
        }
    }

    // If hint provided, try to incorporate it
    if let Some(hint) = &payload.hint {
        let hint_lower = hint.to_lowercase();
        // Try to find matching adjective or noun
        for adj in ADJECTIVES {
            if adj.to_lowercase().contains(&hint_lower) {
                for noun in NOUNS {
                    let name = format!("{}{}", adj, noun);
                    if !existing_names.contains(&name) && !alternatives.contains(&name) {
                        alternatives.insert(0, name);
                        break;
                    }
                }
                break;
            }
        }
    }

    let suggested_name = alternatives.first().cloned().unwrap_or_else(|| "Agent1".to_string());

    Ok(Json(CreateAgentIdentityResponse {
        suggested_name,
        alternatives,
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    Ok(Json(WhoisResponse {
        id: agent.id,
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
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;

    let reservations = if payload.active_only.unwrap_or(true) {
        FileReservationBmc::list_active_for_project(&ctx, mm, project.id).await?
    } else {
        FileReservationBmc::list_all_for_project(&ctx, mm, project.id).await?
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
            agent_id: res.agent_id,
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    let mut released_ids = Vec::new();

    for path in &payload.paths {
        if let Some(id) = FileReservationBmc::release_by_path(&ctx, mm, project.id, agent.id, path).await? {
            released_ids.push(id);
        }
    }

    Ok(Json(ReleaseFileReservationResponse {
        released_count: released_ids.len(),
        released_ids,
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let messages = lib_core::model::message::MessageBmc::list_by_thread(&ctx, mm, project.id, &payload.thread_id).await?;

    let responses: Vec<MessageResponse> = messages.into_iter().map(|msg| MessageResponse {
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
    }).collect();

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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let sender = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.sender_name).await?;

    // Get original message to extract thread_id and original sender as recipient
    let original_msg = lib_core::model::message::MessageBmc::get(&ctx, mm, payload.message_id).await?;

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
        project_id: project.id,
        sender_id: sender.id,
        recipient_ids,
        subject,
        body_md: payload.body_md,
        thread_id,
        importance: payload.importance,
    };

    let message_id = lib_core::model::message::MessageBmc::create(&ctx, mm, msg_c).await?;

    Ok(Json(SendMessageResponse { message_id }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;

    let messages = lib_core::model::message::MessageBmc::search(&ctx, mm, project.id, &payload.query, payload.limit).await?;

    let results: Vec<SearchMessageResult> = messages.into_iter().map(|msg| SearchMessageResult {
        id: msg.id,
        subject: msg.subject,
        sender_name: msg.sender_name,
        thread_id: msg.thread_id,
        body_md: msg.body_md,
        importance: msg.importance,
        created_ts: msg.created_ts,
    }).collect();

    let count = results.len();

    Ok(Json(SearchMessagesResponse {
        query: payload.query,
        results,
        count,
    }).into_response())
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
    }).into_response())
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
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;

    // Count agents
    let agents = lib_core::model::agent::AgentBmc::list_all_for_project(&ctx, mm, project.id).await?;
    let agent_count = agents.len();

    // Count messages
    let message_count = lib_core::model::project::ProjectBmc::count_messages(&ctx, mm, project.id).await?;

    Ok(Json(ProjectInfoResponse {
        id: project.id,
        slug: project.slug,
        human_key: project.human_key,
        created_at: project.created_at,
        agent_count,
        message_count: message_count as usize,
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    // Count sent and received messages
    let sent_count = lib_core::model::agent::AgentBmc::count_messages_sent(&ctx, mm, agent.id).await?;
    let received_count = lib_core::model::agent::AgentBmc::count_messages_received(&ctx, mm, agent.id).await?;

    // Count active reservations
    let reservations = FileReservationBmc::list_active_for_project(&ctx, mm, project.id).await?;
    let active_reservations = reservations.iter().filter(|r| r.agent_id == agent.id).count();

    Ok(Json(AgentProfileResponse {
        id: agent.id,
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
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    lib_core::model::message::MessageBmc::mark_read(&ctx, mm, payload.message_id, agent.id).await?;

    Ok(Json(MarkMessageReadResponse {
        marked: true,
        message_id: payload.message_id,
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    lib_core::model::message::MessageBmc::acknowledge(&ctx, mm, payload.message_id, agent.id).await?;

    Ok(Json(AcknowledgeMessageResponse {
        acknowledged: true,
        message_id: payload.message_id,
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let threads = lib_core::model::message::MessageBmc::list_threads(&ctx, mm, project.id, payload.limit).await?;

    let responses: Vec<ThreadSummaryResponse> = threads.into_iter().map(|t| ThreadSummaryResponse {
        thread_id: t.thread_id,
        subject: t.subject,
        message_count: t.message_count,
        last_message_ts: t.last_message_ts,
    }).collect();

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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    let update = lib_core::model::agent::AgentProfileUpdate {
        task_description: payload.task_description,
        attachments_policy: payload.attachments_policy,
        contact_policy: payload.contact_policy,
    };

    lib_core::model::agent::AgentBmc::update_profile(&ctx, mm, agent.id, update).await?;

    Ok(Json(UpdateAgentProfileResponse {
        updated: true,
        agent_name: payload.agent_name,
    }).into_response())
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

    let from_project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.from_project_slug).await?;
    let from_agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, from_project.id, &payload.from_agent_name).await?;

    let to_project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.to_project_slug).await?;
    let to_agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, to_project.id, &payload.to_agent_name).await?;

    let link_c = lib_core::model::agent_link::AgentLinkForCreate {
        a_project_id: from_project.id,
        a_agent_id: from_agent.id,
        b_project_id: to_project.id,
        b_agent_id: to_agent.id,
        reason: payload.reason,
    };

    let link_id = lib_core::model::agent_link::AgentLinkBmc::request_contact(&ctx, mm, link_c).await?;

    Ok(Json(RequestContactResponse {
        link_id,
        status: "pending".to_string(),
    }).into_response())
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

    lib_core::model::agent_link::AgentLinkBmc::respond_contact(&ctx, mm, payload.link_id, payload.accept).await?;

    Ok(Json(RespondContactResponse {
        link_id: payload.link_id,
        status: if payload.accept { "accepted" } else { "rejected" }.to_string(),
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    let links = lib_core::model::agent_link::AgentLinkBmc::list_contacts(&ctx, mm, project.id, agent.id).await?;

    let responses: Vec<ContactResponse> = links.into_iter().map(|link| {
        // Determine which side is the "other" party
        let (other_project_id, other_agent_id) = if link.a_agent_id == agent.id {
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
    }).collect();

    Ok(Json(responses).into_response())
}

// --- set_contact_policy ---
// This reuses update_agent_profile with just contact_policy field
#[derive(Deserialize)]
pub struct SetContactPolicyPayload {
    pub project_slug: String,
    pub agent_name: String,
    pub contact_policy: String,  // "auto", "manual", "deny"
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    let update = lib_core::model::agent::AgentProfileUpdate {
        task_description: None,
        attachments_policy: None,
        contact_policy: Some(payload.contact_policy.clone()),
    };

    lib_core::model::agent::AgentBmc::update_profile(&ctx, mm, agent.id, update).await?;

    Ok(Json(SetContactPolicyResponse {
        updated: true,
        contact_policy: payload.contact_policy,
    }).into_response())
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
    1800  // 30 minutes default
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    let slot_c = lib_core::model::build_slot::BuildSlotForCreate {
        project_id: project.id,
        agent_id: agent.id,
        slot_name: payload.slot_name.clone(),
        ttl_seconds: payload.ttl_seconds,
    };

    let slot_id = lib_core::model::build_slot::BuildSlotBmc::acquire(&ctx, mm, slot_c).await?;
    let expires = chrono::Utc::now().naive_utc() + chrono::Duration::seconds(payload.ttl_seconds);

    Ok(Json(AcquireBuildSlotResponse {
        slot_id,
        slot_name: payload.slot_name,
        expires_ts: expires.format("%Y-%m-%dT%H:%M:%S").to_string(),
    }).into_response())
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

    let new_expires = lib_core::model::build_slot::BuildSlotBmc::renew(&ctx, mm, payload.slot_id, payload.ttl_seconds).await?;

    Ok(Json(RenewBuildSlotResponse {
        renewed: true,
        slot_id: payload.slot_id,
        new_expires_ts: new_expires.format("%Y-%m-%dT%H:%M:%S").to_string(),
    }).into_response())
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
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let agent = lib_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &payload.agent_name).await?;

    let msg_c = lib_core::model::overseer_message::OverseerMessageForCreate {
        project_id: project.id,
        sender_id: agent.id,
        subject: payload.subject,
        body_md: payload.body_md,
        importance: payload.importance.unwrap_or_else(|| "normal".to_string()),
    };

    let message_id = lib_core::model::overseer_message::OverseerMessageBmc::create(&ctx, mm, msg_c).await?;

    Ok(Json(SendOverseerMessageResponse {
        sent: true,
        message_id,
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let macros = lib_core::model::macro_def::MacroDefBmc::list(&ctx, mm, project.id).await?;

    let responses: Vec<MacroResponse> = macros.into_iter().map(|m| MacroResponse {
        id: m.id,
        name: m.name,
        description: m.description,
        step_count: m.steps.len(),
    }).collect();

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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;

    let macro_c = lib_core::model::macro_def::MacroDefForCreate {
        project_id: project.id,
        name: payload.name.clone(),
        description: payload.description,
        steps: payload.steps,
    };

    let macro_id = lib_core::model::macro_def::MacroDefBmc::create(&ctx, mm, macro_c).await?;

    Ok(Json(RegisterMacroResponse {
        macro_id,
        name: payload.name,
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let deleted = lib_core::model::macro_def::MacroDefBmc::delete(&ctx, mm, project.id, &payload.name).await?;

    Ok(Json(UnregisterMacroResponse {
        deleted,
        name: payload.name,
    }).into_response())
}

// --- invoke_macro ---
#[derive(Deserialize)]
pub struct InvokeMacroPayload {
    pub project_slug: String,
    pub name: String,
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let macro_def = lib_core::model::macro_def::MacroDefBmc::get_by_name(&ctx, mm, project.id, &payload.name).await?;

    // Return the steps - actual execution is client-side
    let step_count = macro_def.steps.len();
    Ok(Json(InvokeMacroResponse {
        name: macro_def.name,
        steps: macro_def.steps,
        message: format!("Macro '{}' has {} steps to execute", payload.name, step_count),
    }).into_response())
}

// --- summarize_thread ---
// Note: Real summarization would use LLM, this returns a simple summary
#[derive(Deserialize)]
pub struct SummarizeThreadPayload {
    pub project_slug: String,
    pub thread_id: String,
}

#[derive(Serialize)]
pub struct SummarizeThreadResponse {
    pub thread_id: String,
    pub message_count: usize,
    pub participants: Vec<String>,
    pub subject: String,
    pub summary: String,
}

pub async fn summarize_thread(
    State(app_state): State<AppState>,
    Json(payload): Json<SummarizeThreadPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let messages = lib_core::model::message::MessageBmc::list_by_thread(&ctx, mm, project.id, &payload.thread_id).await?;

    let mut participants: Vec<String> = messages.iter().map(|m| m.sender_name.clone()).collect();
    participants.sort();
    participants.dedup();

    let subject = messages.first().map(|m| m.subject.clone()).unwrap_or_default();
    let summary = format!(
        "Thread with {} messages from {} participants. Latest: {}",
        messages.len(),
        participants.len(),
        messages.last().map(|m| m.body_md.chars().take(100).collect::<String>()).unwrap_or_default()
    );

    Ok(Json(SummarizeThreadResponse {
        thread_id: payload.thread_id,
        message_count: messages.len(),
        participants,
        subject,
        summary,
    }).into_response())
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

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;
    let threads = lib_core::model::message::MessageBmc::list_threads(&ctx, mm, project.id, payload.limit).await?;

    let summaries: Vec<ThreadSummaryBrief> = threads.into_iter().map(|t| ThreadSummaryBrief {
        thread_id: t.thread_id,
        subject: t.subject,
        message_count: t.message_count,
        last_message_ts: t.last_message_ts,
    }).collect();

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
    let _project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;

    let target_path = std::path::PathBuf::from(&payload.target_repo_path);
    let hooks_dir = target_path.join(".git").join("hooks");
    let hook_path = hooks_dir.join("pre-commit");

    // Create the pre-commit hook script
    let hook_script = format!(r#"#!/bin/sh
# MCP Agent Mail Pre-commit Guard
# Installed for project: {}

# Check for file reservation conflicts
# This is an advisory check - can be bypassed with AGENT_MAIL_BYPASS=1

if [ -n "$AGENT_MAIL_BYPASS" ]; then
    echo "MCP Agent Mail: Bypass enabled, skipping reservation check"
    exit 0
fi

# TODO: Call the API to check for conflicts
# For now, this is a placeholder that always passes
echo "MCP Agent Mail: Pre-commit guard active"
exit 0
"#, payload.project_slug);

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
        message: format!("Pre-commit guard installed for project '{}'", payload.project_slug),
    }).into_response())
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
            }).into_response());
        } else {
            return Ok(Json(UninstallPrecommitGuardResponse {
                uninstalled: false,
                message: "Pre-commit hook exists but is not an MCP Agent Mail guard".to_string(),
            }).into_response());
        }
    }

    Ok(Json(UninstallPrecommitGuardResponse {
        uninstalled: false,
        message: "No pre-commit hook found".to_string(),
    }).into_response())
}

// --- add_attachment ---
// Note: Attachments are stored as references in the message attachments JSON field
#[derive(Deserialize)]
pub struct AddAttachmentPayload {
    pub project_slug: String,
    pub message_id: i64,
    pub filename: String,
    pub content_base64: String,
    pub mime_type: Option<String>,
}

#[derive(Serialize)]
pub struct AddAttachmentResponse {
    pub added: bool,
    pub attachment_id: String,
    pub message: String,
}

pub async fn add_attachment(
    State(app_state): State<AppState>,
    Json(payload): Json<AddAttachmentPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;

    // Verify message exists
    let _message = lib_core::model::message::MessageBmc::get(&ctx, mm, payload.message_id).await?;

    // Generate attachment ID
    let attachment_id = format!("att_{}_{}", payload.message_id, uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0"));

    // Store attachment in Git
    let repo = lib_core::store::git_store::open_repo(&mm.repo_root)?;
    let attachment_path = std::path::PathBuf::from("projects")
        .join(&project.slug)
        .join("attachments")
        .join(&attachment_id)
        .join(&payload.filename);

    // Decode base64 content
    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD
        .decode(&payload.content_base64)
        .map_err(|e| lib_core::Error::InvalidInput(format!("Invalid base64: {}", e)))?;

    lib_core::store::git_store::commit_file(
        &repo,
        &attachment_path,
        &String::from_utf8_lossy(&content),
        &format!("attachment: {} for message {}", payload.filename, payload.message_id),
        "mcp-bot",
        "mcp-bot@localhost",
    )?;

    Ok(Json(AddAttachmentResponse {
        added: true,
        attachment_id: attachment_id.clone(),
        message: format!("Attachment '{}' added with ID {}", payload.filename, attachment_id),
    }).into_response())
}

// --- get_attachment ---
#[derive(Deserialize)]
pub struct GetAttachmentPayload {
    pub project_slug: String,
    pub attachment_id: String,
    pub filename: String,
}

#[derive(Serialize)]
pub struct GetAttachmentResponse {
    pub found: bool,
    pub filename: String,
    pub content_base64: Option<String>,
    pub mime_type: Option<String>,
}

pub async fn get_attachment(
    State(app_state): State<AppState>,
    Json(payload): Json<GetAttachmentPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &app_state.mm;

    let project = lib_core::model::project::ProjectBmc::get_by_slug(&ctx, mm, &payload.project_slug).await?;

    let repo = lib_core::store::git_store::open_repo(&mm.repo_root)?;
    let attachment_path = std::path::PathBuf::from("projects")
        .join(&project.slug)
        .join("attachments")
        .join(&payload.attachment_id)
        .join(&payload.filename);

    match lib_core::store::git_store::read_file_content(&repo, &attachment_path) {
        Ok(content) => {
            use base64::Engine;
            let content_base64 = base64::engine::general_purpose::STANDARD.encode(content.as_bytes());

            // Guess mime type from extension
            let mime_type = match payload.filename.rsplit('.').next() {
                Some("txt") => Some("text/plain".to_string()),
                Some("json") => Some("application/json".to_string()),
                Some("md") => Some("text/markdown".to_string()),
                Some("png") => Some("image/png".to_string()),
                Some("jpg") | Some("jpeg") => Some("image/jpeg".to_string()),
                Some("pdf") => Some("application/pdf".to_string()),
                _ => None,
            };

            Ok(Json(GetAttachmentResponse {
                found: true,
                filename: payload.filename,
                content_base64: Some(content_base64),
                mime_type,
            }).into_response())
        }
        Err(_) => {
            Ok(Json(GetAttachmentResponse {
                found: false,
                filename: payload.filename,
                content_base64: None,
                mime_type: None,
            }).into_response())
        }
    }
}
