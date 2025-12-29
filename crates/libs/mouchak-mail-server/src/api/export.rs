use crate::AppState;
use axum::http::header;
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use mouchak_mail_core::Ctx;
use mouchak_mail_core::model::export::{ExportBmc, ExportFormat, ScrubMode};
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct ExportPayload {
    pub project_slug: String,
    pub format: String, // "json", "html", "md", "csv"
}

// Note: for now keeping handler signatures simple for utoipa
#[utoipa::path(
    post,
    path = "/api/export",
    request_body = ExportPayload,
    responses(
        (status = 200, description = "Export mailbox", body = String, content_type = "text/csv")
    )
)]
pub async fn export_mailbox(
    State(state): State<AppState>,
    Json(payload): Json<ExportPayload>,
) -> crate::error::Result<Response> {
    let ctx = Ctx::root_ctx();

    let format = payload
        .format
        .parse::<ExportFormat>()
        .unwrap_or(ExportFormat::Json);

    let exported = ExportBmc::export_mailbox(
        &ctx,
        &state.mm,
        &payload.project_slug,
        format,
        ScrubMode::None,
        false,
    )
    .await?;

    // Determine content type and extension
    let (content_type, ext) = match format {
        ExportFormat::Html => ("text/html", "html"),
        ExportFormat::Json => ("application/json", "json"),
        ExportFormat::Markdown => ("text/markdown", "md"),
        ExportFormat::Csv => ("text/csv", "csv"),
    };

    let filename = format!("{}_mailbox.{}", payload.project_slug, ext);

    let response = Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(exported.content)
        .map_err(|e| crate::ServerError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response.into_response())
}
