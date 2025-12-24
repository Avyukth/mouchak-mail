//! Macro tool implementations
//!
//! Handles macro definitions, workflow macros, and convenience macros.

use lib_core::{
    ctx::Ctx,
    model::{
        ModelManager,
        agent::{AgentBmc, AgentForCreate},
        agent_link::{AgentLinkBmc, AgentLinkForCreate},
        file_reservation::{FileReservationBmc, FileReservationForCreate},
        macro_def::{MacroDefBmc, MacroDefForCreate},
        message::{MessageBmc, MessageForCreate},
        project::ProjectBmc,
    },
};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use std::sync::Arc;

use super::helpers;
use super::{
    InvokeMacroParams, ListBuiltinWorkflowsParams, ListMacrosParams, MacroContactHandshakeParams,
    MacroFileReservationCycleParams, MacroPrepareThreadParams, MacroStartSessionParams,
    QuickHandoffWorkflowParams, QuickReviewWorkflowParams, QuickStandupWorkflowParams,
    RegisterMacroParams, UnregisterMacroParams,
};

/// List all available macros in a project.
pub async fn list_macros_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: ListMacrosParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let macros = MacroDefBmc::list(ctx, mm, project.id.get())
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut output = format!(
        "Macros in '{}' ({}):\n\n",
        params.project_slug,
        macros.len()
    );
    for m in &macros {
        output.push_str(&format!(
            "- {} ({} steps): {}\n",
            m.name,
            m.steps.len(),
            m.description
        ));
    }
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Register a new macro definition.
pub async fn register_macro_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: RegisterMacroParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let macro_c = MacroDefForCreate {
        project_id: project.id.get(),
        name: params.name.clone(),
        description: params.description,
        steps: params.steps,
    };

    let macro_id = MacroDefBmc::create(ctx, mm, macro_c)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!("Registered macro '{}' with id {}", params.name, macro_id);
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Remove a macro definition.
pub async fn unregister_macro_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: UnregisterMacroParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let deleted = MacroDefBmc::delete(ctx, mm, project.id.get(), &params.name)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = if deleted {
        format!("Unregistered macro '{}'", params.name)
    } else {
        format!("Macro '{}' not found", params.name)
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Execute a pre-defined macro and get its steps.
pub async fn invoke_macro_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: InvokeMacroParams,
) -> Result<CallToolResult, McpError> {
    let project = helpers::resolve_project(ctx, mm, &params.project_slug).await?;

    let macro_def = MacroDefBmc::get_by_name(ctx, mm, project.id.get(), &params.name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Macro not found: {}", e), None))?;

    let steps_json =
        serde_json::to_string_pretty(&macro_def.steps).unwrap_or_else(|_| "[]".to_string());
    let output = format!(
        "Macro '{}' ({} steps)\nDescription: {}\n\nSteps:\n{}",
        macro_def.name,
        macro_def.steps.len(),
        macro_def.description,
        steps_json
    );
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// List the 5 built-in workflow macros available in all projects.
pub async fn list_builtin_workflows_impl(
    _ctx: &Ctx,
    _mm: &Arc<ModelManager>,
    _params: ListBuiltinWorkflowsParams,
) -> Result<CallToolResult, McpError> {
    let workflows = vec![
        ("start_session", "Register agent and check inbox"),
        ("prepare_thread", "Create thread and reserve files"),
        ("file_reservation_cycle", "Reserve, work, release files"),
        ("contact_handshake", "Establish cross-project contact"),
        ("broadcast_message", "Send to multiple agents"),
    ];

    let mut output = String::from("Built-in Workflows:\n\n");
    for (name, desc) in workflows {
        output.push_str(&format!("- {}: {}\n", name, desc));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Broadcast standup request to all agents in a project.
pub async fn quick_standup_workflow_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: QuickStandupWorkflowParams,
) -> Result<CallToolResult, McpError> {
    let (project, sender) =
        helpers::resolve_project_and_agent(ctx, mm, &params.project_slug, &params.sender_name)
            .await?;

    let agents = AgentBmc::list_all_for_project(ctx, mm, project.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let recipient_ids: Vec<i64> = agents.iter().map(|a| a.id.get()).collect();

    let question = params
        .standup_question
        .unwrap_or_else(|| "What are you working on today?".to_string());

    let msg_c = MessageForCreate {
        project_id: project.id.get(),
        sender_id: sender.id.get(),
        recipient_ids,
        cc_ids: None,
        bcc_ids: None,
        subject: "Daily Standup".to_string(),
        body_md: question,
        thread_id: Some("STANDUP".to_string()),
        importance: Some("normal".to_string()),
        ack_required: false,
    };

    let msg_id = MessageBmc::create(ctx, mm, msg_c)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Standup request sent to {} agents (message id: {})",
        agents.len(),
        msg_id
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Facilitate task handoff from one agent to another.
pub async fn quick_handoff_workflow_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: QuickHandoffWorkflowParams,
) -> Result<CallToolResult, McpError> {
    let (project, from_agent) =
        helpers::resolve_project_and_agent(ctx, mm, &params.project_slug, &params.from_agent)
            .await?;

    let to_agent = helpers::resolve_agent(ctx, mm, project.id.get(), &params.to_agent).await?;

    let files_text = if let Some(files) = &params.files {
        format!("\n\nFiles:\n{}", files.join("\n"))
    } else {
        String::new()
    };

    let msg_c = MessageForCreate {
        project_id: project.id.get(),
        sender_id: from_agent.id.get(),
        recipient_ids: vec![to_agent.id.get()],
        cc_ids: None,
        bcc_ids: None,
        subject: format!("Task Handoff: {}", params.task_description),
        body_md: format!("Taking over: {}{}", params.task_description, files_text),
        thread_id: Some(format!(
            "HANDOFF-{}",
            params.task_description.replace(" ", "-")
        )),
        importance: Some("high".to_string()),
        ack_required: true, // Handoffs should be acknowledged
    };

    let msg_id = MessageBmc::create(ctx, mm, msg_c)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Handoff message sent from '{}' to '{}' (id: {})",
        params.from_agent, params.to_agent, msg_id
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Initiate code review process with file reservations.
pub async fn quick_review_workflow_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: QuickReviewWorkflowParams,
) -> Result<CallToolResult, McpError> {
    let (project, requester) =
        helpers::resolve_project_and_agent(ctx, mm, &params.project_slug, &params.requester)
            .await?;

    let reviewer = helpers::resolve_agent(ctx, mm, project.id.get(), &params.reviewer).await?;

    // Reserve files for review (non-exclusive)
    let expires_ts = chrono::Utc::now().naive_utc() + chrono::Duration::hours(2);
    for file in &params.files_to_review {
        let res_c = FileReservationForCreate {
            project_id: project.id,
            agent_id: reviewer.id,
            path_pattern: file.clone(),
            exclusive: false,
            reason: "Code review".to_string(),
            expires_ts,
        };
        FileReservationBmc::create(ctx, mm, res_c)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    }

    let msg_c = MessageForCreate {
        project_id: project.id.get(),
        sender_id: requester.id.get(),
        recipient_ids: vec![reviewer.id.get()],
        cc_ids: None,
        bcc_ids: None,
        subject: "Code Review Request".to_string(),
        body_md: format!(
            "Please review:\n{}\n\nFiles:\n{}",
            params.description,
            params.files_to_review.join("\n")
        ),
        thread_id: Some("CODE-REVIEW".to_string()),
        importance: Some("normal".to_string()),
        ack_required: true, // Review requests should be acknowledged
    };

    let msg_id = MessageBmc::create(ctx, mm, msg_c)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let msg = format!(
        "Review request sent to '{}'. Reserved {} files for review (id: {})",
        params.reviewer,
        params.files_to_review.len(),
        msg_id
    );
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Boot a project session in one call.
pub async fn macro_start_session_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: MacroStartSessionParams,
) -> Result<CallToolResult, McpError> {
    // Ensure project exists
    let project = match ProjectBmc::get_by_identifier(ctx, mm, &params.human_key).await {
        Ok(proj) => proj,
        Err(_) => {
            // Create project with human_key as both slug and human_key
            let slug = params
                .human_key
                .to_lowercase()
                .replace(|c: char| !c.is_alphanumeric() && c != '-', "-");
            ProjectBmc::create(ctx, mm, &slug, &params.human_key)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            ProjectBmc::get_by_identifier(ctx, mm, &slug)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
    };

    // Get or create agent
    let agent_name = params
        .agent_name
        .unwrap_or_else(|| format!("{}-{}", params.program, &params.model.replace(".", "-")));
    let agent = match AgentBmc::get_by_name(ctx, mm, project.id, &agent_name).await {
        Ok(a) => a,
        Err(_) => {
            let agent_c = AgentForCreate {
                project_id: project.id,
                name: agent_name.clone(),
                program: params.program.clone(),
                model: params.model.clone(),
                task_description: params.task_description.clone(),
            };
            let id = AgentBmc::create(ctx, mm, agent_c)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            AgentBmc::get(ctx, mm, id)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
    };

    // Reserve files if requested
    let mut granted_reservations = Vec::new();
    let mut reservation_conflicts = Vec::new();
    if let Some(paths) = params.file_reservation_paths {
        let now = chrono::Utc::now().naive_utc();
        let expires_ts = now + chrono::Duration::seconds(params.file_reservation_ttl_seconds);

        let active_reservations = FileReservationBmc::list_active_for_project(ctx, mm, project.id)
            .await
            .unwrap_or_default();

        for path in paths {
            // Check for conflicts
            for res in &active_reservations {
                if res.agent_id != agent.id
                    && res.exclusive
                    && lib_core::utils::pathspec::paths_conflict(&res.path_pattern, &path)
                {
                    reservation_conflicts.push(format!(
                        "{} conflicts with {} (agent ID {})",
                        path, res.path_pattern, res.agent_id
                    ));
                }
            }

            // Grant reservation (advisory model)
            let fr_c = FileReservationForCreate {
                project_id: project.id,
                agent_id: agent.id,
                path_pattern: path.clone(),
                exclusive: true,
                reason: params.file_reservation_reason.clone(),
                expires_ts,
            };
            if let Ok(id) = FileReservationBmc::create(ctx, mm, fr_c).await {
                granted_reservations.push(serde_json::json!({
                    "path": path,
                    "id": id,
                    "expires_ts": expires_ts.to_string()
                }));
            }
        }
    }

    // Fetch inbox
    let inbox_messages = MessageBmc::list_inbox_for_agent(
        ctx,
        mm,
        project.id.get(),
        agent.id.get(),
        params.inbox_limit,
    )
    .await
    .unwrap_or_default();

    let inbox_items: Vec<serde_json::Value> = inbox_messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "subject": m.subject,
                "sender_name": m.sender_name,
                "created_ts": m.created_ts.to_string(),
                "importance": m.importance,
                "thread_id": m.thread_id,
            })
        })
        .collect();

    let result = serde_json::json!({
        "project": {
            "id": project.id,
            "slug": project.slug,
            "human_key": project.human_key,
        },
        "agent": {
            "id": agent.id,
            "name": agent.name,
            "program": agent.program,
            "model": agent.model,
        },
        "file_reservations": {
            "granted": granted_reservations,
            "conflicts": reservation_conflicts,
        },
        "inbox": inbox_items,
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
    )]))
}

