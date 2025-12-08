use axum::routing::{get, post};
use axum::Router;

use crate::tools;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Core
        .route("/api/health", get(tools::health_check))
        .route("/api/project/ensure", post(tools::ensure_project))
        .route("/api/projects", get(tools::list_all_projects))
        .route("/api/projects/:project_slug/agents", get(tools::list_all_agents_for_project))
        // Identity
        .route("/api/agent/register", post(tools::register_agent))
        .route("/api/agent/whois", post(tools::whois))
        .route("/api/agent/create_identity", post(tools::create_agent_identity))
        // Messaging
        .route("/api/message/send", post(tools::send_message))
        .route("/api/message/reply", post(tools::reply_message))
        .route("/api/message/read", post(tools::mark_message_read))
        .route("/api/message/acknowledge", post(tools::acknowledge_message))
        .route("/api/messages/search", post(tools::search_messages))
        .route("/api/inbox", post(tools::list_inbox))
        .route("/api/messages/:message_id", get(tools::get_message))
        .route("/api/thread", post(tools::get_thread))
        .route("/api/threads", post(tools::list_threads))
        // File Reservations
        .route("/api/file_reservations/paths", post(tools::file_reservation_paths))
        .route("/api/file_reservations/list", post(tools::list_file_reservations))
        .route("/api/file_reservations/release", post(tools::release_file_reservation))
        .route("/api/file_reservations/force_release", post(tools::force_release_reservation))
        .route("/api/file_reservations/renew", post(tools::renew_file_reservation))
        // Extended Info
        .route("/api/project/info", post(tools::get_project_info))
        .route("/api/agent/profile", post(tools::get_agent_profile))
        .route("/api/agent/profile/update", post(tools::update_agent_profile))
        // Contacts
        .route("/api/contacts/request", post(tools::request_contact))
        .route("/api/contacts/respond", post(tools::respond_contact))
        .route("/api/contacts/list", post(tools::list_contacts))
        .route("/api/contacts/policy", post(tools::set_contact_policy))
        // Build Slots
        .route("/api/build_slots/acquire", post(tools::acquire_build_slot))
        .route("/api/build_slots/renew", post(tools::renew_build_slot))
        .route("/api/build_slots/release", post(tools::release_build_slot))
        // Overseer
        .route("/api/overseer/send", post(tools::send_overseer_message))
        // Macros
        .route("/api/macros/list", post(tools::list_macros))
        .route("/api/macros/register", post(tools::register_macro))
        .route("/api/macros/unregister", post(tools::unregister_macro))
        .route("/api/macros/invoke", post(tools::invoke_macro))
        // Thread Summaries
        .route("/api/thread/summarize", post(tools::summarize_thread))
        .route("/api/threads/summarize", post(tools::summarize_threads))
        // Setup (Precommit Guard)
        .route("/api/setup/install_guard", post(tools::install_precommit_guard))
        .route("/api/setup/uninstall_guard", post(tools::uninstall_precommit_guard))
        // Attachments
        .route("/api/attachments/add", post(tools::add_attachment))
        .route("/api/attachments/get", post(tools::get_attachment))
}
