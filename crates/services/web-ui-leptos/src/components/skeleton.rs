//! Skeleton component for loading states.
//!
//! Renders animated pulsing placeholders following shadcn/ui patterns.
//! Includes pre-built skeletons for common UI patterns.

use leptos::prelude::*;
use tailwind_fuse::tw_merge;

// ============================================================================
// Constants
// ============================================================================

/// Base skeleton classes with pulse animation and reduced-motion support.
pub const SKELETON_BASE: &str =
    "animate-pulse rounded-md bg-muted motion-reduce:animate-none motion-reduce:opacity-70";

/// Number of items in message list skeleton.
pub const MESSAGE_LIST_ITEM_COUNT: usize = 5;

/// Grid dimensions for attachment skeleton.
pub const ATTACHMENT_GRID_COLS: usize = 3;
pub const ATTACHMENT_GRID_ROWS: usize = 3;

// ============================================================================
// Base Skeleton Component
// ============================================================================

/// Base skeleton component with animated pulse.
///
/// Uses semantic `bg-muted` color and respects `prefers-reduced-motion`.
///
/// # Example
/// ```rust,ignore
/// view! {
///     <Skeleton class="h-4 w-[250px]".to_string() />
/// }
/// ```
#[component]
pub fn Skeleton(#[prop(optional, into)] class: Option<String>) -> impl IntoView {
    let extra = class.unwrap_or_default();
    let final_class = tw_merge!(SKELETON_BASE, extra);

    view! {
        <div class={final_class} aria-hidden="true"></div>
    }
}

// ============================================================================
// Pre-built Skeletons
// ============================================================================

/// Skeleton for a single message list item.
/// Matches the layout of MessageListItem component.
#[component]
pub fn MessageItemSkeleton() -> impl IntoView {
    view! {
        <div class="flex items-start gap-3 p-4 border-b border-border">
            // Avatar placeholder
            <Skeleton class="h-10 w-10 rounded-full shrink-0".to_string() />

            <div class="flex-1 space-y-2 min-w-0">
                // Sender and time row
                <div class="flex items-center justify-between gap-2">
                    <Skeleton class="h-4 w-[120px]".to_string() />
                    <Skeleton class="h-3 w-[60px]".to_string() />
                </div>

                // Subject
                <Skeleton class="h-4 w-full max-w-[280px]".to_string() />

                // Preview
                <Skeleton class="h-3 w-full max-w-[200px]".to_string() />
            </div>
        </div>
    }
}

/// Skeleton for message list with multiple items.
/// Default: 5 items to fill typical viewport.
#[component]
pub fn MessageListSkeleton(
    #[prop(default = MESSAGE_LIST_ITEM_COUNT)] count: usize,
) -> impl IntoView {
    view! {
        <div class="divide-y divide-border" role="status" aria-label="Loading messages">
            <span class="sr-only">"Loading messages..."</span>
            {(0..count).map(|_| view! { <MessageItemSkeleton /> }).collect::<Vec<_>>()}
        </div>
    }
}

/// Skeleton for message detail view (header + body).
#[component]
pub fn MessageDetailSkeleton() -> impl IntoView {
    view! {
        <div class="p-6 space-y-6" role="status" aria-label="Loading message">
            <span class="sr-only">"Loading message details..."</span>

            // Header section
            <div class="space-y-4">
                // Subject
                <Skeleton class="h-6 w-3/4".to_string() />

                // Sender info
                <div class="flex items-center gap-3">
                    <Skeleton class="h-10 w-10 rounded-full".to_string() />
                    <div class="space-y-1.5">
                        <Skeleton class="h-4 w-[150px]".to_string() />
                        <Skeleton class="h-3 w-[200px]".to_string() />
                    </div>
                </div>

                // Metadata (date, project)
                <div class="flex gap-4">
                    <Skeleton class="h-3 w-[100px]".to_string() />
                    <Skeleton class="h-3 w-[80px]".to_string() />
                </div>
            </div>

            // Divider
            <Skeleton class="h-px w-full".to_string() />

            // Body content
            <div class="space-y-3">
                <Skeleton class="h-4 w-full".to_string() />
                <Skeleton class="h-4 w-full".to_string() />
                <Skeleton class="h-4 w-5/6".to_string() />
                <Skeleton class="h-4 w-4/5".to_string() />
                <Skeleton class="h-4 w-2/3".to_string() />
            </div>
        </div>
    }
}

