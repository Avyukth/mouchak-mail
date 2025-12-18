//! Comprehensive Filter Bar component for inbox views.
//!
//! Provides search, filter dropdowns, view controls, and message count.
//! Responsive design with mobile bottom sheet support.

use super::{Button, ButtonVariant, Input, Select, SelectOption};
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

    /// Create FilterState from URL query parameters
    pub fn from_query_params(params: &std::collections::HashMap<String, String>) -> Self {
        Self {
            query: params.get("q").cloned().unwrap_or_default(),
            project: params.get("project").filter(|s| !s.is_empty()).cloned(),
            sender: params.get("sender").filter(|s| !s.is_empty()).cloned(),
            importance: params.get("importance").filter(|s| !s.is_empty()).cloned(),
            threaded: params.get("threaded").map_or(false, |v| v == "true"),
            view_mode: params
                .get("view")
                .cloned()
                .unwrap_or_else(|| "list".to_string()),
        }
    }

    /// Convert FilterState to URL query string (without leading ?)
    pub fn to_query_string(&self) -> String {
        let mut params = Vec::new();

        if !self.query.is_empty() {
            params.push(format!("q={}", urlencoding::encode(&self.query)));
        }
        if let Some(ref p) = self.project {
            params.push(format!("project={}", urlencoding::encode(p)));
        }
        if let Some(ref s) = self.sender {
            params.push(format!("sender={}", urlencoding::encode(s)));
        }
        if let Some(ref i) = self.importance {
            params.push(format!("importance={}", urlencoding::encode(i)));
        }
        if self.threaded {
            params.push("threaded=true".to_string());
        }
        if self.view_mode != "list" {
            params.push(format!("view={}", &self.view_mode));
        }

        params.join("&")
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

    // Local signals for Select components (sync with filter_state)
    let project_value = RwSignal::new(String::new());
    let sender_value = RwSignal::new(String::new());
    let importance_value = RwSignal::new(String::new());
    let search_value = RwSignal::new(String::new());

    // Sync from filter_state on mount
    Effect::new(move |_| {
        let state = filter_state.get();
        project_value.set(state.project.clone().unwrap_or_default());
        sender_value.set(state.sender.clone().unwrap_or_default());
        importance_value.set(state.importance.clone().unwrap_or_default());
        search_value.set(state.query.clone());
    });

    // Sync project changes to filter_state
    Effect::new(move |prev: Option<String>| {
        let val = project_value.get();
        if prev.is_some() {
            filter_state.update(|s| {
                s.project = if val.is_empty() {
                    None
                } else {
                    Some(val.clone())
                };
            });
        }
        val
    });

    // Sync sender changes to filter_state
    Effect::new(move |prev: Option<String>| {
        let val = sender_value.get();
        if prev.is_some() {
            filter_state.update(|s| {
                s.sender = if val.is_empty() {
                    None
                } else {
                    Some(val.clone())
                };
            });
        }
        val
    });

    // Sync importance changes to filter_state
    Effect::new(move |prev: Option<String>| {
        let val = importance_value.get();
        if prev.is_some() {
            filter_state.update(|s| {
                s.importance = if val.is_empty() {
                    None
                } else {
                    Some(val.clone())
                };
            });
        }
        val
    });

    // View mode toggle
    let set_list_view = move |_| {
        filter_state.update(|s| s.view_mode = "list".to_string());
    };

    let set_grid_view = move |_| {
        filter_state.update(|s| s.view_mode = "grid".to_string());
    };

    // Clear all filters
    let clear_filters = Callback::new(move |_| {
        filter_state.update(|s| s.clear());
        project_value.set(String::new());
        sender_value.set(String::new());
        importance_value.set(String::new());
        search_value.set(String::new());
    });

    // Build options for Select components
    let project_options: Vec<SelectOption> = std::iter::once(SelectOption::new("", "All Projects"))
        .chain(
            projects
                .iter()
                .map(|p| SelectOption::new(p.clone(), p.clone())),
        )
        .collect();

    let sender_options: Vec<SelectOption> = std::iter::once(SelectOption::new("", "All Senders"))
        .chain(
            senders
                .iter()
                .map(|s| SelectOption::new(s.clone(), s.clone())),
        )
        .collect();

    let importance_options: Vec<SelectOption> = IMPORTANCE_OPTIONS
        .iter()
        .map(|(v, l)| SelectOption::new(*v, *l))
        .collect();

    view! {
        <div class="flex flex-col gap-3">
            // Desktop: Single row layout
            <div class="hidden md:flex items-center gap-3 p-3 bg-cream-50 dark:bg-charcoal-800 rounded-xl border border-cream-200 dark:border-charcoal-700">
                // Search Input with icon
                <div class="relative flex-1">
                    <i data-lucide="search" class="absolute left-3 top-1/2 -translate-y-1/2 icon-sm text-charcoal-400 z-10"></i>
                    <Input
                        id="filterSearch".to_string()
                        value=search_value
                        placeholder="Search messages... (⌘K)".to_string()
                        class="pl-10".to_string()
                        on_input=Callback::new(move |v: String| {
                            search_value.set(v.clone());
                            filter_state.update(|s| s.query = v);
                        })
                    />
                </div>

                // Filter Dropdowns using Select component
                <div class="w-44">
                    <Select
                        id="projectFilter".to_string()
                        options=project_options.clone()
                        value=project_value
                        placeholder="All Projects".to_string()
                        icon="folder"
                    />
                </div>

                <div class="w-40">
                    <Select
                        id="senderFilter".to_string()
                        options=sender_options.clone()
                        value=sender_value
                        placeholder="All Senders".to_string()
                        icon="user"
                    />
                </div>

                <div class="w-36">
                    <Select
                        id="importanceFilter".to_string()
                        options=importance_options.clone()
                        value=importance_value
                        placeholder="Importance".to_string()
                        icon="alert-circle"
                    />
                </div>

                // Clear Filters Button (shown when filters active)
                {move || {
                    if filter_state.get().has_filters() {
                        Some(view! {
                            <Button
                                variant=ButtonVariant::Ghost
                                on_click=clear_filters
                            >
                                <i data-lucide="x" class="icon-xs"></i>
                                <span>"Clear"</span>
                            </Button>
                        })
                    } else {
                        None
                    }
                }}

                // View Mode Toggle
                <div class="flex items-center gap-1 border-l border-cream-200 dark:border-charcoal-600 pl-3">
                    <Button
                        variant={if filter_state.get().view_mode == "list" { ButtonVariant::Secondary } else { ButtonVariant::Ghost }}
                        size=super::ButtonSize::Icon
                        on_click=Callback::new(set_list_view)
                        title="List view"
                    >
                        <i data-lucide="list" class="icon-sm"></i>
                    </Button>
                    <Button
                        variant={if filter_state.get().view_mode == "grid" { ButtonVariant::Secondary } else { ButtonVariant::Ghost }}
                        size=super::ButtonSize::Icon
                        on_click=Callback::new(set_grid_view)
                        title="Grid view"
                    >
                        <i data-lucide="grid" class="icon-sm"></i>
                    </Button>
                </div>

                // Message Count Badge
                <div class="text-sm text-charcoal-500 dark:text-charcoal-400 whitespace-nowrap">
                    {move || format!("{} messages", message_count.get())}
                </div>
            </div>

            // Mobile: Compact layout
            <div class="md:hidden space-y-2">
                // Search (full width) with icon
                <div class="relative">
                    <i data-lucide="search" class="absolute left-3 top-1/2 -translate-y-1/2 icon-sm text-charcoal-400 z-10"></i>
                    <Input
                        id="filterSearchMobile".to_string()
                        value=search_value
                        placeholder="Search...".to_string()
                        class="pl-10".to_string()
                        on_input=Callback::new(move |v: String| {
                            search_value.set(v.clone());
                            filter_state.update(|s| s.query = v);
                        })
                    />
                </div>

                // Filters button + count
                <div class="flex items-center justify-between">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| show_filters_sheet.set(true))
                    >
                        <i data-lucide="sliders" class="icon-sm"></i>
                        <span>"Filters"</span>
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
                    </Button>

                    <span class="text-sm text-charcoal-500">
                        {move || format!("{} messages", message_count.get())}
                    </span>
                </div>
            </div>

            // Mobile Bottom Sheet
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
                                    <h3 class="font-semibold text-charcoal-800 dark:text-cream-100">"Filters"</h3>
                                    <Button
                                        variant=ButtonVariant::Ghost
                                        size=super::ButtonSize::Icon
                                        on_click=Callback::new(move |_| show_filters_sheet.set(false))
                                    >
                                        <i data-lucide="x" class="icon-sm"></i>
                                    </Button>
                                </div>

                                <div class="space-y-3">
                                    <Select
                                        id="projectFilterMobile".to_string()
                                        options=project_options.clone()
                                        value=project_value
                                        placeholder="All Projects".to_string()
                                        icon="folder"
                                    />
                                    <Select
                                        id="senderFilterMobile".to_string()
                                        options=sender_options.clone()
                                        value=sender_value
                                        placeholder="All Senders".to_string()
                                        icon="user"
                                    />
                                    <Select
                                        id="importanceFilterMobile".to_string()
                                        options=importance_options.clone()
                                        value=importance_value
                                        placeholder="Importance".to_string()
                                        icon="alert-circle"
                                    />
                                </div>

                                <Button
                                    variant=ButtonVariant::Default
                                    class="w-full".to_string()
                                    on_click=Callback::new(move |_| show_filters_sheet.set(false))
                                >
                                    <span>"Apply Filters"</span>
                                </Button>
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

    #[test]
    fn test_from_query_params() {
        use std::collections::HashMap;

        let mut params = HashMap::new();
        params.insert("q".to_string(), "search term".to_string());
        params.insert("project".to_string(), "my-project".to_string());
        params.insert("importance".to_string(), "high".to_string());
        params.insert("view".to_string(), "grid".to_string());

        let state = FilterState::from_query_params(&params);

        assert_eq!(state.query, "search term");
        assert_eq!(state.project, Some("my-project".to_string()));
        assert_eq!(state.importance, Some("high".to_string()));
        assert_eq!(state.view_mode, "grid");
    }

    #[test]
    fn test_from_query_params_empty() {
        use std::collections::HashMap;
        let params = HashMap::new();
        let state = FilterState::from_query_params(&params);

        assert_eq!(state.query, "");
        assert_eq!(state.project, None);
        assert_eq!(state.view_mode, "list");
    }

    #[test]
    fn test_to_query_string() {
        let mut state = FilterState::new();
        state.query = "test".to_string();
        state.project = Some("proj".to_string());
        state.importance = Some("high".to_string());

        let qs = state.to_query_string();
        assert!(qs.contains("q=test"));
        assert!(qs.contains("project=proj"));
        assert!(qs.contains("importance=high"));
    }

    #[test]
    fn test_to_query_string_empty() {
        let state = FilterState::new();
        assert_eq!(state.to_query_string(), "");
    }

    #[test]
    fn test_query_string_url_encodes() {
        let mut state = FilterState::new();
        state.query = "hello world".to_string();
        let qs = state.to_query_string();
        assert!(qs.contains("q=hello%20world"));
    }
}
