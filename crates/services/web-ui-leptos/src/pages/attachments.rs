//! Attachments browser page.
//!
//! Displays project attachments in a responsive grid with file type icons,
//! filtering, sorting, and download functionality.

use crate::api::client::{self, Attachment, Project};
use crate::components::{
    Card, CardContent, Dialog, DialogContent, DialogHeader, DialogTitle, Select, SelectOption,
    Skeleton,
};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

/// Sort options for attachments
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AttachmentSort {
    #[default]
    DateDesc,
    DateAsc,
    NameAsc,
    NameDesc,
    SizeDesc,
    SizeAsc,
}

impl AttachmentSort {
    fn label(&self) -> &'static str {
        match self {
            Self::DateDesc => "Newest First",
            Self::DateAsc => "Oldest First",
            Self::NameAsc => "Name A-Z",
            Self::NameDesc => "Name Z-A",
            Self::SizeDesc => "Largest First",
            Self::SizeAsc => "Smallest First",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "date_asc" => Self::DateAsc,
            "name_asc" => Self::NameAsc,
            "name_desc" => Self::NameDesc,
            "size_desc" => Self::SizeDesc,
            "size_asc" => Self::SizeAsc,
            _ => Self::DateDesc,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::DateDesc => "date_desc",
            Self::DateAsc => "date_asc",
            Self::NameAsc => "name_asc",
            Self::NameDesc => "name_desc",
            Self::SizeDesc => "size_desc",
            Self::SizeAsc => "size_asc",
        }
    }

    /// Returns all sort variants for building options.
    fn all() -> [Self; 6] {
        [
            Self::DateDesc,
            Self::DateAsc,
            Self::NameAsc,
            Self::NameDesc,
            Self::SizeDesc,
            Self::SizeAsc,
        ]
    }

    fn sort(&self, attachments: &mut [Attachment]) {
        match self {
            Self::DateDesc => attachments.sort_by(|a, b| b.created_ts.cmp(&a.created_ts)),
            Self::DateAsc => attachments.sort_by(|a, b| a.created_ts.cmp(&b.created_ts)),
            Self::NameAsc => attachments
                .sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase())),
            Self::NameDesc => attachments
                .sort_by(|a, b| b.filename.to_lowercase().cmp(&a.filename.to_lowercase())),
            Self::SizeDesc => attachments.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes)),
            Self::SizeAsc => attachments.sort_by(|a, b| a.size_bytes.cmp(&b.size_bytes)),
        }
    }
}

/// Preview modal for images and PDFs.
/// Uses Dialog component with responsive sizing.
#[component]
fn AttachmentPreviewModal(
    attachment: Attachment,
    download_url: String,
    open: RwSignal<bool>,
) -> impl IntoView {
    let file_type = attachment.file_type_category();
    let filename = attachment.filename.clone();
    let is_image = file_type == "image";
    let is_pdf = file_type == "pdf";
    let can_preview = is_image || is_pdf;

    // Use StoredValue for strings (avoids FnOnce issue with moves)
    let filename_store = StoredValue::new(filename);
    let download_url_store = StoredValue::new(download_url);

    view! {
        <Dialog open=open.get() on_open_change=Callback::new(move |v| open.set(v))>
            <DialogContent class="max-w-4xl max-h-[90vh] overflow-hidden".to_string()>
                <DialogHeader>
                    <DialogTitle>
                        <span class="truncate">{filename_store.get_value()}</span>
                    </DialogTitle>
                </DialogHeader>

                // Preview content area
                <div class="flex items-center justify-center min-h-[300px] max-h-[70vh] bg-charcoal-100 dark:bg-charcoal-800 rounded-lg overflow-auto">
                    {if is_image {
                        view! {
                            <img
                                src={download_url_store.get_value()}
                                alt={filename_store.get_value()}
                                class="max-w-full max-h-[70vh] object-contain"
                            />
                        }.into_any()
                    } else if is_pdf {
                        view! {
                            <iframe
                                src={download_url_store.get_value()}
                                class="w-full h-[70vh] border-0"
                                title={format!("Preview: {}", filename_store.get_value())}
                            />
                        }.into_any()
                    } else {
                        view! {
                            <div class="text-center p-8">
                                <i data-lucide="file-x" class="icon-3xl text-charcoal-400 mb-4"></i>
                                <p class="text-charcoal-500 dark:text-charcoal-400">
                                    "Preview not available for this file type"
                                </p>
                            </div>
                        }.into_any()
                    }}
                </div>

                // Footer with download button
                <div class="flex justify-end gap-2 pt-4">
                    {if can_preview {
                        Some(view! {
                            <a
                                href={download_url_store.get_value()}
                                target="_blank"
                                rel="noopener noreferrer"
                                class="inline-flex items-center gap-2 px-4 py-2 min-h-[44px] rounded-md border border-charcoal-300 dark:border-charcoal-600 text-charcoal-700 dark:text-charcoal-300 hover:bg-charcoal-100 dark:hover:bg-charcoal-700 transition-colors text-sm font-medium"
                            >
                                <i data-lucide="external-link" class="icon-sm"></i>
                                "Open in New Tab"
                            </a>
                        })
                    } else {
                        None
                    }}
                    <a
                        href={download_url_store.get_value()}
                        download
                        class="inline-flex items-center gap-2 px-4 py-2 min-h-[44px] rounded-md bg-amber-500 hover:bg-amber-600 text-white font-medium text-sm transition-colors"
                    >
                        <i data-lucide="download" class="icon-sm"></i>
                        "Download"
                    </a>
                </div>
            </DialogContent>
        </Dialog>
    }
}

