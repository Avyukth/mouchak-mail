//! UI components.

pub mod avatar;
pub mod compose_message;
pub mod filter_bar;
pub mod layout;
pub mod message_detail_header;
pub mod select;
pub mod split_view;

pub use avatar::AgentAvatar;
pub use compose_message::{ComposeMessage, ComposeProps, ReplyTo};
pub use filter_bar::{FilterBar, FilterState};
pub use layout::Layout;
pub use message_detail_header::MessageDetailHeader;
pub use select::{Select, SelectOption};
pub use split_view::{EmptyDetailPanel, MessageListItem, SplitViewLayout};