/// Align an agent with an existing thread.
pub async fn macro_prepare_thread_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: MacroPrepareThreadParams,
) -> Result<CallToolResult, McpError> {
    let project = ProjectBmc::get_by_identifier(ctx, mm, &params.project_key)
        .await
        .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

    // Get or create agent
    let agent_name = params
        .agent_name
        .unwrap_or_else(|| format!("{}-{}", params.program, &params.model.replace(".", "-")));
    let agent = if params.register_if_missing {
        match AgentBmc::get_by_name(ctx, mm, project.id, &agent_name).await {
            Ok(a) => a,
            Err(_) => {
                let agent_c = AgentForCreate {
                    project_id: project.id,
                    name: agent_name.clone(),
                    program: params.program.clone(),
                    model: params.model.clone(),
                    task_description: params.task_description.clone(),
                };
                let id = AgentBmc::create(ctx, mm, agent_c)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                AgentBmc::get(ctx, mm, id)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
        }
    } else {
        AgentBmc::get_by_name(ctx, mm, project.id, &agent_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?
    };

    // Get thread messages
    let thread_messages = MessageBmc::list_by_thread(ctx, mm, project.id.get(), &params.thread_id)
        .await
        .unwrap_or_default();

    // Compute thread summary
    let total_messages = thread_messages.len();
    let participants: std::collections::HashSet<String> = thread_messages
        .iter()
        .map(|m| m.sender_name.clone())
        .collect();
    let first_subject = thread_messages.first().map(|m| m.subject.clone());
    let last_activity = thread_messages.last().map(|m| m.created_ts.to_string());

    let examples: Vec<serde_json::Value> = if params.include_examples {
        thread_messages
            .iter()
            .take(3)
            .map(|m| {
                serde_json::json!({
                    "sender": m.sender_name,
                    "subject": m.subject,
                    "body_preview": m.body_md.chars().take(100).collect::<String>(),
                    "created_ts": m.created_ts,
                })
            })
            .collect()
    } else {
        vec![]
    };

    // Fetch inbox
    let inbox_messages = MessageBmc::list_inbox_for_agent(
        ctx,
        mm,
        project.id.get(),
        agent.id.get(),
        params.inbox_limit,
    )
    .await
    .unwrap_or_default();

    let inbox_items: Vec<serde_json::Value> = inbox_messages
        .iter()
        .map(|m| {
            let mut item = serde_json::json!({
                "id": m.id,
                "subject": m.subject,
                "sender_name": m.sender_name,
                "created_ts": m.created_ts.to_string(),
                "importance": m.importance,
                "thread_id": m.thread_id,
            });
            if params.include_inbox_bodies {
                item["body_md"] = serde_json::json!(m.body_md);
            }
            item
        })
        .collect();

    let result = serde_json::json!({
        "project": {
            "id": project.id,
            "slug": project.slug,
            "human_key": project.human_key,
        },
        "agent": {
            "id": agent.id,
            "name": agent.name,
            "program": agent.program,
            "model": agent.model,
        },
        "thread": {
            "thread_id": params.thread_id,
            "total_messages": total_messages,
            "participants": participants.into_iter().collect::<Vec<_>>(),
            "subject": first_subject,
            "last_activity": last_activity,
            "examples": examples,
        },
        "inbox": inbox_items,
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
    )]))
}

