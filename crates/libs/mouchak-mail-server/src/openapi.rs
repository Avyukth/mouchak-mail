use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        // Health
        crate::health_handler,
        crate::ready_handler,
        // Attachments
        crate::api::attachments::add_attachment,
        crate::api::attachments::list_attachments,
        crate::api::attachments::get_attachment,
        // Export
        crate::api::export::export_mailbox,
    ),
    components(
        schemas(
            // Auto-discovery might need explicit import if not working
        )
    ),
    tags(
        (name = "mouchak-mail", description = "Mouchak Mail API")
    )
)]
pub struct ApiDoc;
