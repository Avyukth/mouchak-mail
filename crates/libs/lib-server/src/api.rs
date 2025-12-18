use axum::Router;
use axum::routing::{get, post};

use crate::AppState;
use crate::tools;

pub mod attachments;
pub mod export;
pub mod unified_inbox;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Unified Inbox (Gmail-style cross-project view)
        .route(
            "/mail/api/unified-inbox",
            get(unified_inbox::unified_inbox_json),
        )
        // Core
        // ..
        // Export
        .route("/api/export", post(export::export_mailbox))
        // Attachments
        .route("/api/health", get(tools::health_check))
        .route("/api/health_check", get(tools::health_check)) // Python alias
        .route("/api/project/ensure", post(tools::ensure_project))
        .route("/api/ensure_project", post(tools::ensure_project)) // Python alias
        .route("/api/projects", get(tools::list_all_projects))
        .route("/api/list_projects", get(tools::list_all_projects)) // Python alias
        .route("/api/list_all_projects", get(tools::list_all_projects)) // Python alias
        .route(
            "/api/projects/{project_slug}/agents",
            get(tools::list_all_agents_for_project),
        )
        .route("/api/list_agents", get(tools::list_all_agents_for_project)) // Python alias
        // Identity
        .route("/api/agent/register", post(tools::register_agent))
        .route("/api/register_agent", post(tools::register_agent)) // Python alias
        .route("/api/agent/whois", post(tools::whois))
        .route("/api/whois", post(tools::whois)) // Python alias
        .route(
            "/api/agent/create_identity",
            post(tools::create_agent_identity),
        )
        .route(
            "/api/create_agent_identity",
            post(tools::create_agent_identity),
        ) // Python alias
        // Messaging
        .route("/api/message/send", post(tools::send_message))
        .route("/api/send_message", post(tools::send_message)) // Python alias
        .route("/api/message/reply", post(tools::reply_message))
        .route("/api/reply_message", post(tools::reply_message)) // Python alias
        .route("/api/message/read", post(tools::mark_message_read))
        .route("/api/mark_message_read", post(tools::mark_message_read)) // Python alias
        .route("/api/message/acknowledge", post(tools::acknowledge_message))
        .route("/api/acknowledge_message", post(tools::acknowledge_message)) // Python alias
        .route("/api/messages/search", post(tools::search_messages))
        .route("/api/search_messages", post(tools::search_messages)) // Python alias
        // Pending Reviews (ack_required messages awaiting acknowledgment)
        .route(
            "/api/messages/pending-reviews",
            get(tools::list_pending_reviews),
        )
        .route("/api/pending_reviews", get(tools::list_pending_reviews)) // Python alias
        .route("/api/inbox", post(tools::list_inbox))
        .route("/api/fetch_inbox", post(tools::list_inbox)) // Python alias
        .route("/api/list_inbox", post(tools::list_inbox)) // Python alias
        .route("/api/get_inbox", post(tools::list_inbox)) // Python alias
        .route("/api/outbox", post(tools::list_outbox))
        .route("/api/fetch_outbox", post(tools::list_outbox)) // Python alias
        .route("/api/list_outbox", post(tools::list_outbox)) // Python alias
        .route("/api/get_outbox", post(tools::list_outbox)) // Python alias
        .route("/api/messages/{message_id}", get(tools::get_message))
        .route("/api/get_message/{message_id}", get(tools::get_message)) // Python alias
        .route("/api/thread", post(tools::get_thread))
        .route("/api/get_thread", post(tools::get_thread)) // Python alias
        .route("/api/threads", post(tools::list_threads))
        .route("/api/list_threads", post(tools::list_threads)) // Python alias
        // File Reservations
        .route(
            "/api/file_reservations/paths",
            post(tools::file_reservation_paths),
        )
        .route(
            "/api/file_reservation_paths",
            post(tools::file_reservation_paths),
        ) // Python alias
        .route(
            "/api/file_reservations/list",
            post(tools::list_file_reservations),
        )
        .route(
            "/api/list_file_reservations",
            post(tools::list_file_reservations),
        ) // Python alias
        .route("/api/reservations", post(tools::list_file_reservations)) // Python alias (short)
        .route(
            "/api/file_reservations/release",
            post(tools::release_file_reservation),
        )
        .route(
            "/api/release_file_reservation",
            post(tools::release_file_reservation),
        ) // Python alias
        .route(
            "/api/release_file_reservations",
            post(tools::release_file_reservation),
        ) // Python alias (plural)
        .route(
            "/api/file_reservations/force_release",
            post(tools::force_release_reservation),
        )
        .route(
            "/api/force_release_file_reservation",
            post(tools::force_release_reservation),
        ) // Python alias
        .route(
            "/api/force_release_reservation",
            post(tools::force_release_reservation),
        ) // Python alias (short)
        .route(
            "/api/file_reservations/renew",
            post(tools::renew_file_reservation),
        )
        .route(
            "/api/renew_file_reservation",
            post(tools::renew_file_reservation),
        ) // Python alias
        // Extended Info
        .route("/api/project/info", post(tools::get_project_info))
        .route("/api/get_project_info", post(tools::get_project_info)) // Python alias
        .route("/api/project_info", post(tools::get_project_info)) // Python alias (short)
        .route("/api/agent/profile", post(tools::get_agent_profile))
        .route("/api/get_agent_profile", post(tools::get_agent_profile)) // Python alias
        .route("/api/agent_profile", post(tools::get_agent_profile)) // Python alias (short)
        .route(
            "/api/agent/profile/update",
            post(tools::update_agent_profile),
        )
        .route(
            "/api/update_agent_profile",
            post(tools::update_agent_profile),
        ) // Python alias
        // Contacts
        .route("/api/contacts/request", post(tools::request_contact))
        .route("/api/request_contact", post(tools::request_contact)) // Python alias
        .route("/api/contacts/respond", post(tools::respond_contact))
        .route("/api/respond_contact", post(tools::respond_contact)) // Python alias
        .route("/api/contacts/list", post(tools::list_contacts))
        .route("/api/list_contacts", post(tools::list_contacts)) // Python alias
        .route("/api/contacts/policy", post(tools::set_contact_policy))
        .route("/api/set_contact_policy", post(tools::set_contact_policy)) // Python alias
        // Build Slots
        .route("/api/build_slots/acquire", post(tools::acquire_build_slot))
        .route("/api/acquire_build_slot", post(tools::acquire_build_slot)) // Python alias
        .route("/api/build_slots/renew", post(tools::renew_build_slot))
        .route("/api/renew_build_slot", post(tools::renew_build_slot)) // Python alias
        .route("/api/build_slots/release", post(tools::release_build_slot))
        .route("/api/release_build_slot", post(tools::release_build_slot)) // Python alias
        // Overseer
        .route("/api/overseer/send", post(tools::send_overseer_message))
        .route(
            "/api/send_overseer_message",
            post(tools::send_overseer_message),
        ) // Python alias
        // Macros
        .route("/api/macros/list", post(tools::list_macros))
        .route("/api/list_macros", post(tools::list_macros)) // Python alias
        .route("/api/macros/register", post(tools::register_macro))
        .route("/api/register_macro", post(tools::register_macro)) // Python alias
        .route("/api/macros/unregister", post(tools::unregister_macro))
        .route("/api/unregister_macro", post(tools::unregister_macro)) // Python alias
        .route("/api/macros/invoke", post(tools::invoke_macro))
        .route("/api/invoke_macro", post(tools::invoke_macro)) // Python alias
        // Thread Summaries
        .route("/api/thread/summarize", post(tools::summarize_thread))
        .route("/api/summarize_thread", post(tools::summarize_thread)) // Python alias
        .route("/api/threads/summarize", post(tools::summarize_threads))
        .route("/api/summarize_threads", post(tools::summarize_threads)) // Python alias
        // Setup (Precommit Guard)
        .route(
            "/api/setup/install_guard",
            post(tools::install_precommit_guard),
        )
        .route(
            "/api/install_precommit_guard",
            post(tools::install_precommit_guard),
        ) // Python alias
        .route("/api/install_guard", post(tools::install_precommit_guard)) // Python alias (short)
        .route(
            "/api/setup/uninstall_guard",
            post(tools::uninstall_precommit_guard),
        )
        .route(
            "/api/uninstall_precommit_guard",
            post(tools::uninstall_precommit_guard),
        ) // Python alias
        .route(
            "/api/uninstall_guard",
            post(tools::uninstall_precommit_guard),
        ) // Python alias (short)
        // Attachments
        // Attachments
        .route("/api/attachments", get(attachments::list_attachments))
        .route("/api/attachments/add", post(attachments::add_attachment))
        .route("/api/add_attachment", post(attachments::add_attachment)) // Python alias
        .route("/api/attachments/get", get(attachments::get_attachment)) // Changed to GET
        .route("/api/get_attachment/{id}", get(attachments::get_attachment)) // RESTful
        .route("/api/get_attachment", get(attachments::get_attachment)) // Python alias (path param?)
        // Note: attachments::get_attachment uses Path<i64>.
        // Route must capture it or use Query/Body.
        // My implementation in api/attachments.rs uses `Path(id)`.
        // So I must route to `/api/get_attachment/:id`.
        // But the previous API was POST with JSON body?
        // tools.rs stub used Json body.
        // My new implementation uses Path.
        // I should stick to one or the other or support both.
        // RESTful GET /.../:id is better for downloading files.
        // So I will update routes to match `Path`.
        .route("/api/attachments/{id}", get(attachments::get_attachment))
        // Metrics
        .route("/api/metrics/tools", get(tools::list_tool_metrics))
        .route("/api/list_tool_metrics", get(tools::list_tool_metrics)) // Python alias
        .route("/api/metrics/tools/stats", get(tools::get_tool_stats))
        .route("/api/get_tool_stats", get(tools::get_tool_stats)) // Python alias
        .route("/api/tool_stats", get(tools::get_tool_stats)) // Python alias (short)
        .route("/api/activity", get(tools::list_activity))
        .route("/api/list_activity", get(tools::list_activity)) // Python alias
        // Archive
        .route("/api/archive/commit", post(tools::commit_archive))
        .route("/api/commit_archive", post(tools::commit_archive)) // Python alias
        // Siblings
        .route("/api/project/siblings", post(tools::list_project_siblings))
        .route(
            "/api/list_project_siblings",
            post(tools::list_project_siblings),
        ) // Python alias
}
