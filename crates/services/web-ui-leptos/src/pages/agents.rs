//! Agents page - browse agents across all projects.
//! Digital Correspondence design with Lucide icons.

use leptos::prelude::*;
use crate::api::client::{self, Project, Agent};

/// Agent with project slug for display.
#[derive(Clone)]
struct AgentWithProject {
    agent: Agent,
    project_slug: String,
}

/// Agents page component.
#[component]
pub fn Agents() -> impl IntoView {
    // State
    let all_agents = RwSignal::new(Vec::<AgentWithProject>::new());
    let projects = RwSignal::new(Vec::<Project>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);
    
    // Filters
    let selected_project = RwSignal::new("all".to_string());
    let search_query = RwSignal::new(String::new());

    // Load all agents across projects
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match client::get_projects().await {
                Ok(p) => {
                    projects.set(p.clone());
                    
                    // Load agents for each project
                    let mut agents_list = Vec::new();
                    for project in p {
                        if let Ok(agents) = client::get_agents(&project.slug).await {
                            for agent in agents {
                                agents_list.push(AgentWithProject {
                                    agent,
                                    project_slug: project.slug.clone(),
                                });
                            }
                        }
                    }
                    all_agents.set(agents_list);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.message));
                    loading.set(false);
                }
            }
        });
    });

    // Filter agents
    let filtered_agents = move || {
        let agents = all_agents.get();
        let project_filter = selected_project.get();
        let query = search_query.get().to_lowercase();
        
        agents.into_iter()
            .filter(|a| {
                if project_filter != "all" && a.project_slug != project_filter {
                    return false;
                }
                if !query.is_empty() {
                    let name_match = a.agent.name.to_lowercase().contains(&query);
                    let program_match = a.agent.program.as_ref()
                        .map_or(false, |p| p.to_lowercase().contains(&query));
                    let model_match = a.agent.model.as_ref()
                        .map_or(false, |m| m.to_lowercase().contains(&query));
                    let task_match = a.agent.task_description.as_ref()
                        .map_or(false, |t| t.to_lowercase().contains(&query));
                    if !name_match && !program_match && !model_match && !task_match {
                        return false;
                    }
                }
                true
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="space-y-6">
            // Header
            <div>
                <h1 class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100 flex items-center gap-2">
                    <i data-lucide="bot" class="icon-xl text-amber-500"></i>
                    "All Agents"
                </h1>
                <p class="text-charcoal-500 dark:text-charcoal-400">"Browse agents across all projects"</p>
            </div>

            // Filters
            <div class="card-elevated p-5">
                <div class="flex flex-col md:flex-row gap-4">
                    // Search
                    <div class="flex-1">
                        <div class="relative">
                            <i data-lucide="search" class="icon-sm absolute left-3 top-1/2 -translate-y-1/2 text-charcoal-400"></i>
                            <input
                                type="text"
                                prop:value=move || search_query.get()
                                on:input=move |ev| search_query.set(event_target_value(&ev))
                                placeholder="Search by name, program, model, or task..."
                                class="input pl-10"
                            />
                        </div>
                    </div>

                    // Project Filter
                    <div class="md:w-64">
                        <select
                            on:change=move |ev| selected_project.set(event_target_value(&ev))
                            class="input"
                        >
                            <option value="all">"All Projects"</option>
                            {move || {
                                projects.get().into_iter().map(|p| {
                                    let slug = p.slug.clone();
                                    let slug_display = slug.clone();
                                    view! {
                                        <option value=slug>{slug_display}</option>
                                    }
                                }).collect::<Vec<_>>()
                            }}
                        </select>
                    </div>
                </div>
            </div>

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

            // Content
            {move || {
                if loading.get() {
                    view! {
                        <div class="flex items-center justify-center py-16">
                            <div class="flex flex-col items-center gap-4">
                                <i data-lucide="loader-2" class="icon-2xl text-amber-500 animate-spin"></i>
                                <p class="text-charcoal-500 dark:text-charcoal-400 text-sm">"Loading agents..."</p>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    let filtered = filtered_agents();
                    let total = all_agents.get().len();
                    let count = filtered.len();
                    
                    if filtered.is_empty() {
                        view! {
                            <div class="card-elevated p-12 text-center">
                                <div class="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-amber-100 dark:bg-amber-900/50 mb-6">
                                    <i data-lucide="bot" class="icon-2xl text-amber-600 dark:text-amber-400"></i>
                                </div>
                                {if total == 0 {
                                    view! {
                                        <h3 class="font-display text-xl font-semibold text-charcoal-800 dark:text-cream-100 mb-2">"No agents yet"</h3>
                                        <p class="text-charcoal-500 dark:text-charcoal-400 mb-6">
                                            "Create a project and register agents to get started."
                                        </p>
                                        <a href="/projects" class="btn-primary inline-flex items-center gap-2">
                                            <i data-lucide="folder-plus" class="icon-sm"></i>
                                            "Go to Projects"
                                        </a>
                                    }.into_any()
                                } else {
                                    view! {
                                        <h3 class="font-display text-xl font-semibold text-charcoal-800 dark:text-cream-100 mb-2">"No matching agents"</h3>
                                        <p class="text-charcoal-500 dark:text-charcoal-400">
                                            "Try adjusting your search or filter criteria."
                                        </p>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="space-y-4">
                                // Stats
                                <div class="flex items-center gap-4 text-sm text-charcoal-600 dark:text-charcoal-400">
                                    <span class="flex items-center gap-1.5">
                                        <i data-lucide="users" class="icon-sm"></i>
                                        "Showing " {count} " of " {total} " agents"
                                    </span>
                                    {move || {
                                        let project = selected_project.get();
                                        if project != "all" {
                                            Some(view! {
                                                <span class="badge badge-amber flex items-center gap-1">
                                                    <i data-lucide="folder" class="icon-xs"></i>
                                                    {project}
                                                </span>
                                            })
                                        } else {
                                            None
                                        }
                                    }}
                                </div>

                                // Agents Grid
                                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                    {filtered.into_iter().map(|awp| {
                                        let agent = awp.agent;
                                        let project_slug = awp.project_slug;
                                        let name = agent.name.clone();
                                        let program = agent.program.clone().unwrap_or_default();
                                        let model = agent.model.clone().unwrap_or_default();
                                        let task = agent.task_description.clone();
                                        let last_active = agent.last_active_ts.clone().unwrap_or_default();
                                        let project_link = format!("/projects/{}", project_slug);
                                        let inbox_link = format!("/inbox?project={}&agent={}", project_slug, name);
                                        
                                        view! {
                                            <div class="card-elevated p-6 group hover:border-amber-300 dark:hover:border-amber-700 transition-all">
                                                <div class="flex items-start justify-between mb-4">
                                                    <div class="flex items-center gap-3">
                                                        <div class="w-10 h-10 bg-violet-100 dark:bg-violet-900/50 rounded-xl flex items-center justify-center group-hover:scale-105 transition-transform">
                                                            <i data-lucide="bot" class="icon-lg text-violet-600 dark:text-violet-400"></i>
                                                        </div>
                                                        <div>
                                                            <h3 class="font-display font-semibold text-charcoal-800 dark:text-cream-100">{name.clone()}</h3>
                                                            <p class="text-sm text-charcoal-500 dark:text-charcoal-400">{program}</p>
                                                        </div>
                                                    </div>
                                                </div>

                                                <div class="space-y-2 text-sm">
                                                    <div class="flex justify-between">
                                                        <span class="text-charcoal-500 dark:text-charcoal-400">"Project"</span>
                                                        <a
                                                            href=project_link
                                                            class="text-amber-600 dark:text-amber-400 hover:underline font-medium flex items-center gap-1"
                                                        >
                                                            <i data-lucide="folder" class="icon-xs"></i>
                                                            {project_slug.clone()}
                                                        </a>
                                                    </div>
                                                    <div class="flex justify-between">
                                                        <span class="text-charcoal-500 dark:text-charcoal-400">"Model"</span>
                                                        <span class="text-charcoal-800 dark:text-cream-100 font-mono text-xs">{model}</span>
                                                    </div>
                                                    {task.map(|t| view! {
                                                        <div>
                                                            <span class="text-charcoal-500 dark:text-charcoal-400">"Task"</span>
                                                            <p class="text-charcoal-700 dark:text-charcoal-300 mt-1 line-clamp-2">{t}</p>
                                                        </div>
                                                    })}
                                                    <div class="flex justify-between pt-2 border-t border-cream-200 dark:border-charcoal-700">
                                                        <span class="text-charcoal-500 dark:text-charcoal-400 flex items-center gap-1">
                                                            <i data-lucide="clock" class="icon-xs"></i>
                                                            "Last Active"
                                                        </span>
                                                        <span class="text-charcoal-600 dark:text-charcoal-400 font-mono text-xs">{format_date(&last_active)}</span>
                                                    </div>
                                                </div>

                                                <div class="mt-4 pt-4 border-t border-cream-200 dark:border-charcoal-700">
                                                    <a
                                                        href=inbox_link
                                                        class="flex items-center gap-2 text-amber-600 dark:text-amber-400 hover:text-amber-700 dark:hover:text-amber-300 text-sm font-medium group/link"
                                                    >
                                                        <i data-lucide="inbox" class="icon-sm"></i>
                                                        "View Inbox"
                                                        <i data-lucide="arrow-right" class="icon-xs group-hover/link:translate-x-1 transition-transform"></i>
                                                    </a>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}

fn format_date(date_str: &str) -> String {
    if date_str.is_empty() {
        return "—".to_string();
    }
    date_str.split('T').next().unwrap_or(date_str).to_string()
}