/// Reserve file paths for exclusive editing with optional auto-release.
pub async fn macro_file_reservation_cycle_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: MacroFileReservationCycleParams,
) -> Result<CallToolResult, McpError> {
    let project = ProjectBmc::get_by_identifier(ctx, mm, &params.project_key)
        .await
        .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

    let agent = AgentBmc::get_by_name(ctx, mm, project.id, &params.agent_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

    let now = chrono::Utc::now().naive_utc();
    let expires_ts = now + chrono::Duration::seconds(params.ttl_seconds);

    let active_reservations = FileReservationBmc::list_active_for_project(ctx, mm, project.id)
        .await
        .unwrap_or_default();

    let mut granted = Vec::new();
    let mut conflicts = Vec::new();
    let mut reservation_ids = Vec::new();

    for path in &params.paths {
        // Check for conflicts
        for res in &active_reservations {
            if res.agent_id != agent.id
                && (res.exclusive || params.exclusive)
                && lib_core::utils::pathspec::paths_conflict(&res.path_pattern, path)
            {
                conflicts.push(serde_json::json!({
                    "path": path,
                    "conflicts_with": res.path_pattern,
                    "held_by_agent_id": res.agent_id,
                    "expires": res.expires_ts.to_string(),
                }));
            }
        }

        // Grant reservation
        let fr_c = FileReservationForCreate {
            project_id: project.id,
            agent_id: agent.id,
            path_pattern: path.clone(),
            exclusive: params.exclusive,
            reason: params.reason.clone(),
            expires_ts,
        };
        match FileReservationBmc::create(ctx, mm, fr_c).await {
            Ok(id) => {
                reservation_ids.push(id);
                granted.push(serde_json::json!({
                    "path": path,
                    "id": id,
                    "expires_ts": expires_ts.to_string(),
                }));
            }
            Err(e) => {
                conflicts.push(serde_json::json!({
                    "path": path,
                    "error": e.to_string(),
                }));
            }
        }
    }

    // Auto-release if requested
    let mut released = Vec::new();
    if params.auto_release {
        for id in reservation_ids {
            if FileReservationBmc::release(ctx, mm, id).await.is_ok() {
                released.push(id);
            }
        }
    }

    let result = serde_json::json!({
        "file_reservations": {
            "granted": granted,
            "conflicts": conflicts,
        },
        "released": if params.auto_release { Some(released) } else { None },
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
    )]))
}