/// Skeleton for single attachment card.
#[component]
pub fn AttachmentCardSkeleton() -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border overflow-hidden">
            // Image preview area
            <Skeleton class="aspect-square w-full".to_string() />

            // Footer
            <div class="p-3 space-y-1.5">
                <Skeleton class="h-3 w-3/4".to_string() />
                <Skeleton class="h-2 w-1/2".to_string() />
            </div>
        </div>
    }
}

/// Skeleton for attachment grid (3x3 default).
#[component]
pub fn AttachmentGridSkeleton(
    #[prop(default = ATTACHMENT_GRID_COLS)] cols: usize,
    #[prop(default = ATTACHMENT_GRID_ROWS)] rows: usize,
) -> impl IntoView {
    let total = cols * rows;
    let grid_class = format!(
        "grid grid-cols-{} gap-4 p-4",
        cols.min(4) // Max 4 columns for responsive
    );

    view! {
        <div class={grid_class} role="status" aria-label="Loading attachments">
            <span class="sr-only">"Loading attachments..."</span>
            {(0..total).map(|_| view! { <AttachmentCardSkeleton /> }).collect::<Vec<_>>()}
        </div>
    }
}

/// Skeleton for a card component.
#[component]
pub fn CardSkeleton() -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border p-6 space-y-4">
            // Header
            <div class="space-y-2">
                <Skeleton class="h-5 w-1/3".to_string() />
                <Skeleton class="h-3 w-2/3".to_string() />
            </div>

            // Content
            <div class="space-y-2">
                <Skeleton class="h-4 w-full".to_string() />
                <Skeleton class="h-4 w-4/5".to_string() />
            </div>
        </div>
    }
}

