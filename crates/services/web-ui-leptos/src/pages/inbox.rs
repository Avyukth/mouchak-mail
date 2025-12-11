//! Inbox page - view messages with cascading project/agent selects.

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use crate::api::client::{self, Project, Agent, Message};

/// Inbox page component.
#[component]
pub fn Inbox() -> impl IntoView {
    let query = use_query_map();
    
    // State
    let projects = RwSignal::new(Vec::<Project>::new());
    let agents = RwSignal::new(Vec::<Agent>::new());
    let messages = RwSignal::new(Vec::<Message>::new());
    let loading = RwSignal::new(true);
    let loading_messages = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    
    // Selections
    let selected_project = RwSignal::new(String::new());
    let selected_agent = RwSignal::new(String::new());

    // Initialize from URL params
    let init_project = query.read().get("project").unwrap_or_default();
    let init_agent = query.read().get("agent").unwrap_or_default();

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
    let refresh = move |_| {
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

    view! {
        <div class="space-y-6">
            // Header
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="text-2xl font-bold text-gray-900 dark:text-white">"Inbox"</h1>
                    <p class="text-gray-600 dark:text-gray-400">"View messages for your agents"</p>
                </div>
                {move || {
                    let project = selected_project.get();
                    let agent = selected_agent.get();
                    if !project.is_empty() && !agent.is_empty() {
                        Some(view! {
                            <button
                                class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors flex items-center gap-2"
                            >
                                <span class="text-lg">"✉️"</span>
                                <span>"Compose"</span>
                            </button>
                        })
                    } else {
                        None
                    }
                }}
            </div>

            // Filters
            <div class="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm border border-gray-200 dark:border-gray-700">
                <div class="flex flex-col md:flex-row gap-4">
                    // Project Selector
                    <div class="flex-1">
                        <label for="projectSelect" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Project"
                        </label>
                        <select
                            id="projectSelect"
                            on:change=on_project_change
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
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
                        <label for="agentSelect" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Agent"
                        </label>
                        <select
                            id="agentSelect"
                            on:change=on_agent_change
                            disabled=move || selected_project.get().is_empty() || agents.get().is_empty()
                            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed"
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
                                        on:click=refresh
                                        disabled=move || loading_messages.get()
                                        class="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors disabled:opacity-50"
                                    >
                                        "🔄 Refresh"
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
                    <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl p-4">
                        <p class="text-red-700 dark:text-red-400">{e}</p>
                    </div>
                })
            }}

            // Content
            {move || {
                if loading.get() {
                    view! {
                        <div class="flex items-center justify-center py-12">
                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
                        </div>
                    }.into_any()
                } else if selected_project.get().is_empty() || selected_agent.get().is_empty() {
                    // Selection Prompt
                    view! {
                        <div class="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
                            <div class="text-4xl mb-4">"📬"</div>
                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">"Select an Agent"</h3>
                            <p class="text-gray-600 dark:text-gray-400">
                                "Choose a project and agent to view their inbox."
                            </p>
                        </div>
                    }.into_any()
                } else if loading_messages.get() {
                    view! {
                        <div class="flex items-center justify-center py-12">
                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
                        </div>
                    }.into_any()
                } else {
                    let msg_list = messages.get();
                    if msg_list.is_empty() {
                        // Empty Inbox
                        let agent = selected_agent.get();
                        view! {
                            <div class="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
                                <div class="text-4xl mb-4">"📭"</div>
                                <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">"Inbox is empty"</h3>
                                <p class="text-gray-600 dark:text-gray-400 mb-4">
                                    "No messages for " {agent} " yet."
                                </p>
                                <button
                                    class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
                                >
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
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                                <div class="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                                    <span class="text-sm text-gray-600 dark:text-gray-400">
                                        {msg_count} " message" {if msg_count == 1 { "" } else { "s" }}
                                    </span>
                                </div>
                                <ul class="divide-y divide-gray-200 dark:divide-gray-700">
                                    {msg_list.into_iter().map(|msg| {
                                        let id = msg.id.clone();
                                        let href = format!("/inbox/{}?project={}&agent={}", id, project, agent);
                                        let subject = msg.subject.clone().unwrap_or_else(|| "(No subject)".to_string());
                                        let body = msg.body.clone();
                                        let body_preview = truncate_body(&body, 100);
                                        let created = msg.created_at.clone().unwrap_or_default();
                                        let importance = msg.importance.clone();
                                        let ack_required = msg.ack_required.unwrap_or(false);
                                        let has_thread = msg.thread_id.is_some();
                                        
                                        view! {
                                            <li>
                                                <a
                                                    href=href
                                                    class="block p-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
                                                >
                                                    <div class="flex items-start justify-between gap-4">
                                                        <div class="flex-1 min-w-0">
                                                            <div class="flex items-center gap-2 mb-1">
                                                                <h4 class="font-medium text-gray-900 dark:text-white truncate">
                                                                    {subject}
                                                                </h4>
                                                                {importance.as_ref().filter(|i| *i != "normal").map(|i| {
                                                                    let badge_class = get_importance_badge(i);
                                                                    view! {
                                                                        <span class={format!("px-2 py-0.5 text-xs rounded-full {}", badge_class)}>
                                                                            {i.clone()}
                                                                        </span>
                                                                    }
                                                                })}
                                                                {if ack_required {
                                                                    Some(view! {
                                                                        <span class="px-2 py-0.5 text-xs rounded-full bg-amber-100 dark:bg-amber-900 text-amber-700 dark:text-amber-300">
                                                                            "ACK"
                                                                        </span>
                                                                    })
                                                                } else {
                                                                    None
                                                                }}
                                                                {if has_thread {
                                                                    Some(view! {
                                                                        <span class="text-xs text-gray-400" title="Part of a thread">"🧵"</span>
                                                                    })
                                                                } else {
                                                                    None
                                                                }}
                                                            </div>
                                                            <p class="text-sm text-gray-600 dark:text-gray-400 truncate">
                                                                {body_preview}
                                                            </p>
                                                        </div>
                                                        <div class="text-sm text-gray-500 dark:text-gray-400 whitespace-nowrap">
                                                            {format_date(&created)}
                                                        </div>
                                                    </div>
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

fn truncate_body(body: &str, max_len: usize) -> String {
    if body.len() <= max_len {
        body.to_string()
    } else {
        format!("{}...", &body[..max_len])
    }
}

fn format_date(date_str: &str) -> String {
    if date_str.is_empty() {
        return "—".to_string();
    }
    // Simple: just return the date part
    date_str.split('T').next().unwrap_or(date_str).to_string()
}

fn get_importance_badge(importance: &str) -> &'static str {
    match importance {
        "high" => "bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300",
        "low" => "bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400",
        _ => "bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300",
    }
}
