//! Message detail page - view a single message with reply functionality.

use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};
use crate::api::client::{self, Message, Agent};
use crate::components::{ComposeMessage, ComposeProps, ReplyTo};

/// Message detail page component.
#[component]
pub fn MessageDetail() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    
    let message_id = move || params.read().get("id").unwrap_or_default();
    let project_slug = query.read().get("project").unwrap_or_default();
    let agent_name = query.read().get("agent").unwrap_or_default();

    // State
    let message = RwSignal::new(Option::<Message>::None);
    let agents = RwSignal::new(Vec::<Agent>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);
    let show_reply = RwSignal::new(false);

    // Load message and agents
    Effect::new({
        let project_slug = project_slug.clone();
        move |_| {
            let id = message_id();
            let project = project_slug.clone();
            
            leptos::task::spawn_local(async move {
                // Load message
                match client::get_message(&id).await {
                    Ok(m) => {
                        message.set(Some(m));
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e.message));
                        loading.set(false);
                    }
                }
                
                // Load agents for reply
                if !project.is_empty() {
                    if let Ok(a) = client::get_agents(&project).await {
                        agents.set(a);
                    }
                }
            });
        }
    });

    // Back to inbox URL
    let back_url = {
        let project = project_slug.clone();
        let agent = agent_name.clone();
        if !project.is_empty() && !agent.is_empty() {
            format!("/inbox?project={}&agent={}", project, agent)
        } else {
            "/inbox".to_string()
        }
    };

    view! {
        <div class="space-y-6">
            // Breadcrumb / Back
            <nav class="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
                <a href=back_url.clone() class="hover:text-primary-600 dark:hover:text-primary-400 flex items-center gap-1">
                    <span>"←"</span>
                    <span>"Back to Inbox"</span>
                </a>
                {if !agent_name.is_empty() {
                    Some(view! {
                        <span>"/"</span>
                        <span class="text-gray-900 dark:text-white font-medium">{agent_name.clone()}</span>
                    })
                } else {
                    None
                }}
            </nav>

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
                } else if let Some(msg) = message.get() {
                    let subject = msg.subject.clone().unwrap_or_else(|| "(No subject)".to_string());
                    let body = msg.body.clone();
                    let created = msg.created_at.clone().unwrap_or_default();
                    let importance = msg.importance.clone();
                    let ack_required = msg.ack_required.unwrap_or(false);
                    let thread_id = msg.thread_id.clone();
                    let msg_id = msg.id.clone();
                    let sender = msg.sender.clone();
                    let recipient = msg.recipient.clone();
                    let can_reply = !agents.get().is_empty();

                    view! {
                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                            // Message Header
                            <div class="p-6 border-b border-gray-200 dark:border-gray-700">
                                <div class="flex items-start justify-between gap-4">
                                    <div class="flex-1">
                                        <h1 class="text-xl font-bold text-gray-900 dark:text-white mb-2">
                                            {subject.clone()}
                                        </h1>
                                        <div class="flex flex-wrap items-center gap-2 text-sm">
                                            {importance.as_ref().filter(|i| *i != "normal").map(|i| {
                                                let badge_class = get_importance_badge(i);
                                                view! {
                                                    <span class={format!("px-2 py-0.5 rounded-full {}", badge_class)}>
                                                        {i.clone()} " priority"
                                                    </span>
                                                }
                                            })}
                                            {if ack_required {
                                                Some(view! {
                                                    <span class="px-2 py-0.5 rounded-full bg-amber-100 dark:bg-amber-900 text-amber-700 dark:text-amber-300">
                                                        "Acknowledgment required"
                                                    </span>
                                                })
                                            } else {
                                                None
                                            }}
                                            {thread_id.as_ref().map(|tid| view! {
                                                <span class="px-2 py-0.5 rounded-full bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300">
                                                    "Thread: " {tid.clone()}
                                                </span>
                                            })}
                                        </div>
                                    </div>
                                    {if can_reply {
                                        Some(view! {
                                            <button
                                                on:click=move |_| show_reply.set(true)
                                                class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors flex items-center gap-2"
                                            >
                                                <span>"↩️"</span>
                                                <span>"Reply"</span>
                                            </button>
                                        })
                                    } else {
                                        None
                                    }}
                                </div>

                                <div class="mt-4 text-sm text-gray-600 dark:text-gray-400">
                                    <div class="flex items-center gap-4">
                                        <span>"From: " {sender.clone()}</span>
                                        <span>"To: " {recipient.clone()}</span>
                                        <span>"Received: " {format_date(&created)}</span>
                                    </div>
                                </div>
                            </div>

                            // Message Body
                            <div class="p-6">
                                <div class="prose dark:prose-invert max-w-none">
                                    <pre class="whitespace-pre-wrap font-sans text-gray-700 dark:text-gray-300 bg-transparent p-0 overflow-visible">
                                        {body}
                                    </pre>
                                </div>
                            </div>

                            // Message Metadata
                            <div class="p-6 bg-gray-50 dark:bg-gray-700/50 border-t border-gray-200 dark:border-gray-700">
                                <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">"Message Details"</h3>
                                <dl class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                                    <div>
                                        <dt class="text-gray-500 dark:text-gray-400">"Message ID"</dt>
                                        <dd class="font-mono text-gray-900 dark:text-white">{msg_id}</dd>
                                    </div>
                                    <div>
                                        <dt class="text-gray-500 dark:text-gray-400">"Thread ID"</dt>
                                        <dd class="font-mono text-gray-900 dark:text-white">
                                            {thread_id.unwrap_or_else(|| "None".to_string())}
                                        </dd>
                                    </div>
                                </dl>
                            </div>
                        </div>

                        // Quick Actions
                        <div class="flex items-center gap-3">
                            <a
                                href=back_url.clone()
                                class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                            >
                                "← Back to Inbox"
                            </a>
                            {if can_reply {
                                Some(view! {
                                    <button
                                        on:click=move |_| show_reply.set(true)
                                        class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
                                    >
                                        "Reply to Message"
                                    </button>
                                })
                            } else {
                                None
                            }}
                        </div>
                    }.into_any()
                } else {
                    // Not Found
                    view! {
                        <div class="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
                            <div class="text-4xl mb-4">"📭"</div>
                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">"Message not found"</h3>
                            <p class="text-gray-600 dark:text-gray-400 mb-4">
                                "The message you're looking for doesn't exist or has been deleted."
                            </p>
                            <a
                                href=back_url.clone()
                                class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors inline-block"
                            >
                                "Back to Inbox"
                            </a>
                        </div>
                    }.into_any()
                }
            }}

            // Reply Modal
            {move || {
                if show_reply.get() {
                    if let Some(msg) = message.get() {
                        let props = ComposeProps {
                            project_slug: project_slug.clone(),
                            sender_name: agent_name.clone(),
                            agents: agents.get(),
                            reply_to: Some(ReplyTo {
                                thread_id: msg.thread_id.clone().or_else(|| Some(format!("thread-{}", msg.id))),
                                subject: msg.subject.clone().unwrap_or_default(),
                                recipient_name: Some(msg.sender.clone()),
                            }),
                        };
                        
                        Some(view! {
                            <div class="fixed inset-0 bg-black/50 flex items-center justify-center p-4 z-50">
                                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full max-h-[90vh] overflow-hidden">
                                    <ComposeMessage
                                        props=props
                                        on_close=Callback::new(move |_| show_reply.set(false))
                                        on_sent=Callback::new(move |_| {
                                            show_reply.set(false);
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
        </div>
    }
}

fn format_date(date_str: &str) -> String {
    if date_str.is_empty() {
        return "—".to_string();
    }
    date_str.split('T').next().unwrap_or(date_str).to_string()
}

fn get_importance_badge(importance: &str) -> &'static str {
    match importance {
        "high" => "bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300",
        "low" => "bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400",
        _ => "bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300",
    }
}
