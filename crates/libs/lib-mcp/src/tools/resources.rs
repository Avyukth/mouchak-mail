//! MCP Resource implementations for Agent Mail

use lib_core::{
    ctx::Ctx,
    model::{
        ModelManager, agent::AgentBmc, file_reservation::FileReservationBmc, message::MessageBmc,
        product::ProductBmc, project::ProjectBmc,
    },
};
use rmcp::{
    ErrorData as McpError,
    model::{
        ListResourcesResult, PaginatedRequestParam, RawResource, ReadResourceRequestParam,
        ReadResourceResult, Resource, ResourceContents,
    },
};
use std::sync::Arc;

#[derive(serde::Serialize)]
struct ResourceMessage<'a> {
    id: i64,
    sender_name: &'a str,
    subject: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    body_md: Option<&'a str>,
    thread_id: Option<&'a String>,
    importance: &'a str,
    created_ts: chrono::NaiveDateTime,
}

pub async fn read_resource_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    request: ReadResourceRequestParam,
) -> Result<ReadResourceResult, McpError> {
    let uri_str = request.uri;
    let uri = url::Url::parse(&uri_str)
        .map_err(|e| McpError::invalid_params(format!("Invalid URI: {}", e), None))?;

    if uri.scheme() != "agent-mail" && uri.scheme() != "resource" {
        return Err(McpError::invalid_params(
            "URI scheme must be 'agent-mail' or 'resource'".to_string(),
            None,
        ));
    }

    let query: std::collections::HashMap<_, _> = uri.query_pairs().into_owned().collect();
    let project_slug_param = query.get("project");
    let limit = query
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(20);
    let include_bodies = query
        .get("include_bodies")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let (project_slug, resource_type, resource_id) = if uri.scheme() == "agent-mail" {
        let host = uri.host_str().ok_or(McpError::invalid_params(
            "URI missing host (project slug)".to_string(),
            None,
        ))?;
        let segments: Vec<&str> = uri
            .path_segments()
            .ok_or(McpError::invalid_params(
                "Invalid URI path".to_string(),
                None,
            ))?
            .collect();
        if segments.is_empty() {
            return Err(McpError::invalid_params(
                "URI path missing resource type".to_string(),
                None,
            ));
        }
        (host, segments[0], segments.get(1).cloned())
    } else {
        let resource_type = uri.host_str().ok_or(McpError::invalid_params(
            "URI missing resource type".to_string(),
            None,
        ))?;
        let segments: Vec<&str> = uri
            .path_segments()
            .ok_or(McpError::invalid_params(
                "Invalid URI path".to_string(),
                None,
            ))?
            .collect();
        let resource_id = segments.first().cloned();
        let slug = project_slug_param.map(|s| s.as_str()).unwrap_or("");
        (slug, resource_type, resource_id)
    };

    if resource_type == "identity" {
        let path = uri.path();
        if path.is_empty() {
            return Err(McpError::invalid_params(
                "Missing identity path".to_string(),
                None,
            ));
        }
        let data = serde_json::json!({
            "path": path,
            "type": "repository",
            "identity": format!("repo-{}", path.replace("/", "-").trim_start_matches('-')),
        });
        return Ok(ReadResourceResult {
            contents: vec![ResourceContents::TextResourceContents {
                uri: uri_str,
                mime_type: Some("application/json".to_string()),
                text: serde_json::to_string_pretty(&data)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?,
                meta: None,
            }],
        });
    }

    if resource_type == "product" {
        let product_uid = resource_id.ok_or(McpError::invalid_params(
            "Missing product UID".to_string(),
            None,
        ))?;
        let product = ProductBmc::get_by_uid(ctx, mm, product_uid)
            .await
            .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;
        return Ok(ReadResourceResult {
            contents: vec![ResourceContents::TextResourceContents {
                uri: uri_str,
                mime_type: Some("application/json".to_string()),
                text: serde_json::to_string_pretty(&product)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?,
                meta: None,
            }],
        });
    }

    let project = ProjectBmc::get_by_slug(ctx, mm, project_slug)
        .await
        .map_err(|e| McpError::invalid_params(format!("Project not found: {}", e), None))?;
    let project_id = project.id;

    let content = match resource_type {
        "agents" | "agent" => {
            if let Some(agent_name) = resource_id {
                let agent = AgentBmc::get_by_name(ctx, mm, project_id, agent_name)
                    .await
                    .map_err(|e| {
                        McpError::invalid_params(format!("Agent not found: {}", e), None)
                    })?;
                serde_json::to_string_pretty(&agent)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            } else {
                let agents = AgentBmc::list_all_for_project(ctx, mm, project_id)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                serde_json::to_string_pretty(&agents)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
        }
        "file_reservations" => {
            let reservations = FileReservationBmc::list_active_for_project(ctx, mm, project_id)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            serde_json::to_string_pretty(&reservations)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        "inbox" => {
            let agent_name = resource_id.ok_or(McpError::invalid_params(
                "Missing agent name".to_string(),
                None,
            ))?;
            let agent = AgentBmc::get_by_name(ctx, mm, project_id, agent_name)
                .await
                .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

            let messages = MessageBmc::list_inbox_for_agent(ctx, mm, project_id, agent.id, limit)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let resource_messages: Vec<_> = messages
                .iter()
                .map(|m| ResourceMessage {
                    id: m.id,
                    sender_name: &m.sender_name,
                    subject: &m.subject,
                    body_md: if include_bodies {
                        Some(&m.body_md)
                    } else {
                        None
                    },
                    thread_id: m.thread_id.as_ref(),
                    importance: &m.importance,
                    created_ts: m.created_ts,
                })
                .collect();

            serde_json::to_string_pretty(&resource_messages)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        "outbox" => {
            let agent_name = resource_id.ok_or(McpError::invalid_params(
                "Missing agent name".to_string(),
                None,
            ))?;
            let agent = AgentBmc::get_by_name(ctx, mm, project_id, agent_name)
                .await
                .map_err(|e| McpError::invalid_params(format!("Agent not found: {}", e), None))?;

            let messages = MessageBmc::list_outbox_for_agent(ctx, mm, project_id, agent.id, limit)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let resource_messages: Vec<_> = messages
                .iter()
                .map(|m| ResourceMessage {
                    id: m.id,
                    sender_name: &m.sender_name,
                    subject: &m.subject,
                    body_md: if include_bodies {
                        Some(&m.body_md)
                    } else {
                        None
                    },
                    thread_id: m.thread_id.as_ref(),
                    importance: &m.importance,
                    created_ts: m.created_ts,
                })
                .collect();

            serde_json::to_string_pretty(&resource_messages)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        "thread" => {
            let thread_id_str = resource_id.ok_or(McpError::invalid_params(
                "Missing thread ID".to_string(),
                None,
            ))?;
            let messages = MessageBmc::list_by_thread(ctx, mm, project_id, thread_id_str)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let resource_messages: Vec<_> = messages
                .iter()
                .map(|m| ResourceMessage {
                    id: m.id,
                    sender_name: &m.sender_name,
                    subject: &m.subject,
                    body_md: if include_bodies {
                        Some(&m.body_md)
                    } else {
                        None
                    },
                    thread_id: m.thread_id.as_ref(),
                    importance: &m.importance,
                    created_ts: m.created_ts,
                })
                .collect();

            serde_json::to_string_pretty(&resource_messages)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        "threads" => {
            let threads = MessageBmc::list_threads(ctx, mm, project_id, limit)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            serde_json::to_string_pretty(&threads)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        "product" => {
            let product_uid = resource_id.ok_or(McpError::invalid_params(
                "Missing product UID".to_string(),
                None,
            ))?;
            let product = ProductBmc::get_by_uid(ctx, mm, product_uid)
                .await
                .map_err(|e| McpError::invalid_params(format!("Product not found: {}", e), None))?;
            serde_json::to_string_pretty(&product)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        }
        _ => {
            return Err(McpError::invalid_params(
                format!("Unknown resource type: {}", resource_type),
                None,
            ));
        }
    };

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri_str,
            mime_type: Some("application/json".to_string()),
            text: content,
            meta: None,
        }],
    })
}

pub async fn list_resources_impl(
    ctx: &Ctx,
    mm: &Arc<ModelManager>,
    _request: Option<PaginatedRequestParam>,
) -> Result<ListResourcesResult, McpError> {
    let projects = ProjectBmc::list_all(ctx, mm)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut resources = Vec::new();

    for project in projects {
        let slug = &project.slug;

        resources.push(Resource {
            raw: RawResource {
                uri: format!("agent-mail://{}/agents", slug),
                name: format!("Agents ({})", slug),
                description: Some(format!("List of all agents in project '{}'", slug)),
                mime_type: Some("application/json".to_string()),
                size: None,
                icons: None,
                meta: None,
                title: None,
            },
            annotations: None,
        });

        let project_agents = AgentBmc::list_all_for_project(ctx, mm, project.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        for agent in project_agents {
            resources.push(Resource {
                raw: RawResource {
                    uri: format!("agent-mail://{}/inbox/{}", slug, agent.name),
                    name: format!("Inbox: {} ({})", agent.name, slug),
                    description: Some(format!("Inbox for agent '{}'", agent.name)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                    title: None,
                },
                annotations: None,
            });
            resources.push(Resource {
                raw: RawResource {
                    uri: format!("agent-mail://{}/outbox/{}", slug, agent.name),
                    name: format!("Outbox: {} ({})", agent.name, slug),
                    description: Some(format!("Outbox for agent '{}'", agent.name)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                    title: None,
                },
                annotations: None,
            });

            resources.push(Resource {
                raw: RawResource {
                    uri: format!("resource://inbox/{}?project={}", agent.name, slug),
                    name: format!("Inbox (resource://): {} ({})", agent.name, slug),
                    description: Some(format!("Inbox for agent '{}'", agent.name)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                    title: None,
                },
                annotations: None,
            });
            resources.push(Resource {
                raw: RawResource {
                    uri: format!("resource://outbox/{}?project={}", agent.name, slug),
                    name: format!("Outbox (resource://): {} ({})", agent.name, slug),
                    description: Some(format!("Outbox for agent '{}'", agent.name)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                    title: None,
                },
                annotations: None,
            });
        }

        resources.push(Resource {
            raw: RawResource {
                uri: format!("agent-mail://{}/threads", slug),
                name: format!("Threads ({})", slug),
                description: Some(format!(
                    "List of all conversation threads in project '{}'",
                    slug
                )),
                mime_type: Some("application/json".to_string()),
                size: None,
                icons: None,
                meta: None,
                title: None,
            },
            annotations: None,
        });

        resources.push(Resource {
            raw: RawResource {
                uri: format!("agent-mail://{}/file_reservations", slug),
                name: format!("File Reservations ({})", slug),
                description: Some(format!("Active file reservations in project '{}'", slug)),
                mime_type: Some("application/json".to_string()),
                size: None,
                icons: None,
                meta: None,
                title: None,
            },
            annotations: None,
        });

        resources.push(Resource {
            raw: RawResource {
                uri: format!("resource://agents?project={}", slug),
                name: format!("Agents (resource:// {})", slug),
                description: Some(format!("List of all agents in project '{}'", slug)),
                mime_type: Some("application/json".to_string()),
                size: None,
                icons: None,
                meta: None,
                title: None,
            },
            annotations: None,
        });

        resources.push(Resource {
            raw: RawResource {
                uri: format!("resource://threads?project={}", slug),
                name: format!("Threads (resource:// {})", slug),
                description: Some(format!(
                    "List of all conversation threads in project '{}'",
                    slug
                )),
                mime_type: Some("application/json".to_string()),
                size: None,
                icons: None,
                meta: None,
                title: None,
            },
            annotations: None,
        });
    }

    let products = ProductBmc::list_all(ctx, mm)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    for product in products {
        resources.push(Resource {
            raw: RawResource {
                uri: format!("resource://product/{}", product.product_uid),
                name: format!("Product: {}", product.name),
                description: Some(format!("Information about product '{}'", product.name)),
                mime_type: Some("application/json".to_string()),
                size: None,
                icons: None,
                meta: None,
                title: None,
            },
            annotations: None,
        });
    }

    Ok(ListResourcesResult {
        resources,
        next_cursor: None,
        meta: None,
    })
}
