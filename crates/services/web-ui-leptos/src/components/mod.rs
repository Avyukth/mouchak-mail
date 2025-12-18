//! UI components.

pub mod alert;
pub mod avatar;
pub mod badge;
pub mod breadcrumb;
pub mod button;
pub mod card;
pub mod compose_message;
pub mod dialog;
pub mod filter_bar;
pub mod inline_message_detail;
pub mod input;
pub mod layout;
pub mod message_detail_header;
pub mod project_card;
pub mod select;
pub mod skeleton;
pub mod split_view;

pub use alert::{Alert, AlertDescription, AlertTitle, AlertVariant};
pub use avatar::{AgentAvatar, AvatarSize};
pub use badge::{Badge, BadgeVariant};
pub use breadcrumb::{Breadcrumb, BreadcrumbItem};
pub use button::{Button, ButtonSize, ButtonVariant};
pub use card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
pub use compose_message::{ComposeMessage, ComposeProps, ReplyTo};
pub use dialog::{
    Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
    DialogTrigger,
};

pub use filter_bar::{FilterBar, FilterState};
pub use inline_message_detail::InlineMessageDetail;
pub use input::Input;
pub use layout::Layout;
pub use message_detail_header::MessageDetailHeader;
pub use project_card::{ProjectCard, ProjectStatus, determine_project_status};
pub use select::{Select, SelectOption};
pub use skeleton::Skeleton;
pub use split_view::{EmptyDetailPanel, MessageListItem, SplitViewLayout};
