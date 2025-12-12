//! Project detail page - view project info and manage agents.
//! Digital Correspondence design with Lucide icons.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use crate::api::client::{self, Agent};

/// Project detail page component.
#[component]
pub fn ProjectDetail() -> impl IntoView {
    let params = use_params_map();
    let slug = move || params.read().get("slug").unwrap_or_default();

    // State
    let agents = RwSignal::new(Vec::<Agent>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);
    let show_new_form = RwSignal::new(false);
    let creating = RwSignal::new(false);
    
    // Form fields
    let new_name = RwSignal::new(String::new());
    let new_program = RwSignal::new(String::new());
    let new_model = RwSignal::new(String::new());
    let new_task = RwSignal::new(String::new());

    // Load agents
    let load_agents = {
        let slug = slug.clone();
        move || {
            let project_slug = slug();
            loading.set(true);
            error.set(None);
            leptos::task::spawn_local(async move {
                match client::get_agents(&project_slug).await {
                    Ok(a) => {
                        agents.set(a);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e.message));
                        loading.set(false);
                    }
                }
            });
        }
    };

    // Initial load
    Effect::new({
        let load = load_agents.clone();
        move |_| { load(); }
    });

    // Create agent handler
    let create_agent = {
        let slug = slug.clone();
        let _load_agents = load_agents.clone();
        move |_| {
            let name = new_name.get();
            if name.trim().is_empty() {
                return;
            }

            let project_slug = slug();
            let program = new_program.get();
            let model = new_model.get();
            let task = new_task.get();

            creating.set(true);
            error.set(None);

            leptos::task::spawn_local(async move {
                match client::register_agent(
                    &project_slug,
                    &name,
                    if program.is_empty() { "unknown" } else { &program },
                    if model.is_empty() { "unknown" } else { &model },
                    if task.is_empty() { None } else { Some(task.as_str()) },
                ).await {
                    Ok(_) => {
                        // Reload agents
                        match client::get_agents(&project_slug).await {
                            Ok(a) => agents.set(a),
                            Err(e) => error.set(Some(e.message)),
                        }
                        new_name.set(String::new());
                        new_program.set(String::new());
                        new_model.set(String::new());
                        new_task.set(String::new());
                        show_new_form.set(false);
                        creating.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e.message));
                        creating.set(false);
                    }
                }
            });
        }
    };

    view! {
        <div class="space-y-6">
            // Breadcrumb
            <nav class="flex items-center gap-2 text-sm text-charcoal-500 dark:text-charcoal-400">
                <a href="/projects" class="flex items-center gap-1 hover:text-amber-600 dark:hover:text-amber-400 transition-colors">
                    <i data-lucide="folder-open" class="icon-sm"></i>
                    "Projects"
                </a>
                <i data-lucide="chevron-right" class="icon-xs text-charcoal-400"></i>
                <span class="text-charcoal-800 dark:text-cream-100 font-medium">{slug}</span>
            </nav>

            // Header
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100 flex items-center gap-2">
                        <i data-lucide="folder" class="icon-xl text-amber-500"></i>
                        {slug}
                    </h1>
                    <p class="text-charcoal-500 dark:text-charcoal-400">"Agents in this project"</p>
                </div>
                <button
                    on:click=move |_| show_new_form.update(|v| *v = !*v)
                    class="btn-primary flex items-center gap-2"
                >
                    <i data-lucide="user-plus" class="icon-sm"></i>
                    <span>"Register Agent"</span>
                </button>
            </div>

            // New Agent Form
            {move || {
                if show_new_form.get() {
                    Some(view! {
                        <div class="card-elevated p-6 animate-slide-up">
                            <h2 class="font-display text-lg font-semibold text-charcoal-800 dark:text-cream-100 mb-4 flex items-center gap-2">
                                <i data-lucide="bot" class="icon-lg text-violet-500"></i>
                                "Register New Agent"
                            </h2>
                            <form on:submit=move |ev| { ev.prevent_default(); create_agent(()); } class="space-y-4">
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                    <div>
                                        <label for="agentName" class="block text-sm font-medium text-charcoal-700 dark:text-charcoal-300 mb-2">
                                            "Agent Name *"
                                        </label>
                                        <input
                                            id="agentName"
                                            type="text"
                                            prop:value=move || new_name.get()
                                            on:input=move |ev| new_name.set(event_target_value(&ev))
                                            placeholder="BlueStone"
                                            class="input"
                                        />
                                    </div>
                                    <div>
                                        <label for="agentProgram" class="block text-sm font-medium text-charcoal-700 dark:text-charcoal-300 mb-2">
                                            "Program"
                                        </label>
                                        <input
                                            id="agentProgram"
                                            type="text"
                                            prop:value=move || new_program.get()
                                            on:input=move |ev| new_program.set(event_target_value(&ev))
                                            placeholder="antigravity"
                                            class="input"
                                        />
                                    </div>
                                    <div>
                                        <label for="agentModel" class="block text-sm font-medium text-charcoal-700 dark:text-charcoal-300 mb-2">
                                            "Model"
                                        </label>
                                        <input
                                            id="agentModel"
                                            type="text"
                                            prop:value=move || new_model.get()
                                            on:input=move |ev| new_model.set(event_target_value(&ev))
                                            placeholder="gemini-2.0-pro"
                                            class="input"
                                        />
                                    </div>
                                    <div>
                                        <label for="agentTask" class="block text-sm font-medium text-charcoal-700 dark:text-charcoal-300 mb-2">
                                            "Task Description"
                                        </label>
                                        <input
                                            id="agentTask"
                                            type="text"
                                            prop:value=move || new_task.get()
                                            on:input=move |ev| new_task.set(event_target_value(&ev))
                                            placeholder="Research and implement features"
                                            class="input"
                                        />
                                    </div>
                                </div>
                                <div class="flex gap-3">
                                    <button
                                        type="submit"
                                        disabled=move || creating.get() || new_name.get().trim().is_empty()
                                        class="btn-primary flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
                                    >
                                        {move || if creating.get() {
                                            view! { <i data-lucide="loader-2" class="icon-sm animate-spin"></i> }
                                        } else {
                                            view! { <i data-lucide="plus" class="icon-sm"></i> }
                                        }}
                                        {move || if creating.get() { "Registering..." } else { "Register Agent" }}
                                    </button>
                                    <button
                                        type="button"
                                        on:click=move |_| {
                                            show_new_form.set(false);
                                            new_name.set(String::new());
                                            new_program.set(String::new());
                                            new_model.set(String::new());
                                            new_task.set(String::new());
                                        }
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

            // Content: Loading / Empty / Grid
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
                    let agent_list = agents.get();
                    if agent_list.is_empty() {
                        view! {
                            <div class="card-elevated p-12 text-center">
                                <div class="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-violet-100 dark:bg-violet-900/50 mb-6">
                                    <i data-lucide="bot" class="icon-2xl text-violet-600 dark:text-violet-400"></i>
                                </div>
                                <h3 class="font-display text-xl font-semibold text-charcoal-800 dark:text-cream-100 mb-2">"No agents yet"</h3>
                                <p class="text-charcoal-500 dark:text-charcoal-400 mb-6">
                                    "Register your first agent to start sending and receiving messages."
                                </p>
                                <button
                                    on:click=move |_| show_new_form.set(true)
                                    class="btn-primary inline-flex items-center gap-2"
                                >
                                    <i data-lucide="user-plus" class="icon-sm"></i>
                                    "Register Agent"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        let project_slug = slug();
                        view! {
                            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                {agent_list.into_iter().map(|agent| {
                                    let name = agent.name.clone();
                                    let program = agent.program.clone().unwrap_or_else(|| "unknown".to_string());
                                    let model = agent.model.clone().unwrap_or_else(|| "unknown".to_string());
                                    let task = agent.task_description.clone();
                                    let last_active = agent.last_active_ts.clone().unwrap_or_default();
                                    let inbox_href = format!("/inbox?project={}&agent={}", project_slug, name);
                                    
                                    view! {
                                        <div class="card-elevated p-6 group hover:border-violet-300 dark:hover:border-violet-700 transition-all">
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
                                                    href=inbox_href
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
