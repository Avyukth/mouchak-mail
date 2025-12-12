//! Projects page - list and create projects.
//! Digital Correspondence design with Lucide icons.

use leptos::prelude::*;
use crate::api::client::{self, Project};

/// Projects page component.
#[component]
pub fn Projects() -> impl IntoView {
    // State
    let projects = RwSignal::new(Vec::<Project>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);
    let show_new_form = RwSignal::new(false);
    let new_project_path = RwSignal::new(String::new());
    let creating = RwSignal::new(false);

    // Load projects on mount
    let load_projects = move || {
        loading.set(true);
        error.set(None);
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
    };

    // Initial load
    Effect::new(move |_| {
        load_projects();
    });

    // Create project handler
    let create_project = move |_| {
        let path = new_project_path.get();
        if path.trim().is_empty() {
            return;
        }

        creating.set(true);
        error.set(None);
        
        leptos::task::spawn_local(async move {
            match client::ensure_project(&path).await {
                Ok(_) => {
                    // Reload projects
                    match client::get_projects().await {
                        Ok(p) => {
                            projects.set(p);
                        }
                        Err(e) => {
                            error.set(Some(e.message));
                        }
                    }
                    new_project_path.set(String::new());
                    show_new_form.set(false);
                    creating.set(false);
                }
                Err(e) => {
                    error.set(Some(e.message));
                    creating.set(false);
                }
            }
        });
    };

    view! {
        <div class="space-y-6">
            // Header
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100 flex items-center gap-2">
                        <i data-lucide="folder-open" class="icon-xl text-amber-500"></i>
                        "Projects"
                    </h1>
                    <p class="text-charcoal-500 dark:text-charcoal-400">"Manage your agent mail projects"</p>
                </div>
                <button
                    on:click=move |_| show_new_form.update(|v| *v = !*v)
                    class="btn-primary flex items-center gap-2"
                >
                    <i data-lucide="folder-plus" class="icon-sm"></i>
                    <span>"New Project"</span>
                </button>
            </div>

            // New Project Form
            {move || {
                if show_new_form.get() {
                    Some(view! {
                        <div class="card-elevated p-6 animate-slide-up">
                            <h2 class="font-display text-lg font-semibold text-charcoal-800 dark:text-cream-100 mb-4 flex items-center gap-2">
                                <i data-lucide="file-plus" class="icon-lg text-amber-500"></i>
                                "Create New Project"
                            </h2>
                            <form on:submit=move |ev| { ev.prevent_default(); create_project(()); } class="space-y-4">
                                <div>
                                    <label for="projectPath" class="block text-sm font-medium text-charcoal-700 dark:text-charcoal-300 mb-2">
                                        "Project Path (human_key)"
                                    </label>
                                    <input
                                        id="projectPath"
                                        type="text"
                                        prop:value=move || new_project_path.get()
                                        on:input=move |ev| new_project_path.set(event_target_value(&ev))
                                        placeholder="/path/to/your/project"
                                        class="input"
                                    />
                                    <p class="mt-1 text-sm text-charcoal-500 dark:text-charcoal-400">
                                        "The absolute path to your project directory"
                                    </p>
                                </div>
                                <div class="flex gap-3">
                                    <button
                                        type="submit"
                                        disabled=move || creating.get() || new_project_path.get().trim().is_empty()
                                        class="btn-primary flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
                                    >
                                        {move || if creating.get() {
                                            view! { <i data-lucide="loader-2" class="icon-sm animate-spin"></i> }
                                        } else {
                                            view! { <i data-lucide="plus" class="icon-sm"></i> }
                                        }}
                                        {move || if creating.get() { "Creating..." } else { "Create Project" }}
                                    </button>
                                    <button
                                        type="button"
                                        on:click=move |_| { show_new_form.set(false); new_project_path.set(String::new()); }
                                        class="btn-secondary"
                                    >
                                        "Cancel"
                                    </button>
                                </div>
                            </form>
                        </div>
                    })
                } else {
                    None
                }
            }}

            // Error Message
            {move || {
                error.get().map(|e| view! {
                    <div class="rounded-xl border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/20 p-4 animate-slide-up">
                        <div class="flex items-start gap-3">
                            <i data-lucide="triangle-alert" class="icon-lg text-red-500"></i>
                            <p class="text-red-700 dark:text-red-400">{e}</p>
                        </div>
                    </div>
                })
            }}

            // Content: Loading / Empty / List
            {move || {
                if loading.get() {
                    // Loading State
                    view! {
                        <div class="flex items-center justify-center py-16">
                            <div class="flex flex-col items-center gap-4">
                                <i data-lucide="loader-2" class="icon-2xl text-amber-500 animate-spin"></i>
                                <p class="text-charcoal-500 dark:text-charcoal-400 text-sm">"Loading projects..."</p>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    let project_list = projects.get();
                    if project_list.is_empty() {
                        // Empty State
                        view! {
                            <div class="card-elevated p-12 text-center">
                                <div class="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-amber-100 dark:bg-amber-900/50 mb-6">
                                    <i data-lucide="folder-open" class="icon-2xl text-amber-600 dark:text-amber-400"></i>
                                </div>
                                <h3 class="font-display text-xl font-semibold text-charcoal-800 dark:text-cream-100 mb-2">"No projects yet"</h3>
                                <p class="text-charcoal-500 dark:text-charcoal-400 mb-6">
                                    "Create your first project to start sending messages between agents."
                                </p>
                                <button
                                    on:click=move |_| show_new_form.set(true)
                                    class="btn-primary inline-flex items-center gap-2"
                                >
                                    <i data-lucide="folder-plus" class="icon-sm"></i>
                                    "Create Project"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        // Responsive layout: Cards on mobile, Table on desktop
                        view! {
                            <div>
                                // Mobile Card Layout (visible on small screens only)
                                <div class="lg:hidden space-y-3">
                                    {project_list.clone().into_iter().map(|project| {
                                        let slug = project.slug.clone();
                                        let href = format!("/projects/{}", slug);
                                        let human_key = project.human_key.clone().unwrap_or_default();
                                        let created = project.created_at.clone().unwrap_or_default();
                                        view! {
                                            <a
                                                href=href
                                                class="card-elevated block p-4 group hover:border-amber-300 dark:hover:border-amber-700 transition-all"
                                            >
                                                <div class="flex items-start justify-between gap-3">
                                                    <div class="flex items-start gap-3 min-w-0 flex-1">
                                                        <div class="flex-shrink-0 w-9 h-9 rounded-lg bg-amber-100 dark:bg-amber-900/50 flex items-center justify-center">
                                                            <i data-lucide="folder" class="icon-base text-amber-600 dark:text-amber-400"></i>
                                                        </div>
                                                        <div class="min-w-0 flex-1">
                                                            <h3 class="font-medium text-charcoal-800 dark:text-cream-100 truncate text-sm">
                                                                {slug}
                                                            </h3>
                                                            <p class="text-xs text-charcoal-500 dark:text-charcoal-400 font-mono truncate mt-0.5">
                                                                {human_key}
                                                            </p>
                                                            <p class="text-xs text-charcoal-400 dark:text-charcoal-500 mt-1">
                                                                {format_date(&created)}
                                                            </p>
                                                        </div>
                                                    </div>
                                                    <i data-lucide="chevron-right" class="icon-sm flex-shrink-0 text-charcoal-300 dark:text-charcoal-600 group-hover:text-amber-500 self-center"></i>
                                                </div>
                                            </a>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>

                                // Desktop Table Layout (visible on lg+ screens)
                                <div class="hidden lg:block card-elevated overflow-hidden">
                                    <table class="w-full">
                                        <thead class="bg-cream-50 dark:bg-charcoal-800 border-b border-cream-200 dark:border-charcoal-700">
                                            <tr>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-charcoal-500 dark:text-charcoal-400 uppercase tracking-wider">
                                                    "Slug"
                                                </th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-charcoal-500 dark:text-charcoal-400 uppercase tracking-wider">
                                                    "Path"
                                                </th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-charcoal-500 dark:text-charcoal-400 uppercase tracking-wider">
                                                    "Created"
                                                </th>
                                                <th class="px-6 py-3 text-right text-xs font-medium text-charcoal-500 dark:text-charcoal-400 uppercase tracking-wider">
                                                    "Actions"
                                                </th>
                                            </tr>
                                        </thead>
                                        <tbody class="divide-y divide-cream-200 dark:divide-charcoal-700">
                                            {project_list.into_iter().map(|project| {
                                                let slug = project.slug.clone();
                                                let href = format!("/projects/{}", slug);
                                                let href2 = href.clone();
                                                let human_key = project.human_key.clone().unwrap_or_default();
                                                let created = project.created_at.clone().unwrap_or_default();
                                                view! {
                                                    <tr class="hover:bg-cream-50 dark:hover:bg-charcoal-800/50 transition-colors group">
                                                        <td class="px-6 py-4">
                                                            <a
                                                                href=href
                                                                class="flex items-center gap-2 text-amber-600 dark:text-amber-400 font-medium hover:text-amber-700 dark:hover:text-amber-300"
                                                            >
                                                                <i data-lucide="folder" class="icon-sm flex-shrink-0"></i>
                                                                <span class="truncate max-w-xs">{slug}</span>
                                                            </a>
                                                        </td>
                                                        <td class="px-6 py-4">
                                                            <span class="text-charcoal-600 dark:text-charcoal-400 text-sm font-mono truncate block max-w-sm">
                                                                {human_key}
                                                            </span>
                                                        </td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-charcoal-500 dark:text-charcoal-400 font-mono">
                                                            {format_date(&created)}
                                                        </td>
                                                        <td class="px-6 py-4 whitespace-nowrap text-right">
                                                            <a
                                                                href=href2
                                                                class="inline-flex items-center gap-1.5 text-amber-600 dark:text-amber-400 hover:text-amber-700 dark:hover:text-amber-300 text-sm font-medium group/link"
                                                            >
                                                                "View Agents"
                                                                <i data-lucide="arrow-right" class="icon-xs group-hover/link:translate-x-1 transition-transform"></i>
                                                            </a>
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}

/// Format date string for display.
fn format_date(date_str: &str) -> String {
    // Simple date formatting - just show the date part
    if date_str.is_empty() {
        return "—".to_string();
    }
    // Try to extract just the date part (YYYY-MM-DD)
    date_str.split('T').next().unwrap_or(date_str).to_string()
}