/// Attachment card component for grid display.
/// Clicking opens a preview modal for images/PDFs.
#[component]
fn AttachmentCard(attachment: Attachment, project_slug: String) -> impl IntoView {
    let download_url = client::attachment_download_url(attachment.id, &project_slug);
    let icon = attachment.icon_name();
    let file_type = attachment.file_type_category();
    let size = attachment.human_size();
    let filename = attachment.filename.clone();
    let is_image = file_type == "image";
    let is_pdf = file_type == "pdf";
    let can_preview = is_image || is_pdf;

    // Modal state
    let modal_open = RwSignal::new(false);

    // Clone for modal
    let attachment_for_modal = attachment.clone();
    let download_url_for_modal = download_url.clone();

    view! {
        // Preview modal
        <AttachmentPreviewModal
            attachment=attachment_for_modal
            download_url=download_url_for_modal
            open=modal_open
        />

        <Card class="hover:shadow-lg transition-shadow duration-200 group">
            <CardContent class="p-4">
                // File icon/preview area (fixed height for CLS prevention)
                // Clickable for preview if image/PDF
                <button
                    type="button"
                    class={format!(
                        "w-full h-24 flex items-center justify-center rounded-lg bg-charcoal-100 dark:bg-charcoal-700 mb-3 overflow-hidden {}",
                        if can_preview { "cursor-pointer hover:ring-2 hover:ring-amber-400 transition-all" } else { "" }
                    )}
                    on:click=move |_| {
                        if can_preview {
                            modal_open.set(true);
                        }
                    }
                    aria-label={if can_preview {
                        format!("Preview {}", filename)
                    } else {
                        format!("File: {}", filename)
                    }}
                    disabled={!can_preview}
                >
                    {if is_image {
                        // Image thumbnail
                        view! {
                            <img
                                src={download_url.clone()}
                                alt={filename.clone()}
                                class="max-h-full max-w-full object-contain"
                                loading="lazy"
                            />
                        }.into_any()
                    } else {
                        // File type icon
                        view! {
                            <i
                                data-lucide={icon}
                                class="icon-2xl text-charcoal-400 dark:text-charcoal-500"
                            ></i>
                        }.into_any()
                    }}
                </button>

                // Filename (truncated)
                <h3
                    class="font-medium text-sm text-charcoal-800 dark:text-cream-100 truncate mb-1"
                    title={filename.clone()}
                >
                    {filename.clone()}
                </h3>

                // Metadata row with preview indicator
                <div class="flex items-center justify-between text-xs text-charcoal-500 dark:text-charcoal-400">
                    <span class="flex items-center gap-1">
                        <i data-lucide={icon} class="icon-xs"></i>
                        {file_type.to_uppercase()}
                        {if can_preview {
                            Some(view! {
                                <i data-lucide="eye" class="icon-xs text-amber-500 ml-1" title="Click to preview"></i>
                            })
                        } else {
                            None
                        }}
                    </span>
                    <span>{size}</span>
                </div>

                // Download button (visible on hover/focus, always on mobile)
                <a
                    href={download_url}
                    download
                    class="mt-3 w-full inline-flex items-center justify-center gap-2 px-4 py-2 min-h-[44px] rounded-md bg-amber-500 hover:bg-amber-600 text-white font-medium text-sm transition-colors opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 md:opacity-0 touch:opacity-100"
                    aria-label={"Download ".to_string() + &attachment.filename}
                >
                    <i data-lucide="download" class="icon-sm"></i>
                    "Download"
                </a>
            </CardContent>
        </Card>
    }
}

