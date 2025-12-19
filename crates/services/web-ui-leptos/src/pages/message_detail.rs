//! Message detail page - view a single message with reply functionality.
//! Digital Correspondence design with Lucide icons.

use crate::api::client::{self, Agent, Message};
use crate::components::{
    Button, ButtonVariant, ComposeMessage, ComposeProps, MessageDetailHeader, ReplyTo,
};
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

/// Message detail page component.
#[component]
pub fn MessageDetail() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();

    // Use with_untracked since route params don't change without navigation
    // This avoids reactive tracking warnings while still getting the values
    let message_id = params.with_untracked(|p| p.get("id").unwrap_or_default());
    let project_slug = query.with_untracked(|q| q.get("project").unwrap_or_default());
    let agent_name = query.with_untracked(|q| q.get("agent").unwrap_or_default());

    // State
    let message = RwSignal::new(Option::<Message>::None);
    let agents = RwSignal::new(Vec::<Agent>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);
    let show_reply = RwSignal::new(false);

    // Clone values for use in Effect
    let id_for_effect = message_id.clone();
    let project_for_effect = project_slug.clone();

    // Load message and agents
    Effect::new(move |_| {
        let id = id_for_effect.clone();
        let project = project_for_effect.clone();

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
            if !project.is_empty()
                && let Ok(a) = client::get_agents(&project).await
            {
                agents.set(a);
            }
        });
    });

    // Back to unified inbox URL
    let back_url = "/mail/unified".to_string();

    view! {
        <div class="space-y-6">
            // Breadcrumb / Back
            <nav class="flex items-center gap-2 text-sm text-charcoal-500 dark:text-charcoal-400">
                <a href=back_url.clone() class="flex items-center gap-1.5 hover:text-amber-600 dark:hover:text-amber-400 transition-colors">
                    <i data-lucide="arrow-left" class="icon-sm"></i>
                    <span>"Back to Inbox"</span>
                </a>
                {
                    let agent = agent_name.clone();
                    if !agent.is_empty() {
                        Some(view! {
                            <i data-lucide="chevron-right" class="icon-xs text-charcoal-400"></i>
                            <span class="badge badge-teal flex items-center gap-1">
                                <i data-lucide="bot" class="icon-xs"></i>
                                {agent}
                            </span>
                        })
                    } else {
                        None
                    }
                }
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
            {
                let back_url_for_content = back_url.clone();
                let project_slug_for_detail = project_slug.clone();
                move || {
                let back_url = back_url_for_content.clone();
                let project_slug = project_slug_for_detail.clone();
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
                            // Message Header using new component with AgentAvatar
                            <MessageDetailHeader
                                subject={subject.clone()}
                                sender={sender.clone()}
                                recipients={msg.recipients.clone()}
                                project_slug={project_slug.clone()}
                                sent_at={created.clone()}
                                message_id={msg_id}
                            />

                            // Badges and Reply button
                            <div class="px-6 py-3 border-b border-cream-200 dark:border-charcoal-700 flex flex-wrap items-center justify-between gap-2">
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
                                {if can_reply {
                                    Some(view! {
                                        <Button
                                            variant=ButtonVariant::Default
                                            on_click=Callback::new(move |_| show_reply.set(true))
                                        >
                                            <i data-lucide="reply" class="icon-sm"></i>
                                            <span>"Reply"</span>
                                        </Button>
                                    })
                                } else {
                                    None
                                }}
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
                                    <Button
                                        variant=ButtonVariant::Default
                                        on_click=Callback::new(move |_| show_reply.set(true))
                                    >
                                        <i data-lucide="reply" class="icon-sm"></i>
                                        "Reply to Message"
                                    </Button>
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
            {
                let project_for_modal = project_slug.clone();
                let agent_for_modal = agent_name.clone();
                move || {
                if show_reply.get() {
                    if let Some(msg) = message.get() {
                        let props = ComposeProps {
                            project_slug: project_for_modal.clone(),
                            sender_name: agent_for_modal.clone(),
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

#[allow(dead_code)] // Utility function for future date formatting needs
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
