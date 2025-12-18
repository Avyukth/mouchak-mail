//! Comprehensive Filter Bar component for inbox views.
//!
//! Provides search, filter dropdowns, view controls, and message count.
//! Responsive design with mobile bottom sheet support.

use leptos::prelude::*;

/// Filter state for the inbox
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FilterState {
    /// Search query
    pub query: String,
    /// Selected project (None = all)
    pub project: Option<String>,
    /// Selected sender (None = all)
    pub sender: Option<String>,
    /// Importance filter (None = all)
    pub importance: Option<String>,
    /// Show threaded view
    pub threaded: bool,
    /// View mode: "list" or "grid"
    pub view_mode: String,
}

impl FilterState {
    pub fn new() -> Self {
        Self {
            view_mode: "list".to_string(),
            ..Default::default()
        }
    }

    /// Check if any filter is active
    pub fn has_filters(&self) -> bool {
        !self.query.is_empty()
            || self.project.is_some()
            || self.sender.is_some()
            || self.importance.is_some()
    }

    /// Clear all filters
    pub fn clear(&mut self) {
        self.query.clear();
        self.project = None;
        self.sender = None;
        self.importance = None;
    }
}

/// Importance options for the filter
const IMPORTANCE_OPTIONS: &[(&str, &str)] = &[
    ("", "All"),
    ("high", "High"),
    ("normal", "Normal"),
    ("low", "Low"),
];

