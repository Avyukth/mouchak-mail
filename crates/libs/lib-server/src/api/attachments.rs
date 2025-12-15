use axum::{
    extract::{State, Query, Path},
    Json,
    response::{IntoResponse, Response},
    body::Body,
};
use axum::http::header;
use serde::{Deserialize, Serialize};
use crate::AppState;
use lib_core::Ctx;
use lib_core::model::attachment::{AttachmentBmc, AttachmentForCreate};
use lib_core::model::project::ProjectBmc;
use std::path::PathBuf;
use tokio::fs;
use base64::Engine;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct AddAttachmentPayload {
    pub project_slug: String,
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
    Json(payload): Json<AddAttachmentPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx(); // TODO: Use user context
    let mm = &state.mm;

    // 1. Get Project
    let project = ProjectBmc::get_by_identifier(&ctx, mm, &payload.project_slug).await?;

    // 2. Decode Content
    // Sanitize base64 string (remove data URL prefix if present)
    let b64 = if let Some(idx) = payload.content_base64.find(',') {
        &payload.content_base64[idx+1..]
    } else {
        &payload.content_base64
    };
    
    let content = base64::engine::general_purpose::STANDARD.decode(b64)
        .map_err(|e| crate::ServerError::BadRequest(format!("Invalid base64: {}", e)))?;
    
    let size = content.len() as i64;
    // Max size check (10MB)
    if size > 10 * 1024 * 1024 {
        return Err(crate::ServerError::BadRequest("File too large (>10MB)".into()));
    }

    // 3. Prepare Storage Logic
    // Sanitize filename
    let filename = sanitize_filename::sanitize(&payload.filename);
    if filename.is_empty() {
        return Err(crate::ServerError::BadRequest("Invalid filename".into()));
    }

    // Determine path: data/attachments/<project_id>/<uuid>_<filename>
    let attachment_root = std::env::current_dir()?.join("data").join("attachments").join(project.id.to_string());
    fs::create_dir_all(&attachment_root).await?;

    let unique_id = uuid::Uuid::new_v4();
    let stored_filename = format!("{}_{}", unique_id, filename);
    let stored_path = attachment_root.join(&stored_filename);

    // 4. Write File
    fs::write(&stored_path, &content).await?;
    
    // 5. Create DB Record
    // Infer media type manually or naive
    let mime = mime_guess::from_path(&filename).first_or_octet_stream();

    let id = AttachmentBmc::create(&ctx, mm, AttachmentForCreate {
        project_id: project.id,
        filename: filename.clone(),
        stored_path: stored_path.to_string_lossy().to_string(),
        media_type: mime.to_string(),
        size_bytes: size,
    }).await?;

    Ok(Json(AddAttachmentResponse {
        id,
        filename,
        size,
    }).into_response())
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ListAttachmentsParams {
    pub project_slug: String,
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
    Query(params): Query<ListAttachmentsParams>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let project = ProjectBmc::get_by_identifier(&ctx, &state.mm, &params.project_slug).await?;
    let items = AttachmentBmc::list_by_project(&ctx, &state.mm, project.id).await?;
    Ok(Json(items).into_response())
}

#[utoipa::path(
    get,
    path = "/api/attachments/{id}",
    params(
        ("id" = i64, Path, description = "Attachment ID")
    ),
    responses(
        (status = 200, description = "Download attachment", body = String)
    )
)]
pub async fn get_attachment(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();
    let attachment = AttachmentBmc::get(&ctx, &state.mm, id).await?;

    // Read file
    let path = PathBuf::from(&attachment.stored_path);
    if !path.exists() {
        return Err(crate::ServerError::NotFound("File on disk not found".into()));
    }

    let file = tokio::fs::File::open(path).await?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, attachment.media_type)
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", attachment.filename))
        .body(body)
        .unwrap()
        .into_response())
}
