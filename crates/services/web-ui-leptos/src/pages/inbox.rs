//! Inbox page - view messages with cascading project/agent selects.
//! Digital Correspondence design - envelope-style message cards.

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use crate::api::client::{self, Project, Agent, InboxMessage};

/// Inbox page component.
#[component]
pub fn Inbox() -> impl IntoView {
    let query = use_query_map();
    
    // State
    let projects = RwSignal::new(Vec::<Project>::new());
    let agents = RwSignal::new(Vec::<Agent>::new());
    let messages = RwSignal::new(Vec::<InboxMessage>::new());
    let loading = RwSignal::new(true);
    let loading_messages = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    
    // Selections
    let selected_project = RwSignal::new(String::new());
    let selected_agent = RwSignal::new(String::new());

    // Initialize from URL params
    // Initialize from URL params
    let init_project = query.with_untracked(|params| params.get("project").unwrap_or_default());
    let init_agent = query.with_untracked(|params| params.get("agent").unwrap_or_default());

    // Load projects and initialize
    Effect::new(move |_| {
        let url_project = init_project.clone();
        let url_agent = init_agent.clone();
        
        leptos::task::spawn_local(async move {
            match client::get_projects().await {
                Ok(p) => {
                    projects.set(p);
                    
                    // Set from URL params if provided
                    if !url_project.is_empty() {
                        selected_project.set(url_project.clone());
                        // Load agents for this project
                        if let Ok(a) = client::get_agents(&url_project).await {
                            agents.set(a);
                            if !url_agent.is_empty() {
                                selected_agent.set(url_agent.clone());
                                // Load messages
                                loading_messages.set(true);
                                if let Ok(m) = client::get_inbox(&url_project, &url_agent).await {
                                    messages.set(m);
                                }
                                loading_messages.set(false);
                            }
                        }
                    }
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.message));
                    loading.set(false);
                }
            }
        });
    });

    // Handle project change
    let on_project_change = move |ev: web_sys::Event| {
        let value = event_target_value(&ev);
        selected_project.set(value.clone());
        selected_agent.set(String::new());
        messages.set(Vec::new());
        
        if value.is_empty() {
            agents.set(Vec::new());
        } else {
            leptos::task::spawn_local(async move {
                match client::get_agents(&value).await {
                    Ok(a) => agents.set(a),
                    Err(e) => error.set(Some(e.message)),
                }
            });
        }
    };

    // Handle agent change
    let on_agent_change = move |ev: web_sys::Event| {
        let value = event_target_value(&ev);
        selected_agent.set(value.clone());
        
        if value.is_empty() {
            messages.set(Vec::new());
        } else {
            let project = selected_project.get();
            loading_messages.set(true);
            error.set(None);
            
            leptos::task::spawn_local(async move {
                match client::get_inbox(&project, &value).await {
                    Ok(m) => {
                        messages.set(m);
                        loading_messages.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e.message));
                        loading_messages.set(false);
                    }
                }
            });
        }
    };

    // Refresh messages
    let refresh = move || {
        let project = selected_project.get();
        let agent = selected_agent.get();
        if project.is_empty() || agent.is_empty() {
            return;
        }
        
        loading_messages.set(true);
        leptos::task::spawn_local(async move {
            match client::get_inbox(&project, &agent).await {
                Ok(m) => {
                    messages.set(m);
                    loading_messages.set(false);
                }
                Err(e) => {
                    error.set(Some(e.message));
                    loading_messages.set(false);
                }
            }
        });
    };

    // Compose state
    let show_compose = RwSignal::new(false);

    view! {
        <div class="space-y-6">
            // Compose Modal
            {move || {
                if show_compose.get() {
                    let project = selected_project.get();
                    let agent = selected_agent.get();
                    let agent_list = agents.get();
                    
                    if !project.is_empty() && !agent.is_empty() {
                         Some(view! {
                            <div class="fixed inset-0 z-50 flex items-center justify-center p-4 sm:p-6">
                                <div 
                                    class="fixed inset-0 bg-charcoal-900/50 backdrop-blur-sm transition-opacity" 
                                    on:click=move |_| show_compose.set(false)
                                ></div>
                                <div class="relative w-full max-w-2xl bg-white dark:bg-charcoal-800 rounded-2xl shadow-xl overflow-hidden animate-scale-in">
                                    <crate::components::ComposeMessage
                                        props=crate::components::ComposeProps {
                                            project_slug: project,
                                            sender_name: agent,
                                            agents: agent_list,
                                            reply_to: None,
                                        }
                                        on_close=Callback::new(move |_| show_compose.set(false))
                                        on_sent=Callback::new(move |_| {
                                            show_compose.set(false);
                                            refresh();
                                        })
                                    />
                                </div>
                            </div>
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }}

            // Header with gradient accent
            <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
                <div>
                    <h1 class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100 flex items-center gap-2">
                        <i data-lucide="inbox" class="icon-xl text-amber-500"></i>
                        "Inbox"
                    </h1>
                    <p class="text-charcoal-500 dark:text-charcoal-400">"View messages for your agents"</p>
                </div>
                {move || {
                    let project = selected_project.get();
                    let agent = selected_agent.get();
                    if !project.is_empty() && !agent.is_empty() {
                        Some(view! {
                            <button 
                                on:click=move |_| show_compose.set(true)
                                class="btn-primary flex items-center gap-2"
                            >
                                <i data-lucide="square-pen" class="icon-sm"></i>
                                <span>"Compose"</span>
                            </button>
                        })
                    } else {
                        None
                    }
                }}
            </div>

            // Filters Card
            <div class="card-elevated p-5">
                <div class="flex flex-col md:flex-row gap-4">
                    // Project Selector
                    <div class="flex-1">
                        <label for="projectSelect" class="flex items-center gap-2 text-sm font-medium text-charcoal-700 dark:text-charcoal-300 mb-2">
                            <i data-lucide="folder" class="icon-sm text-charcoal-400"></i>
                            "Project"
                        </label>
                        <select
                            id="projectSelect"
                            on:change=on_project_change
                            class="input"
                        >
                            <option value="">"Select a project..."</option>
                            {move || {
                                projects.get().into_iter().map(|p| {
                                    let slug = p.slug.clone();
                                    let slug_display = slug.clone();
                                    let selected = selected_project.get() == slug;
                                    view! {
                                        <option value=slug selected=selected>{slug_display}</option>
                                    }
                                }).collect::<Vec<_>>()
                            }}
                        </select>
                    </div>

                    // Agent Selector
                    <div class="flex-1">
                        <label for="agentSelect" class="flex items-center gap-2 text-sm font-medium text-charcoal-700 dark:text-charcoal-300 mb-2">
                            <i data-lucide="bot" class="icon-sm text-charcoal-400"></i>
                            "Agent"
                        </label>
                        <select
                            id="agentSelect"
                            on:change=on_agent_change
                            disabled=move || selected_project.get().is_empty() || agents.get().is_empty()
                            class="input disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <option value="">"Select an agent..."</option>
                            {move || {
                                agents.get().into_iter().map(|a| {
                                    let name = a.name.clone();
                                    let name_display = name.clone();
                                    let selected = selected_agent.get() == name;
                                    view! {
                                        <option value=name selected=selected>{name_display}</option>
                                    }
                                }).collect::<Vec<_>>()
                            }}
                        </select>
                    </div>

                    // Refresh Button
                    {move || {
                        let project = selected_project.get();
                        let agent = selected_agent.get();
                        if !project.is_empty() && !agent.is_empty() {
                            Some(view! {
                                <div class="flex items-end">
                                    <button
                                        on:click=move |_| refresh()
                                        disabled=move || loading_messages.get()
                                        class="btn-secondary flex items-center gap-2 disabled:opacity-50"
                                    >
                                        {move || if loading_messages.get() {
                                            view! { <i data-lucide="loader-2" class="icon-sm animate-spin"></i> }
                                        } else {
                                            view! { <i data-lucide="refresh-cw" class="icon-sm"></i> }
                                        }}
                                        <span>"Refresh"</span>
                                    </button>
                                </div>
                            })
                        } else {
                            None
                        }
                    }}
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
                                <p class="text-charcoal-500 dark:text-charcoal-400 text-sm">"Loading..."</p>
                            </div>
                        </div>
                    }.into_any()
                } else if selected_project.get().is_empty() || selected_agent.get().is_empty() {
                    // Selection Prompt
                    view! {
                        <div class="card-elevated p-12 text-center">
                            <div class="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-amber-100 dark:bg-amber-900/50 mb-6">
                                <i data-lucide="inbox" class="icon-2xl text-amber-600 dark:text-amber-400"></i>
                            </div>
                            <h3 class="font-display text-xl font-semibold text-charcoal-800 dark:text-cream-100 mb-2">"Select an Agent"</h3>
                            <p class="text-charcoal-500 dark:text-charcoal-400 max-w-sm mx-auto">
                                "Choose a project and agent from the dropdowns above to view their inbox."
                            </p>
                        </div>
                    }.into_any()
                } else if loading_messages.get() {
                    view! {
                        <div class="flex items-center justify-center py-16">
                            <div class="flex flex-col items-center gap-4">
                                <i data-lucide="loader-2" class="icon-2xl text-amber-500 animate-spin"></i>
                                <p class="text-charcoal-500 dark:text-charcoal-400 text-sm">"Fetching messages..."</p>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    let msg_list = messages.get();
                    if msg_list.is_empty() {
                        // Empty Inbox
                        let agent = selected_agent.get();
                        view! {
                            <div class="card-elevated p-12 text-center">
                                <div class="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-cream-200 dark:bg-charcoal-700 mb-6">
                                    <i data-lucide="mail-open" class="icon-2xl text-charcoal-400"></i>
                                </div>
                                <h3 class="font-display text-xl font-semibold text-charcoal-800 dark:text-cream-100 mb-2">"Inbox is empty"</h3>
                                <p class="text-charcoal-500 dark:text-charcoal-400 mb-6">
                                    "No messages for " <span class="font-medium text-charcoal-700 dark:text-cream-200">{agent}</span> " yet."
                                </p>
                                <button
                                    on:click=move |_| show_compose.set(true)
                                    class="btn-primary flex items-center gap-2 mx-auto"
                                >
                                    <i data-lucide="send" class="icon-sm"></i>
                                    "Send a Message"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        // Messages List
                        let project = selected_project.get();
                        let agent = selected_agent.get();
                        let msg_count = msg_list.len();
                        view! {
                            <div class="card-elevated overflow-hidden">
                                // Header
                                <div class="px-6 py-4 border-b border-cream-200 dark:border-charcoal-700 bg-cream-50/50 dark:bg-charcoal-800/50">
                                    <div class="flex items-center justify-between">
                                        <span class="flex items-center gap-2 text-sm font-medium text-charcoal-600 dark:text-charcoal-400">
                                            <i data-lucide="mails" class="icon-sm"></i>
                                            {msg_count} " message" {if msg_count == 1 { "" } else { "s" }}
                                        </span>
                                        <span class="badge badge-teal flex items-center gap-1.5">
                                            <i data-lucide="bot" class="icon-xs"></i>
                                            {agent.clone()}
                                        </span>
                                    </div>
                                </div>
                                
                                // Message List
                                <ul class="divide-y divide-cream-200 dark:divide-charcoal-700">
                                    {msg_list.into_iter().map(|msg| {
                                        let id = msg.id;
                                        let href = format!("/inbox/{}?project={}&agent={}", id, project, agent);
                                        let subject = msg.subject.clone();
                                        let sender = msg.sender_name.clone();
                                        let created = msg.created_ts.clone();
                                        
                                        view! {
                                            <li class="group">
                                                <a
                                                    href=href
                                                    class="flex items-start gap-4 px-6 py-4 hover:bg-cream-50 dark:hover:bg-charcoal-800/50 transition-colors"
                                                >
                                                    // Envelope icon
                                                    <div class="flex-shrink-0 w-10 h-10 rounded-lg bg-amber-100 dark:bg-amber-900/50 flex items-center justify-center group-hover:scale-105 transition-transform">
                                                        <i data-lucide="mail" class="icon-lg text-amber-600 dark:text-amber-400"></i>
                                                    </div>
                                                    
                                                    // Content
                                                    <div class="flex-1 min-w-0">
                                                        <div class="flex items-baseline justify-between gap-4 mb-1">
                                                            <h4 class="font-medium text-charcoal-800 dark:text-cream-100 truncate group-hover:text-amber-600 dark:group-hover:text-amber-400 transition-colors">
                                                                {subject}
                                                            </h4>
                                                            <span class="flex-shrink-0 text-xs font-mono text-charcoal-400 dark:text-charcoal-500">
                                                                {format_date(&created)}
                                                            </span>
                                                        </div>
                                                        <p class="text-sm text-charcoal-500 dark:text-charcoal-400 flex items-center gap-1.5">
                                                            <i data-lucide="user" class="icon-xs"></i>
                                                            <span>{sender}</span>
                                                        </p>
                                                    </div>
                                                    
                                                    // Arrow
                                                    <i data-lucide="chevron-right" class="icon-sm flex-shrink-0 text-charcoal-300 dark:text-charcoal-600 group-hover:text-amber-500 group-hover:translate-x-1 transition-all"></i>
                                                </a>
                                            </li>
                                        }
                                    }).collect::<Vec<_>>()}
                                </ul>
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
    // Return just the date part for cleaner display
    date_str.split('T').next().unwrap_or(date_str).to_string()
}
