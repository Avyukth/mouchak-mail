//! Message detail page - view a single message with reply functionality.
//! Digital Correspondence design with Lucide icons.

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
            <nav class="flex items-center gap-2 text-sm text-charcoal-500 dark:text-charcoal-400">
                <a href=back_url.clone() class="flex items-center gap-1.5 hover:text-amber-600 dark:hover:text-amber-400 transition-colors">
                    <i data-lucide="arrow-left" class="icon-sm"></i>
                    <span>"Back to Inbox"</span>
                </a>
                {if !agent_name.is_empty() {
                    Some(view! {
                        <i data-lucide="chevron-right" class="icon-xs text-charcoal-400"></i>
                        <span class="badge badge-teal flex items-center gap-1">
                            <i data-lucide="bot" class="icon-xs"></i>
                            {agent_name.clone()}
                        </span>
                    })
                } else {
                    None
                }}
            </nav>

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
                                <p class="text-charcoal-500 dark:text-charcoal-400 text-sm">"Loading message..."</p>
                            </div>
                        </div>
                    }.into_any()
                } else if let Some(msg) = message.get() {
                    let subject = msg.subject.clone();
                    let body = msg.body_md.clone();
                    let created = msg.created_ts.clone();
                    let importance = msg.importance.clone();
                    let ack_required = msg.ack_required;
                    let thread_id = msg.thread_id.clone();
                    let msg_id = msg.id;
                    let sender = msg.sender_name.clone();
                    let can_reply = !agents.get().is_empty();

                    view! {
                        <div class="card-elevated overflow-hidden">
                            // Message Header
                            <div class="p-6 border-b border-cream-200 dark:border-charcoal-700">
                                <div class="flex items-start justify-between gap-4">
                                    <div class="flex-1">
                                        <h1 class="font-display text-xl font-bold text-charcoal-800 dark:text-cream-100 mb-3 flex items-center gap-2">
                                            <i data-lucide="mail" class="icon-lg text-amber-500"></i>
                                            {subject.clone()}
                                        </h1>
                                        <div class="flex flex-wrap items-center gap-2 text-sm">
                                            {if importance != "normal" {
                                                let badge_class = get_importance_badge(&importance);
                                                Some(view! {
                                                    <span class={format!("badge {}", badge_class)}>
                                                        <i data-lucide={if importance == "high" { "alert-circle" } else { "minus-circle" }} class="icon-xs"></i>
                                                        {importance.clone()} " priority"
                                                    </span>
                                                })
                                            } else {
                                                None
                                            }}
                                            {if ack_required {
                                                Some(view! {
                                                    <span class="badge badge-amber flex items-center gap-1">
                                                        <i data-lucide="check-circle" class="icon-xs"></i>
                                                        "Acknowledgment required"
                                                    </span>
                                                })
                                            } else {
                                                None
                                            }}
                                            {thread_id.as_ref().map(|tid| view! {
                                                <span class="badge badge-violet flex items-center gap-1">
                                                    <i data-lucide="git-branch" class="icon-xs"></i>
                                                    "Thread: " {tid.clone()}
                                                </span>
                                            })}
                                        </div>
                                    </div>
                                    {if can_reply {
                                        Some(view! {
                                            <button
                                                on:click=move |_| show_reply.set(true)
                                                class="btn-primary flex items-center gap-2"
                                            >
                                                <i data-lucide="reply" class="icon-sm"></i>
                                                <span>"Reply"</span>
                                            </button>
                                        })
                                    } else {
                                        None
                                    }}
                                </div>

                                <div class="mt-4 text-sm text-charcoal-500 dark:text-charcoal-400">
                                    <div class="flex items-center gap-4">
                                        <span class="flex items-center gap-1.5">
                                            <i data-lucide="user" class="icon-xs"></i>
                                            "From: " {sender.clone()}
                                        </span>
                                        <span class="flex items-center gap-1.5">
                                            <i data-lucide="calendar" class="icon-xs"></i>
                                            "Received: " {format_date(&created)}
                                        </span>
                                    </div>
                                </div>
                            </div>

                            // Message Body
                            <div class="p-6">
                                <div class="prose dark:prose-invert max-w-none">
                                    <pre class="whitespace-pre-wrap font-sans text-charcoal-700 dark:text-charcoal-300 bg-transparent p-0 overflow-visible">
                                        {body}
                                    </pre>
                                </div>
                            </div>

                            // Message Metadata
                            <div class="p-6 bg-cream-50 dark:bg-charcoal-800/50 border-t border-cream-200 dark:border-charcoal-700">
                                <h3 class="text-sm font-medium text-charcoal-700 dark:text-charcoal-300 mb-3 flex items-center gap-2">
                                    <i data-lucide="info" class="icon-sm"></i>
                                    "Message Details"
                                </h3>
                                <dl class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                                    <div>
                                        <dt class="text-charcoal-500 dark:text-charcoal-400">"Message ID"</dt>
                                        <dd class="font-mono text-charcoal-800 dark:text-cream-100 text-xs">{msg_id}</dd>
                                    </div>
                                    <div>
                                        <dt class="text-charcoal-500 dark:text-charcoal-400">"Thread ID"</dt>
                                        <dd class="font-mono text-charcoal-800 dark:text-cream-100 text-xs">
                                            {thread_id.clone().unwrap_or_else(|| "None".to_string())}
                                        </dd>
                                    </div>
                                </dl>
                            </div>
                        </div>

                        // Quick Actions
                        <div class="flex items-center gap-3">
                            <a
                                href=back_url.clone()
                                class="btn-secondary flex items-center gap-2"
                            >
                                <i data-lucide="arrow-left" class="icon-sm"></i>
                                "Back to Inbox"
                            </a>
                            {if can_reply {
                                Some(view! {
                                    <button
                                        on:click=move |_| show_reply.set(true)
                                        class="btn-primary flex items-center gap-2"
                                    >
                                        <i data-lucide="reply" class="icon-sm"></i>
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
                        <div class="card-elevated p-12 text-center">
                            <div class="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-cream-200 dark:bg-charcoal-700 mb-6">
                                <i data-lucide="mail-x" class="icon-2xl text-charcoal-400"></i>
                            </div>
                            <h3 class="font-display text-xl font-semibold text-charcoal-800 dark:text-cream-100 mb-2">"Message not found"</h3>
                            <p class="text-charcoal-500 dark:text-charcoal-400 mb-6">
                                "The message you're looking for doesn't exist or has been deleted."
                            </p>
                            <a
                                href=back_url.clone()
                                class="btn-primary inline-flex items-center gap-2"
                            >
                                <i data-lucide="inbox" class="icon-sm"></i>
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
                                subject: msg.subject.clone(),
                                recipient_name: Some(msg.sender_name.clone()),
                            }),
                        };
                        
                        Some(view! {
                            <div class="fixed inset-0 bg-charcoal-900/60 backdrop-blur-sm flex items-center justify-center p-4 z-50">
                                <div class="card-elevated max-w-2xl w-full max-h-[90vh] overflow-hidden shadow-2xl">
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
        "high" => "badge-red",
        "low" => "bg-charcoal-100 dark:bg-charcoal-700 text-charcoal-600 dark:text-charcoal-400",
        _ => "badge-teal",
    }
}
