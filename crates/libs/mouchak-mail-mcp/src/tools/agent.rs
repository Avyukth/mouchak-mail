//! Agent management tool implementations
//!
//! Handles agent registration, lookup, and profile management.

use mouchak_mail_core::{
    ctx::Ctx,
    model::{
        ModelManager,
        agent::{AgentBmc, AgentForCreate, AgentProfileUpdate},
        agent_capabilities::AgentCapabilityBmc,
        file_reservation::FileReservationBmc,
    },
    utils::mistake_detection::detect_unix_username_as_agent,
    utils::validation::{validate_agent_name, validate_project_key},
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{
    CreateAgentIdentityParams, GetAgentProfileParams, ListAgentsParams, RegisterAgentParams,
    UpdateAgentProfileParams, WhoisParams,
};

/// Register an agent in a project.
pub async fn register_agent_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: RegisterAgentParams,
) -> Result<CallToolResult, McpError> {
    // Validate inputs
    validate_project_key(&params.project_slug).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    validate_agent_name(&params.name).map_err(|e| {
        McpError::invalid_params(
            format!("{}", e),
            Some(serde_json::json!({ "details": e.context() })),
        )
    })?;

    // Get project
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    // Check if agent exists
    match AgentBmc::get_by_name(ctx, mm, project.id, &params.name).await {
        Ok(agent) => {
            let msg = format!(
                "Agent '{}' already exists (id: {}, program: {})",
                agent.name, agent.id, agent.program
            );
            Ok(CallToolResult::success(vec![Content::text(msg)]))
        }
        Err(_) => {
            let agent_c = AgentForCreate {
                project_id: project.id,
                name: params.name.clone(),
                program: params.program,
                model: params.model,
                task_description: params.task_description,
            };

            let id = AgentBmc::create(ctx, mm, agent_c)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            AgentCapabilityBmc::grant_defaults(ctx, mm, id.get())
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let mut msg = format!(
                "Registered agent '{}' with id {} (granted default capabilities)",
                params.name,
                id.get()
            );

            if let Some(hint) = detect_unix_username_as_agent(&params.name) {
                msg.push_str(&format!("\n\nHint: {}", hint.suggestion));
            }

            Ok(CallToolResult::success(vec![Content::text(msg)]))
        }
    }
}

/// Get information about an agent.
pub async fn whois_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: WhoisParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let output = format!(
        "Agent: {}\nID: {}\nProgram: {}\nModel: {}\nTask: {}\nContact Policy: {}\nAttachments Policy: {}",
        agent.name,
        agent.id,
        agent.program,
        agent.model,
        agent.task_description,
        agent.contact_policy,
        agent.attachments_policy
    );

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Update an agent's profile settings.
pub async fn update_agent_profile_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: UpdateAgentProfileParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let update = AgentProfileUpdate {
        task_description: params.task_description,
        attachments_policy: params.attachments_policy,
        contact_policy: params.contact_policy,
    };

    AgentBmc::update_profile(ctx, mm, agent.id, update)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!("Updated profile for agent '{}'", params.agent_name);
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Get detailed profile information for an agent.
pub async fn get_agent_profile_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: GetAgentProfileParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let sent_count = AgentBmc::count_messages_sent(ctx, mm, agent.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let received_count = AgentBmc::count_messages_received(ctx, mm, agent.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let reservations = FileReservationBmc::list_active_for_project(ctx, mm, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let active_reservations = reservations
        .iter()
        .filter(|r| r.agent_id == agent.id)
        .count();

    let output = format!(
        "Agent: {}\nID: {}\nProgram: {}\nModel: {}\nTask: {}\nContact Policy: {}\nAttachments Policy: {}\nMessages Sent: {}\nMessages Received: {}\nActive Reservations: {}\nInception: {}\nLast Active: {}",
        agent.name,
        agent.id,
        agent.program,
        agent.model,
        agent.task_description,
        agent.contact_policy,
        agent.attachments_policy,
        sent_count,
        received_count,
        active_reservations,
        agent.inception_ts,
        agent.last_active_ts
    );
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// List all agents registered in a project.
pub async fn list_agents_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListAgentsParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let agents = AgentBmc::list_all_for_project(ctx, mm, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!(
        "Agents in '{}' ({}):\n\n",
        params.project_slug,
        agents.len()
    );
    for a in &agents {
        output.push_str(&format!(
            "- {} (program: {}, model: {})\n  Task: {}\n",
            a.name, a.program, a.model, a.task_description
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

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

pub async fn create_agent_identity_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: CreateAgentIdentityParams,
) -> Result<CallToolResult, McpError> {
    use std::collections::HashSet;

    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let existing_agents = AgentBmc::list_all_for_project(ctx, mm, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let existing_names: HashSet<String> = existing_agents.iter().map(|a| a.name.clone()).collect();

    let mut alternatives = generate_random_names(&existing_names, 5);

    if let Some(hint) = &params.hint {
        if let Some(hint_name) = find_hint_match(hint, &existing_names, &alternatives) {
            alternatives.insert(0, hint_name);
        }
    }

    let suggested_name = alternatives
        .first()
        .cloned()
        .unwrap_or_else(|| "Agent1".to_string());

    let output = format!(
        "Suggested name: {}\nAlternatives: {}",
        suggested_name,
        alternatives.join(", ")
    );

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