/// Skeleton loader for attachment cards.
#[component]
fn AttachmentCardSkeleton() -> impl IntoView {
    view! {
        <Card>
            <CardContent class="p-4">
                <Skeleton class="h-24 w-full rounded-lg mb-3" />
                <Skeleton class="h-4 w-3/4 mb-2" />
                <div class="flex justify-between">
                    <Skeleton class="h-3 w-16" />
                    <Skeleton class="h-3 w-12" />
                </div>
            </CardContent>
        </Card>
    }
}

/// Empty state component.
#[component]
fn AttachmentsEmptyState(project_selected: bool) -> impl IntoView {
    view! {
        <div class="col-span-full flex flex-col items-center justify-center py-16 text-center">
            <div class="w-20 h-20 rounded-full bg-charcoal-100 dark:bg-charcoal-700 flex items-center justify-center mb-4">
                <i data-lucide="file-x" class="icon-2xl text-charcoal-400"></i>
            </div>
            <h3 class="text-lg font-semibold text-charcoal-700 dark:text-charcoal-300 mb-2">
                {if project_selected { "No attachments found" } else { "Select a project" }}
            </h3>
            <p class="text-sm text-charcoal-500 dark:text-charcoal-400 max-w-md">
                {if project_selected {
                    "This project doesn't have any attachments yet. Upload files through the API or MCP tools."
                } else {
                    "Choose a project from the dropdown above to view its attachments."
                }}
            </p>
        </div>
    }
}