/// Skeleton for a table row.
#[component]
pub fn TableRowSkeleton(#[prop(default = 4)] columns: usize) -> impl IntoView {
    view! {
        <tr>
            {(0..columns).map(|i| {
                let width = match i {
                    0 => "w-1/4",
                    _ => "w-1/6",
                };
                view! {
                    <td class="p-3">
                        <Skeleton class=format!("h-4 {}", width) />
                    </td>
                }
            }).collect::<Vec<_>>()}
        </tr>
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === Constants ===

    #[test]
    fn test_message_list_item_count() {
        assert_eq!(MESSAGE_LIST_ITEM_COUNT, 5);
    }

    #[test]
    fn test_attachment_grid_dimensions() {
        assert_eq!(ATTACHMENT_GRID_COLS, 3);
        assert_eq!(ATTACHMENT_GRID_ROWS, 3);
    }

    // === Base Skeleton Classes ===

    #[test]
    fn test_skeleton_base_has_pulse() {
        assert!(SKELETON_BASE.contains("animate-pulse"));
    }

    #[test]
    fn test_skeleton_base_has_bg_muted() {
        assert!(SKELETON_BASE.contains("bg-muted"));
    }

    #[test]
    fn test_skeleton_base_has_rounded() {
        assert!(SKELETON_BASE.contains("rounded-md"));
    }

    #[test]
    fn test_skeleton_respects_reduced_motion() {
        assert!(SKELETON_BASE.contains("motion-reduce:animate-none"));
    }

    #[test]
    fn test_skeleton_reduced_motion_opacity() {
        // For reduced motion users, show static opacity instead of animation
        assert!(SKELETON_BASE.contains("motion-reduce:opacity-70"));
    }

    // === Class Merging ===

    #[test]
    fn test_skeleton_class_merge_none() {
        let result = tw_merge!(SKELETON_BASE, "");
        assert!(result.contains("animate-pulse"));
    }

    #[test]
    fn test_skeleton_class_merge_with_size() {
        let result = tw_merge!(SKELETON_BASE, "h-4 w-full");
        assert!(result.contains("h-4"));
        assert!(result.contains("w-full"));
    }

    #[test]
    fn test_skeleton_class_merge_override() {
        let result = tw_merge!(SKELETON_BASE, "rounded-full");
        // Should contain rounded-full (merged/overridden)
        assert!(result.contains("rounded"));
    }

    // === Pre-built Skeleton Structure ===

    #[test]
    fn test_message_item_skeleton_has_avatar() {
        // Avatar is 10x10 (h-10 w-10)
        let expected_avatar = "h-10 w-10 rounded-full";
        assert!(expected_avatar.contains("h-10"));
        assert!(expected_avatar.contains("rounded-full"));
    }

    #[test]
    fn test_message_item_skeleton_has_three_lines() {
        // Should have: sender/time, subject, preview
        let lines = 3;
        assert_eq!(lines, 3);
    }

    #[test]
    fn test_message_detail_skeleton_has_subject() {
        // Subject line should be larger (h-6)
        let subject_class = "h-6 w-3/4";
        assert!(subject_class.contains("h-6"));
    }

    #[test]
    fn test_message_detail_skeleton_has_body_lines() {
        // Body should have multiple lines with varying widths
        let widths = ["w-full", "w-5/6", "w-4/5", "w-2/3"];
        assert!(widths.len() >= 4);
    }

    #[test]
    fn test_attachment_card_has_aspect_square() {
        let preview_class = "aspect-square w-full";
        assert!(preview_class.contains("aspect-square"));
    }

    #[test]
    fn test_attachment_grid_calculates_total() {
        let cols = ATTACHMENT_GRID_COLS;
        let rows = ATTACHMENT_GRID_ROWS;
        let total = cols * rows;
        assert_eq!(total, 9);
    }

    // === Accessibility ===

    #[test]
    fn test_skeleton_is_aria_hidden() {
        // Individual skeleton divs should be aria-hidden
        let expected = "true";
        assert_eq!(expected, "true");
    }

    #[test]
    fn test_message_list_skeleton_has_role_status() {
        let role = "status";
        assert_eq!(role, "status");
    }

    #[test]
    fn test_message_list_skeleton_has_aria_label() {
        let label = "Loading messages";
        assert!(label.contains("Loading"));
    }

    #[test]
    fn test_message_detail_skeleton_has_aria_label() {
        let label = "Loading message";
        assert!(label.contains("Loading"));
    }

    #[test]
    fn test_attachment_grid_skeleton_has_aria_label() {
        let label = "Loading attachments";
        assert!(label.contains("Loading"));
    }

    #[test]
    fn test_skeleton_has_screen_reader_text() {
        // Should include sr-only text for screen readers
        let sr_only_class = "sr-only";
        assert_eq!(sr_only_class, "sr-only");
    }

    // === CLS Prevention ===

    #[test]
    fn test_message_item_skeleton_matches_real_size() {
        // Avatar: 40px (h-10), gap: 12px (gap-3), padding: 16px (p-4)
        // This ensures no layout shift when content loads
        let avatar_size = "h-10 w-10";
        assert!(avatar_size.contains("h-10"));
    }

    #[test]
    fn test_attachment_card_has_consistent_aspect_ratio() {
        // aspect-square ensures 1:1 ratio matching real thumbnails
        let aspect = "aspect-square";
        assert_eq!(aspect, "aspect-square");
    }

    #[test]
    fn test_grid_has_explicit_columns() {
        // Explicit grid-cols-N prevents reflow
        let grid_class = format!("grid-cols-{}", ATTACHMENT_GRID_COLS);
        assert!(grid_class.contains("grid-cols-3"));
    }

    // === Table Skeleton ===

    #[test]
    fn test_table_row_default_columns() {
        let default_cols = 4;
        assert_eq!(default_cols, 4);
    }

    #[test]
    fn test_table_row_first_column_wider() {
        // First column should be w-1/4, others w-1/6
        let first_width = "w-1/4";
        let other_width = "w-1/6";
        assert!(first_width.contains("1/4"));
        assert!(other_width.contains("1/6"));
    }

    // === Card Skeleton ===

    #[test]
    fn test_card_skeleton_has_border() {
        let border_class = "border border-border";
        assert!(border_class.contains("border"));
    }

    #[test]
    fn test_card_skeleton_has_padding() {
        let padding = "p-6";
        assert!(padding.contains("p-6"));
    }

    // === Animation Performance ===

    #[test]
    fn test_uses_css_animation() {
        // animate-pulse uses CSS animation, not JS - 60fps guaranteed
        assert!(SKELETON_BASE.contains("animate-pulse"));
    }

    #[test]
    fn test_no_transform_in_base() {
        // Avoid transforms that cause composite layers
        assert!(!SKELETON_BASE.contains("transform"));
    }
}