/// Comprehensive filter bar component.
///
/// # Props
/// - `on_filter_change`: Callback when any filter changes
/// - `message_count`: Number of messages to display
/// - `projects`: Available project options
/// - `senders`: Available sender options
///
/// # Example
/// ```rust,ignore
/// let filter_state = RwSignal::new(FilterState::new());
/// view! {
///     <FilterBar
///         filter_state=filter_state
///         message_count=Signal::derive(|| 42)
///         projects=vec!["project-a".to_string()]
///         senders=vec!["worker-1".to_string()]
///     />
/// }
/// ```
#[component]
pub fn FilterBar(
    /// Filter state signal (two-way binding)
    filter_state: RwSignal<FilterState>,
    /// Message count to display
    #[prop(into)]
    message_count: Signal<usize>,
    /// Available projects for dropdown
    #[prop(default = vec![])]
    projects: Vec<String>,
    /// Available senders for dropdown
    #[prop(default = vec![])]
    senders: Vec<String>,
) -> impl IntoView {
    // Mobile filters sheet visibility
    let show_filters_sheet = RwSignal::new(false);

    // Clone for closures
    let projects_for_select = projects.clone();
    let senders_for_select = senders.clone();

    // Search input handler with debounce
    let on_search_input = move |ev: web_sys::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        let value = target.value();
        filter_state.update(|s| s.query = value);
    };

    // Project filter handler
    let on_project_change = move |ev: web_sys::Event| {
        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
        let value = target.value();
        filter_state.update(|s| {
            s.project = if value.is_empty() { None } else { Some(value) };
        });
    };

    // Sender filter handler
    let on_sender_change = move |ev: web_sys::Event| {
        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
        let value = target.value();
        filter_state.update(|s| {
            s.sender = if value.is_empty() { None } else { Some(value) };
        });
    };

    // Importance filter handler
    let on_importance_change = move |ev: web_sys::Event| {
        let target = event_target::<web_sys::HtmlSelectElement>(&ev);
        let value = target.value();
        filter_state.update(|s| {
            s.importance = if value.is_empty() { None } else { Some(value) };
        });
    };

    // View mode toggle
    let set_list_view = move |_| {
        filter_state.update(|s| s.view_mode = "list".to_string());
    };

    let set_grid_view = move |_| {
        filter_state.update(|s| s.view_mode = "grid".to_string());
    };

    // Clear all filters
    let clear_filters = move |_| {
        filter_state.update(|s| s.clear());
    };

    view! {
        <div class="flex flex-col gap-3">
            // Desktop: Single row layout
            <div class="hidden md:flex items-center gap-3 p-3 bg-cream-50 dark:bg-charcoal-800 rounded-xl border border-cream-200 dark:border-charcoal-700">
                // Search Input
                <div class="relative flex-1">
                    <i data-lucide="search" class="absolute left-3 top-1/2 -translate-y-1/2 icon-sm text-charcoal-400"></i>
                    <input
                        type="text"
                        placeholder="Search messages... (⌘K)"
                        class="w-full pl-10 pr-4 py-2 bg-white dark:bg-charcoal-900 border border-cream-200 dark:border-charcoal-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500/50"
                        prop:value={move || filter_state.get().query}
                        on:input=on_search_input
                    />
                </div>

                // Filter Dropdowns
                <select
                    class="px-3 py-2 bg-white dark:bg-charcoal-900 border border-cream-200 dark:border-charcoal-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500/50"
                    on:change=on_project_change
                >
                    <option value="">"All Projects"</option>
                    {projects_for_select.iter().map(|p| {
                        let p_value = p.clone();
                        let p_text = p.clone();
                        view! { <option value={p_value}>{p_text}</option> }
                    }).collect::<Vec<_>>()}
                </select>

                <select
                    class="px-3 py-2 bg-white dark:bg-charcoal-900 border border-cream-200 dark:border-charcoal-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500/50"
                    on:change=on_sender_change
                >
                    <option value="">"All Senders"</option>
                    {senders_for_select.iter().map(|s| {
                        let s_value = s.clone();
                        let s_text = s.clone();
                        view! { <option value={s_value}>{s_text}</option> }
                    }).collect::<Vec<_>>()}
                </select>

                <select
                    class="px-3 py-2 bg-white dark:bg-charcoal-900 border border-cream-200 dark:border-charcoal-600 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500/50"
                    on:change=on_importance_change
                >
                    {IMPORTANCE_OPTIONS.iter().map(|(value, label)| {
                        view! { <option value={*value}>{*label}</option> }
                    }).collect::<Vec<_>>()}
                </select>

                // Clear Filters Button (shown when filters active)
                {move || {
                    if filter_state.get().has_filters() {
                        Some(view! {
                            <button
                                class="px-3 py-2 text-sm text-amber-600 hover:text-amber-700 dark:text-amber-400 flex items-center gap-1"
                                on:click=clear_filters
                            >
                                <i data-lucide="x" class="icon-xs"></i>
                                "Clear"
                            </button>
                        })
                    } else {
                        None
                    }
                }}

                // View Mode Toggle
                <div class="flex items-center gap-1 border-l border-cream-200 dark:border-charcoal-600 pl-3">
                    <button
                        class={move || format!(
                            "p-2 rounded {}",
                            if filter_state.get().view_mode == "list" {
                                "bg-amber-100 dark:bg-amber-900/30 text-amber-600"
                            } else {
                                "text-charcoal-400 hover:text-charcoal-600"
                            }
                        )}
                        on:click=set_list_view
                        title="List view"
                    >
                        <i data-lucide="list" class="icon-sm"></i>
                    </button>
                    <button
                        class={move || format!(
                            "p-2 rounded {}",
                            if filter_state.get().view_mode == "grid" {
                                "bg-amber-100 dark:bg-amber-900/30 text-amber-600"
                            } else {
                                "text-charcoal-400 hover:text-charcoal-600"
                            }
                        )}
                        on:click=set_grid_view
                        title="Grid view"
                    >
                        <i data-lucide="grid" class="icon-sm"></i>
                    </button>
                </div>

                // Message Count Badge
                <div class="text-sm text-charcoal-500 dark:text-charcoal-400 whitespace-nowrap">
                    {move || format!("{} messages", message_count.get())}
                </div>
            </div>

            // Mobile: Compact layout
            <div class="md:hidden space-y-2">
                // Search (full width)
                <div class="relative">
                    <i data-lucide="search" class="absolute left-3 top-1/2 -translate-y-1/2 icon-sm text-charcoal-400"></i>
                    <input
                        type="text"
                        placeholder="Search..."
                        class="w-full pl-10 pr-4 py-2 bg-white dark:bg-charcoal-900 border border-cream-200 dark:border-charcoal-600 rounded-lg text-sm"
                        prop:value={move || filter_state.get().query}
                        on:input=on_search_input
                    />
                </div>

                // Filters button + count
                <div class="flex items-center justify-between">
                    <button
                        class="flex items-center gap-2 px-3 py-2 bg-cream-100 dark:bg-charcoal-800 rounded-lg text-sm"
                        on:click=move |_| show_filters_sheet.set(true)
                    >
                        <i data-lucide="sliders" class="icon-sm"></i>
                        "Filters"
                        {move || {
                            if filter_state.get().has_filters() {
                                Some(view! {
                                    <span class="px-1.5 py-0.5 bg-amber-500 text-white text-xs rounded-full">
                                        "!"
                                    </span>
                                })
                            } else {
                                None
                            }
                        }}
                    </button>

                    <span class="text-sm text-charcoal-500">
                        {move || format!("{} messages", message_count.get())}
                    </span>
                </div>
            </div>

            // Mobile Bottom Sheet (simplified - full implementation would use portal)
            {move || {
                if show_filters_sheet.get() {
                    Some(view! {
                        <div
                            class="fixed inset-0 bg-black/50 z-50 md:hidden"
                            on:click=move |_| show_filters_sheet.set(false)
                        >
                            <div
                                class="absolute bottom-0 left-0 right-0 bg-white dark:bg-charcoal-900 rounded-t-2xl p-4 space-y-4"
                                on:click=|e| e.stop_propagation()
                            >
                                <div class="flex items-center justify-between">
                                    <h3 class="font-semibold">"Filters"</h3>
                                    <button
                                        class="p-2"
                                        on:click=move |_| show_filters_sheet.set(false)
                                    >
                                        <i data-lucide="x" class="icon-sm"></i>
                                    </button>
                                </div>

                                <div class="space-y-3">
                                    <select class="w-full px-3 py-2 border rounded-lg">
                                        <option>"All Projects"</option>
                                    </select>
                                    <select class="w-full px-3 py-2 border rounded-lg">
                                        <option>"All Senders"</option>
                                    </select>
                                    <select class="w-full px-3 py-2 border rounded-lg">
                                        <option>"All Importance"</option>
                                    </select>
                                </div>

                                <button
                                    class="w-full py-3 bg-amber-500 text-white rounded-lg font-medium"
                                    on:click=move |_| show_filters_sheet.set(false)
                                >
                                    "Apply Filters"
                                </button>
                            </div>
                        </div>
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
    fn test_filter_state_default() {
        let state = FilterState::new();
        assert_eq!(state.query, "");
        assert_eq!(state.project, None);
        assert_eq!(state.sender, None);
        assert_eq!(state.importance, None);
        assert_eq!(state.view_mode, "list");
        assert!(!state.threaded);
    }

    #[test]
    fn test_filter_state_has_filters() {
        let mut state = FilterState::new();
        assert!(!state.has_filters());

        state.query = "test".to_string();
        assert!(state.has_filters());

        state.query.clear();
        state.project = Some("myproj".to_string());
        assert!(state.has_filters());

        state.project = None;
        state.importance = Some("high".to_string());
        assert!(state.has_filters());
    }

    #[test]
    fn test_filter_state_clear() {
        let mut state = FilterState::new();
        state.query = "search".to_string();
        state.project = Some("proj".to_string());
        state.sender = Some("agent".to_string());
        state.importance = Some("high".to_string());

        state.clear();

        assert_eq!(state.query, "");
        assert_eq!(state.project, None);
        assert_eq!(state.sender, None);
        assert_eq!(state.importance, None);
    }

    #[test]
    fn test_importance_options_contains_all() {
        assert!(IMPORTANCE_OPTIONS.iter().any(|(v, _)| v.is_empty()));
        assert!(IMPORTANCE_OPTIONS.iter().any(|(v, _)| *v == "high"));
        assert!(IMPORTANCE_OPTIONS.iter().any(|(v, _)| *v == "normal"));
        assert!(IMPORTANCE_OPTIONS.iter().any(|(v, _)| *v == "low"));
    }
}