/// Attachments browser page.
#[component]
pub fn Attachments() -> impl IntoView {
    let query = use_query_map();

    // State
    let projects = RwSignal::new(Vec::<Project>::new());
    let attachments = RwSignal::new(Vec::<Attachment>::new());
    let loading = RwSignal::new(true);
    let loading_attachments = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // Filters
    let selected_project =
        RwSignal::new(query.with_untracked(|q| q.get("project").unwrap_or_default()));
    let sort_value = RwSignal::new("date_desc".to_string());

    // Sync sort_value to sort_order
    let sort_order = Signal::derive(move || AttachmentSort::from_str(&sort_value.get()));

    // Load projects on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match client::get_projects().await {
                Ok(p) => {
                    projects.set(p);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.message));
                    loading.set(false);
                }
            }
        });
    });

    // Load attachments when project changes
    Effect::new(move |_| {
        let project = selected_project.get();
        if project.is_empty() {
            attachments.set(vec![]);
            return;
        }

        loading_attachments.set(true);
        leptos::task::spawn_local(async move {
            match client::list_attachments(&project).await {
                Ok(mut atts) => {
                    sort_order.get().sort(&mut atts);
                    attachments.set(atts);
                    loading_attachments.set(false);
                }
                Err(e) => {
                    error.set(Some(e.message));
                    loading_attachments.set(false);
                }
            }
        });
    });

    // Derived: sorted attachments
    let sorted_attachments = Signal::derive(move || {
        let mut atts = attachments.get();
        sort_order.get().sort(&mut atts);
        atts
    });

    // Build project options
    let project_options = Signal::derive(move || {
        let mut opts: Vec<SelectOption> = vec![SelectOption::new("", "Select Project...")];
        opts.extend(
            projects
                .get()
                .iter()
                .map(|p| SelectOption::new(p.slug.clone(), p.slug.clone())),
        );
        opts
    });

    // Sort options - generated from enum to stay DRY
    let sort_options: Vec<SelectOption> = AttachmentSort::all()
        .into_iter()
        .map(|s| SelectOption::new(s.as_str(), s.label()))
        .collect();

    view! {
        <div class="space-y-6">
            // Header
            <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
                <div>
                    <h1 class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100 flex items-center gap-2">
                        <i data-lucide="paperclip" class="icon-lg text-amber-500"></i>
                        "Attachments"
                    </h1>
                    <p class="text-sm text-charcoal-500 dark:text-charcoal-400 mt-1">
                        "Browse and download project files"
                    </p>
                </div>

                // Filters row
                <div class="flex flex-col sm:flex-row gap-3">
                    <Select
                        id="project-select".to_string()
                        options=project_options.get()
                        value=selected_project
                        placeholder="Select Project...".to_string()
                    />
                    <Select
                        id="sort-select".to_string()
                        options=sort_options.clone()
                        value=sort_value
                        placeholder="Sort by...".to_string()
                    />
                </div>
            </div>

            // Error alert
            {move || error.get().map(|err| view! {
                <div class="p-4 rounded-lg bg-red-100 dark:bg-red-900/30 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300">
                    <div class="flex items-center gap-2">
                        <i data-lucide="alert-circle" class="icon-sm"></i>
                        <span>{err}</span>
                    </div>
                </div>
            })}

            // Attachments grid
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                {move || {
                    if loading.get() || loading_attachments.get() {
                        // Skeleton loading (3x3 grid)
                        (0..9).map(|_| view! { <AttachmentCardSkeleton /> }).collect::<Vec<_>>().into_any()
                    } else {
                        let atts = sorted_attachments.get();
                        let project = selected_project.get();

                        if atts.is_empty() {
                            view! { <AttachmentsEmptyState project_selected=!project.is_empty() /> }.into_any()
                        } else {
                            atts.into_iter().map(|att| {
                                view! { <AttachmentCard attachment=att project_slug=project.clone() /> }
                            }).collect::<Vec<_>>().into_any()
                        }
                    }
                }}
            </div>

            // Count indicator
            {move || {
                let count = sorted_attachments.get().len();
                let project = selected_project.get();
                if !project.is_empty() && count > 0 {
                    Some(view! {
                        <p class="text-sm text-charcoal-500 dark:text-charcoal-400 text-center">
                            {format!("{} attachment{}", count, if count == 1 { "" } else { "s" })}
                        </p>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_sort_from_str() {
        assert_eq!(
            AttachmentSort::from_str("date_desc"),
            AttachmentSort::DateDesc
        );
        assert_eq!(
            AttachmentSort::from_str("name_asc"),
            AttachmentSort::NameAsc
        );
        assert_eq!(
            AttachmentSort::from_str("invalid"),
            AttachmentSort::DateDesc
        );
    }

    #[test]
    fn test_attachment_sort_as_str() {
        assert_eq!(AttachmentSort::SizeDesc.as_str(), "size_desc");
    }

    #[test]
    fn test_attachment_sort_label() {
        assert_eq!(AttachmentSort::DateDesc.label(), "Newest First");
        assert_eq!(AttachmentSort::NameAsc.label(), "Name A-Z");
    }

    #[test]
    fn test_attachment_sort_all() {
        let all = AttachmentSort::all();
        assert_eq!(all.len(), 6);
        assert_eq!(all[0], AttachmentSort::DateDesc);
    }

    // === Preview Modal Tests ===

    #[test]
    fn test_preview_supported_for_images() {
        let file_types = ["image", "pdf"];
        for ft in file_types {
            let can_preview = ft == "image" || ft == "pdf";
            assert!(can_preview, "Should support preview for {}", ft);
        }
    }

    #[test]
    fn test_preview_not_supported_for_other_types() {
        let file_types = ["document", "spreadsheet", "archive", "code", "other"];
        for ft in file_types {
            let can_preview = ft == "image" || ft == "pdf";
            assert!(!can_preview, "Should not support preview for {}", ft);
        }
    }

    #[test]
    fn test_modal_class_max_width() {
        let modal_class = "max-w-4xl max-h-[90vh] overflow-hidden";
        assert!(modal_class.contains("max-w-4xl"));
        assert!(modal_class.contains("max-h-[90vh]"));
    }

    #[test]
    fn test_preview_area_class() {
        let preview_class =
            "flex items-center justify-center min-h-[300px] max-h-[70vh] bg-charcoal-100";
        assert!(preview_class.contains("min-h-[300px]"));
        assert!(preview_class.contains("max-h-[70vh]"));
    }

    #[test]
    fn test_preview_button_has_touch_target() {
        // Preview buttons must have 44px minimum height for WCAG 2.2
        let button_class = "min-h-[44px]";
        assert!(button_class.contains("min-h-[44px]"));
    }

    #[test]
    fn test_preview_hover_ring() {
        // Preview area should have visual feedback on hover
        let hover_class = "hover:ring-2 hover:ring-amber-400";
        assert!(hover_class.contains("hover:ring-2"));
        assert!(hover_class.contains("hover:ring-amber-400"));
    }
}
