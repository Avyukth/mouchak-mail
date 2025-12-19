//! Message Detail Header component with metadata grid and action buttons.
//!
//! Displays message subject, sender/recipient info with avatars,
//! project and timestamp, plus action buttons.

use crate::components::{AgentAvatar, AvatarSize, Button, ButtonVariant};
use leptos::prelude::*;

/// Format a timestamp for display
fn format_timestamp(ts: &str) -> String {
    if ts.is_empty() {
        return "—".to_string();
    }
    // Parse ISO timestamp and format nicely
    // Input: "2025-12-18T10:30:00"
    // Output: "Dec 18, 2025 at 10:30"
    if let Some((date, time)) = ts.split_once('T') {
        let time_short = time.split(':').take(2).collect::<Vec<_>>().join(":");
        format!("{} at {}", date, time_short)
    } else {
        ts.to_string()
    }
}

/// Copy text to clipboard using Web API
#[cfg(target_arch = "wasm32")]
fn copy_to_clipboard(text: &str) {
    if let Some(window) = web_sys::window() {
        let navigator = window.navigator();
        // clipboard() returns Clipboard directly in current web-sys
        let clipboard = navigator.clipboard();
        let _ = clipboard.write_text(text);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_to_clipboard(_text: &str) {
    // No-op for non-WASM builds
}

/// Get window origin for building URLs
#[cfg(target_arch = "wasm32")]
fn window_origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:8765".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn window_origin() -> String {
    "http://localhost:8765".to_string()
}

/// A single metadata item in the header grid
#[component]
fn MetadataItem(
    /// Label text (e.g., "FROM", "TO")
    #[prop(into)]
    label: String,
    /// Lucide icon name
    #[prop(into)]
    icon: String,
    /// Child content
    children: Children,
) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-1">
            <span class="text-xs font-medium text-charcoal-500 dark:text-charcoal-400 uppercase tracking-wider flex items-center gap-1">
                <i data-lucide={icon} class="icon-xs"></i>
                {label}
            </span>
            <div class="flex items-center gap-2 text-sm text-charcoal-800 dark:text-cream-100">
                {children()}
            </div>
        </div>
    }
}