/// Request contact permission with optional auto-accept and welcome message.
pub async fn macro_contact_handshake_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    params: MacroContactHandshakeParams,
) -> Result<CallToolResult, McpError> {
    // Resolve aliases
    let requester_name = params.requester.or(params.agent_name).ok_or_else(|| {
        McpError::invalid_params("requester or agent_name is required".to_string(), None)
    })?;
    let target_name = params.target.or(params.to_agent).ok_or_else(|| {
        McpError::invalid_params("target or to_agent is required".to_string(), None)
    })?;

    let project = ProjectBmc::get_by_identifier(ctx, mm, &params.project_key)
        .await
        .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;

    // Get or create requester agent
    let requester = if params.register_if_missing {
        match AgentBmc::get_by_name(ctx, mm, project.id, &requester_name).await {
            Ok(a) => a,
            Err(_) => {
                let program = params
                    .program
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
                let model = params
                    .model
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
                let agent_c = AgentForCreate {
                    project_id: project.id,
                    name: requester_name.clone(),
                    program,
                    model,
                    task_description: String::new(),
                };
                let id = AgentBmc::create(ctx, mm, agent_c)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                AgentBmc::get(ctx, mm, id)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
        }
    } else {
        AgentBmc::get_by_name(ctx, mm, project.id, &requester_name)
            .await
            .map_err(|e| McpError::invalid_params(format!("Requester not found: {}", e), None))?
    };

    // Get target agent (must exist)
    let target = AgentBmc::get_by_name(ctx, mm, project.id, &target_name)
        .await
        .map_err(|e| McpError::invalid_params(format!("Target agent not found: {}", e), None))?;

    // Create contact request using AgentLinkBmc
    let link_c = AgentLinkForCreate {
        a_project_id: project.id.get(),
        a_agent_id: requester.id.get(),
        b_project_id: project.id.get(),
        b_agent_id: target.id.get(),
        reason: params.reason.clone(),
    };
    let link_id = AgentLinkBmc::request_contact(ctx, mm, link_c)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let request_result = serde_json::json!({
        "link_id": link_id,
        "from_agent": requester.name,
        "to_agent": target.name,
        "status": "pending",
    });

    // Auto-accept if requested
    let response_result = if params.auto_accept {
        AgentLinkBmc::respond_contact(ctx, mm, link_id, true)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Some(serde_json::json!({
            "link_id": link_id,
            "status": "accepted",
        }))
    } else {
        None
    };

    // Send welcome message if provided
    let welcome_result =
        if let (Some(subject), Some(body)) = (params.welcome_subject, params.welcome_body) {
            let msg_c = MessageForCreate {
                project_id: project.id.get(),
                sender_id: requester.id.get(),
                recipient_ids: vec![target.id.get()],
                cc_ids: None,
                bcc_ids: None,
                subject,
                body_md: body,
                thread_id: params.thread_id,
                importance: Some("normal".to_string()),
                ack_required: false,
            };
            match MessageBmc::create(ctx, mm, msg_c).await {
                Ok(msg_id) => Some(serde_json::json!({
                    "message_id": msg_id,
                    "sent": true,
                })),
                Err(e) => Some(serde_json::json!({
                    "sent": false,
                    "error": e.to_string(),
                })),
            }
        } else {
            None
        };

    let result = serde_json::json!({
        "request": request_result,
        "response": response_result,
        "welcome_message": welcome_result,
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
    )]))
}
