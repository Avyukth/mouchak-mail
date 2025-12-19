use crate::AppState;
use crate::auth::AuthenticatedUser;
use axum::http::header;
use axum::{
    Extension, Json,
    body::Body,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use base64::Engine;
use lib_core::Ctx;
use lib_core::model::agent::AgentBmc;
use lib_core::model::attachment::{AttachmentBmc, AttachmentForCreate};
use lib_core::model::project::ProjectBmc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tracing::warn;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct AddAttachmentPayload {
    pub project_slug: String,
    /// Optional agent name that uploaded this file.
    #[serde(default)]
    pub agent_name: Option<String>,
    pub filename: String,
    pub content_base64: String,
}

#[derive(Serialize, ToSchema)]
pub struct AddAttachmentResponse {
    pub id: i64,
    pub filename: String,
    pub size: i64,
}

#[utoipa::path(
    post,
    path = "/api/attachments/add",
    request_body = AddAttachmentPayload,
    responses(
        (status = 200, description = "Attachment added", body = AddAttachmentResponse)
    )
)]
pub async fn add_attachment(
    State(state): State<AppState>,
    auth_user: Option<Extension<AuthenticatedUser>>,
    Json(payload): Json<AddAttachmentPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &state.mm;

    if auth_user.is_none() {
        warn!(
            "add_attachment called without authenticated user for project: {}",
            payload.project_slug
        );
    }

    // 1. Get Project
    let project = ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug).await?;

    // 1b. Resolve agent_name to agent_id if provided
    let agent_id = if let Some(ref agent_name) = payload.agent_name {
        let agent = AgentBmc::get_by_name(&ctx, mm, project.id, agent_name).await?;
        Some(agent.id)
    } else {
        None
    };

    // 2. Decode Content
    // Sanitize base64 string (remove data URL prefix if present)
    let b64 = if let Some(idx) = payload.content_base64.find(',') {
        &payload.content_base64[idx + 1..]
    } else {
        &payload.content_base64
    };

    let content = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| crate::ServerError::BadRequest(format!("Invalid base64: {}", e)))?;

    let size = content.len() as i64;
    // Max size check (10MB)
    if size > 10 * 1024 * 1024 {
        return Err(crate::ServerError::BadRequest(
            "File too large (>10MB)".into(),
        ));
    }

    // 3. Prepare Storage Logic
    // Sanitize filename
    let filename = sanitize_filename::sanitize(&payload.filename);
    if filename.is_empty() {
        return Err(crate::ServerError::BadRequest("Invalid filename".into()));
    }

    // Determine path: data/attachments/<project_id>/<uuid>_<filename>
    let attachment_root = std::env::current_dir()?
        .join("data")
        .join("attachments")
        .join(project.id.to_string());
    fs::create_dir_all(&attachment_root).await?;

    let unique_id = uuid::Uuid::new_v4();
    let stored_filename = format!("{}_{}", unique_id, filename);
    let stored_path = attachment_root.join(&stored_filename);

    // 4. Write File
    fs::write(&stored_path, &content).await?;

    // 5. Create DB Record
    // Infer media type manually or naive
    let mime = mime_guess::from_path(&filename).first_or_octet_stream();

    let id = AttachmentBmc::create(
        &ctx,
        mm,
        AttachmentForCreate {
            project_id: project.id,
            agent_id,
            filename: filename.clone(),
            stored_path: stored_path.to_string_lossy().to_string(),
            media_type: mime.to_string(),
            size_bytes: size,
        },
    )
    .await?;

    Ok(Json(AddAttachmentResponse { id, filename, size }).into_response())
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ListAttachmentsParams {
    pub project_slug: String,
    /// Optional agent name to filter attachments by.
    pub agent_name: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/attachments",
    params(ListAttachmentsParams),
    responses(
        (status = 200, description = "List attachments", body = Vec<lib_core::model::attachment::Attachment>)
    )
)]
pub async fn list_attachments(
    State(state): State<AppState>,
    auth_user: Option<Extension<AuthenticatedUser>>,
    Query(params): Query<ListAttachmentsParams>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();

    if auth_user.is_none() {
        warn!(
            "list_attachments called without authenticated user for project: {}",
            params.project_slug
        );
    }

    let project = ProjectBmc::get_by_identifier(&ctx, &state.mm, &params.project_slug).await?;

    // Resolve agent_name to agent_id if provided
    let agent_id = if let Some(ref agent_name) = params.agent_name {
        let agent = AgentBmc::get_by_name(&ctx, &state.mm, project.id, agent_name).await?;
        Some(agent.id)
    } else {
        None
    };

    let items =
        AttachmentBmc::list_by_project_and_agent(&ctx, &state.mm, project.id, agent_id).await?;
    Ok(Json(items).into_response())
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct GetAttachmentParams {
    pub project_slug: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/attachments/{id}",
    params(
        ("id" = i64, Path, description = "Attachment ID"),
        GetAttachmentParams
    ),
    responses(
        (status = 200, description = "Download attachment", body = String)
    )
)]
pub async fn get_attachment(
    State(state): State<AppState>,
    auth_user: Option<Extension<AuthenticatedUser>>,
    Path(id): Path<i64>,
    Query(params): Query<GetAttachmentParams>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let mm = &state.mm;

    if auth_user.is_none() {
        warn!(
            "get_attachment called without authenticated user for id: {}",
            id
        );
    }

    let attachment = AttachmentBmc::get(&ctx, mm, id).await?;

    // Security: Verify project access if project_slug provided
    if let Some(ref project_slug) = params.project_slug {
        let project = ProjectBmc::get_by_identifier(&ctx, mm, project_slug).await?;
        if attachment.project_id != project.id {
            warn!(
                "get_attachment: project mismatch - attachment {} belongs to project {}, not {}",
                id, attachment.project_id, project.id
            );
            return Err(crate::ServerError::Forbidden);
        }
    } else {
        warn!(
            "get_attachment called without project_slug validation for attachment id: {}",
            id
        );
    }

    let path = PathBuf::from(&attachment.stored_path);
    if !path.exists() {
        return Err(crate::ServerError::NotFound(
            "File on disk not found".into(),
        ));
    }

    let file = tokio::fs::File::open(path).await?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let response = Response::builder()
        .header(header::CONTENT_TYPE, attachment.media_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", attachment.filename),
        )
        .body(body)
        .map_err(|e| crate::ServerError::Internal(format!("Failed to build response: {}", e)))?
        .into_response();

    Ok(response)
}