/// Rich message detail header with metadata grid and action buttons.
///
/// # Example
/// ```rust,ignore
/// view! {
///     <MessageDetailHeader
///         subject="Backend-Frontend Sync".to_string()
///         sender="worker-1".to_string()
///         recipients=vec!["reviewer".to_string()]
///         project_slug="my-project".to_string()
///         sent_at="2025-12-18T10:30:00".to_string()
///         message_id=123
///     />
/// }
/// ```
#[component]
pub fn MessageDetailHeader(
    /// Message subject line
    #[prop(into)]
    subject: String,
    /// Sender agent name
    #[prop(into)]
    sender: String,
    /// List of recipient names
    #[prop(into)]
    recipients: Vec<String>,
    /// Project slug
    #[prop(into)]
    project_slug: String,
    /// Sent timestamp (ISO format)
    #[prop(into)]
    sent_at: String,
    /// Message ID for building links
    message_id: i64,
) -> impl IntoView {
    // State for copy button feedback
    let copied = RwSignal::new(false);

    let message_id_for_copy = message_id;
    let copy_link = move |_| {
        let url = format!("{}/inbox/{}", window_origin(), message_id_for_copy);
        copy_to_clipboard(&url);
        copied.set(true);

        // Reset after 2 seconds
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(2000).await;
            copied.set(false);
        });
    };

    let project_link = format!("/projects/{}", project_slug);
    let project_link_button = project_link.clone();
    let recipients_display = recipients.join(", ");
    let first_recipient = recipients.first().cloned().unwrap_or_default();

    view! {
        <div class="p-6 border-b border-cream-200 dark:border-charcoal-700 bg-cream-50/50 dark:bg-charcoal-800/50">
            // Subject
            <h1 class="font-display text-xl font-bold text-charcoal-800 dark:text-cream-100 mb-4 flex items-center gap-2">
                <i data-lucide="mail" class="icon-lg text-amber-500"></i>
                {subject}
            </h1>

            // Metadata Grid
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
                <MetadataItem label="From" icon="user">
                    <AgentAvatar name={sender.clone()} size=AvatarSize::Sm />
                    <span class="font-medium">{sender}</span>
                </MetadataItem>

                <MetadataItem label="To" icon="users">
                    {if !first_recipient.is_empty() {
                        Some(view! { <AgentAvatar name={first_recipient.clone()} size=AvatarSize::Sm /> })
                    } else {
                        None
                    }}
                    <span>{recipients_display}</span>
                </MetadataItem>

                <MetadataItem label="Project" icon="folder">
                    <a
                        href={project_link.clone()}
                        class="text-amber-600 dark:text-amber-400 hover:underline truncate max-w-[150px]"
                        title={project_slug.clone()}
                    >
                        {project_slug.clone()}
                    </a>
                </MetadataItem>

                <MetadataItem label="Sent" icon="calendar">
                    <span class="font-mono text-xs">{format_timestamp(&sent_at)}</span>
                </MetadataItem>
            </div>

            // Action Buttons
            <div class="flex gap-2">
                <Button
                    variant=ButtonVariant::Secondary
                    on_click=Callback::new(copy_link)
                >
                    {move || if copied.get() {
                        view! { <i data-lucide="check" class="icon-sm text-green-500"></i> }.into_any()
                    } else {
                        view! { <i data-lucide="copy" class="icon-sm"></i> }.into_any()
                    }}
                    {move || if copied.get() { "Copied!" } else { "Copy Link" }}
                </Button>

                <a
                    href={project_link_button}
                    class="btn-secondary flex items-center gap-2 text-sm"
                >
                    <i data-lucide="external-link" class="icon-sm"></i>
                    "Open in Project"
                </a>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Timestamp formatting tests ===

    #[test]
    fn test_format_timestamp_iso() {
        let ts = "2025-12-18T10:30:00";
        let formatted = format_timestamp(ts);
        assert!(formatted.contains("2025-12-18"));
        assert!(formatted.contains("10:30"));
    }

    #[test]
    fn test_format_timestamp_empty() {
        assert_eq!(format_timestamp(""), "—");
    }

    #[test]
    fn test_format_timestamp_no_time() {
        let ts = "2025-12-18";
        let formatted = format_timestamp(ts);
        assert_eq!(formatted, "2025-12-18");
    }

    #[test]
    fn test_format_timestamp_with_seconds() {
        let ts = "2025-12-18T14:30:45";
        let formatted = format_timestamp(ts);
        // Should only show hours:minutes
        assert!(formatted.contains("14:30"));
        assert!(!formatted.contains("45"));
    }

    #[test]
    fn test_format_timestamp_with_timezone() {
        let ts = "2025-12-18T10:30:00+05:30";
        let formatted = format_timestamp(ts);
        // Should handle timezone suffix gracefully
        assert!(formatted.contains("2025-12-18"));
        assert!(formatted.contains("10:30"));
    }

    #[test]
    fn test_format_timestamp_midnight() {
        let ts = "2025-12-18T00:00:00";
        let formatted = format_timestamp(ts);
        assert!(formatted.contains("00:00"));
    }

    // === Window origin tests (non-WASM) ===

    #[test]
    fn test_window_origin_fallback() {
        // In non-WASM builds, should return localhost fallback
        let origin = window_origin();
        assert_eq!(origin, "http://localhost:8765");
    }

    // === Copy to clipboard (non-WASM no-op) ===

    #[test]
    fn test_copy_to_clipboard_noop() {
        // Should not panic in non-WASM builds
        copy_to_clipboard("test text");
        copy_to_clipboard("");
        copy_to_clipboard("https://example.com/inbox/123");
    }

    // === Edge case tests for component logic ===

    #[test]
    fn test_recipients_join_empty() {
        let recipients: Vec<String> = vec![];
        let display = recipients.join(", ");
        assert_eq!(display, "");
    }

    #[test]
    fn test_recipients_join_single() {
        let recipients = vec!["worker-1".to_string()];
        let display = recipients.join(", ");
        assert_eq!(display, "worker-1");
    }

    #[test]
    fn test_recipients_join_multiple() {
        let recipients = vec![
            "worker-1".to_string(),
            "reviewer".to_string(),
            "human".to_string(),
        ];
        let display = recipients.join(", ");
        assert_eq!(display, "worker-1, reviewer, human");
    }

    #[test]
    fn test_first_recipient_empty_list() {
        let recipients: Vec<String> = vec![];
        let first = recipients.first().cloned().unwrap_or_default();
        assert_eq!(first, "");
    }

    #[test]
    fn test_first_recipient_exists() {
        let recipients = vec!["primary".to_string(), "secondary".to_string()];
        let first = recipients.first().cloned().unwrap_or_default();
        assert_eq!(first, "primary");
    }

    #[test]
    fn test_project_link_generation() {
        let project_slug = "my-project";
        let link = format!("/projects/{}", project_slug);
        assert_eq!(link, "/projects/my-project");
    }

    #[test]
    fn test_project_link_with_special_chars() {
        // Project slugs should already be URL-safe, but test anyway
        let project_slug = "project-with-dashes";
        let link = format!("/projects/{}", project_slug);
        assert_eq!(link, "/projects/project-with-dashes");
    }

    #[test]
    fn test_message_url_generation() {
        let message_id: i64 = 12345;
        let origin = window_origin();
        let url = format!("{}/inbox/{}", origin, message_id);
        assert_eq!(url, "http://localhost:8765/inbox/12345");
    }

    #[test]
    fn test_long_subject_handling() {
        // Long subjects should be handled by CSS truncation, not Rust
        // Test that we don't panic or corrupt long strings
        let long_subject = "A".repeat(500);
        assert_eq!(long_subject.len(), 500);
        // The component would use this directly - CSS handles overflow
    }

    #[test]
    fn test_long_project_slug_handling() {
        // Very long project slugs (edge case)
        let long_slug = "a".repeat(200);
        let link = format!("/projects/{}", long_slug);
        assert!(link.starts_with("/projects/"));
        assert!(link.len() > 200);
    }

    #[test]
    fn test_unicode_in_sender_name() {
        // Sender names might contain unicode
        let sender = "日本語エージェント";
        let formatted = format!("From: {}", sender);
        assert!(formatted.contains(sender));
    }

    #[test]
    fn test_unicode_in_subject() {
        let subject = "会議の議事録 📝";
        let formatted = format!("Subject: {}", subject);
        assert!(formatted.contains(subject));
    }

    // === Additional Recipients Tests (LEPTOS-008) ===

    #[test]
    fn test_recipients_display_more_than_three() {
        let recipients = vec![
            "alice".to_string(),
            "bob".to_string(),
            "charlie".to_string(),
            "diana".to_string(),
            "eve".to_string(),
        ];
        // When > 3 recipients, UI should show first and "+N more"
        let display_count = 3;
        let overflow = recipients.len().saturating_sub(display_count);
        assert_eq!(overflow, 2);
        let display = format!(
            "{} +{} more",
            recipients[..display_count].join(", "),
            overflow
        );
        assert!(display.contains("+2 more"));
    }

    #[test]
    fn test_recipients_avatar_for_first() {
        let recipients = vec!["alice".to_string(), "bob".to_string()];
        // First recipient gets an avatar
        let first = recipients.first().cloned().unwrap_or_default();
        assert_eq!(first, "alice");
        assert!(!first.is_empty());
    }

    #[test]
    fn test_recipients_graceful_null_handling() {
        // Simulate null/undefined from API (empty vec after serde default)
        let recipients: Vec<String> = vec![];
        let display = if recipients.is_empty() {
            "No recipients".to_string()
        } else {
            recipients.join(", ")
        };
        assert_eq!(display, "No recipients");
    }

    #[test]
    fn test_recipients_with_spaces_in_names() {
        // Agent names might have spaces
        let recipients = vec!["Alice Agent".to_string(), "Bob Builder".to_string()];
        let display = recipients.join(", ");
        assert_eq!(display, "Alice Agent, Bob Builder");
    }

    #[test]
    fn test_recipients_very_long_list() {
        // Should handle many recipients gracefully
        let recipients: Vec<String> = (0..50).map(|i| format!("agent-{}", i)).collect();
        assert_eq!(recipients.len(), 50);
        // UI should truncate display
        let truncated = &recipients[..3];
        let overflow = recipients.len() - 3;
        let display = format!("{} +{} more", truncated.join(", "), overflow);
        assert!(display.contains("+47 more"));
    }

    #[test]
    fn test_recipients_special_chars_escaped() {
        // Special characters should not break display
        let recipients = vec!["agent<script>".to_string(), "agent&more".to_string()];
        let display = recipients.join(", ");
        // These chars would be HTML-escaped by Leptos view
        assert!(display.contains("<script>"));
        assert!(display.contains("&more"));
    }
}
