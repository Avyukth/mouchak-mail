//! Page components for each route.

mod agents;
mod dashboard;
mod file_reservations;
mod inbox;
mod message_detail;
mod project_detail;
mod projects;
mod unified_inbox;

pub use agents::Agents;
pub use dashboard::Dashboard;
pub use file_reservations::FileReservations;
pub use inbox::Inbox;
pub use message_detail::MessageDetail;
pub use project_detail::ProjectDetail;
pub use projects::Projects;
pub use unified_inbox::UnifiedInbox;
