//! Page components for each route.

mod agents;
mod archive;
mod attachments;
mod dashboard;
mod file_reservations;
mod inbox;
mod message_detail;
mod project_detail;
mod projects;
mod search;
mod thread;
mod unified_inbox;

pub use agents::Agents;
pub use archive::ArchiveBrowser;
pub use attachments::Attachments;
pub use dashboard::Dashboard;
pub use file_reservations::FileReservations;
pub use inbox::Inbox;
pub use message_detail::MessageDetail;
pub use project_detail::ProjectDetail;
pub use projects::Projects;
pub use search::Search;
pub use thread::ThreadView;
pub use unified_inbox::UnifiedInbox;
