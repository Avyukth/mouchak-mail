//! Inline Message Detail component for SplitViewLayout.
//!
//! Displays message details without navigation elements, designed to be
//! embedded in a split view panel.

use crate::api::client::{self, Message};
use crate::components::MessageDetailHeader;
use leptos::prelude::*;

/// Get badge class for importance level
fn get_importance_badge(importance: &str) -> &'static str {
    match importance {
        "high" => "badge-red",
        "low" => "bg-charcoal-100 dark:bg-charcoal-700 text-charcoal-600 dark:text-charcoal-400",
        _ => "badge-teal",
    }
}

/// Inline message detail component for embedding in split view.
///
/// # Props
/// - `message_id`: ID of the message to display
/// - `project_slug`: Project context for the message
///
/// # Example
/// ```rust,ignore
/// view! {
///     <InlineMessageDetail
///         message_id=selected_id
///         project_slug="my-project".to_string()
///     />
/// }
/// ```
#[component]
pub fn InlineMessageDetail(
    /// Message ID to display
    #[prop(into)]
    message_id: Signal<i64>,
    /// Project slug for context
    #[prop(into)]
    project_slug: Signal<String>,
) -> impl IntoView {
    // State
    let message = RwSignal::new(Option::<Message>::None);
    let loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);

    // Load message when ID changes
    Effect::new(move |_| {
        let id = message_id.get();

        leptos::task::spawn_local(async move {
            loading.set(true);
            error.set(None);

            match client::get_message(&id.to_string()).await {
                Ok(m) => {
                    message.set(Some(m));
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.message));
                    loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="h-full flex flex-col">
            // Error
            {move || {
                error.get().map(|e| view! {
                    <div class="m-4 rounded-xl border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/20 p-4">
                        <div class="flex items-start gap-3">
                            <i data-lucide="triangle-alert" class="icon-lg text-red-500"></i>
                            <p class="text-red-700 dark:text-red-400">{e}</p>
                        </div>
                    </div>
                })
            }}

            // Loading
            {move || {
                if loading.get() {
                    Some(view! {
                        <div class="flex-1 flex items-center justify-center">
                            <div class="flex flex-col items-center gap-4">
                                <i data-lucide="loader-2" class="icon-2xl text-amber-500 animate-spin"></i>
                                <p class="text-charcoal-500 dark:text-charcoal-400 text-sm">"Loading message..."</p>
                            </div>
                        </div>
                    })
                } else {
                    None
                }
            }}

            // Message Content
            {move || {
                let project = project_slug.get();
                if !loading.get() {
                    if let Some(msg) = message.get() {
                        let subject = msg.subject.clone();
                        let body = msg.body_md.clone();
                        let created = msg.created_ts.clone();
                        let importance = msg.importance.clone();
                        let ack_required = msg.ack_required;
                        let thread_id = msg.thread_id.clone();
                        let msg_id = msg.id;
                        let sender = msg.sender_name.clone();

                        Some(view! {
                            <div class="flex-1 overflow-y-auto">
                                // Header with AgentAvatar
                                <MessageDetailHeader
                                    subject={subject.clone()}
                                    sender={sender.clone()}
                                    recipients={vec!["recipient".to_string()]}
                                    project_slug={project.clone()}
                                    sent_at={created.clone()}
                                    message_id={msg_id}
                                />

                                // Badges
                                <div class="px-6 py-3 border-b border-cream-200 dark:border-charcoal-700 flex flex-wrap items-center gap-2">
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
                                                "Ack required"
                                            </span>
                                        })
                                    } else {
                                        None
                                    }}
                                    {thread_id.as_ref().map(|tid| view! {
                                        <span class="badge badge-violet flex items-center gap-1">
                                            <i data-lucide="git-branch" class="icon-xs"></i>
                                            {tid.clone()}
                                        </span>
                                    })}
                                </div>

                                // Message Body
                                <div class="p-6">
                                    <div class="prose dark:prose-invert max-w-none">
                                        <pre class="whitespace-pre-wrap font-sans text-charcoal-700 dark:text-charcoal-300 bg-transparent p-0 overflow-visible text-sm">
                                            {body}
                                        </pre>
                                    </div>
                                </div>

                                // Open in full view link
                                <div class="px-6 py-4 border-t border-cream-200 dark:border-charcoal-700 bg-cream-50/50 dark:bg-charcoal-800/50">
                                    <a
                                        href={format!("/inbox/{}?project={}", msg_id, project)}
                                        class="text-sm text-amber-600 dark:text-amber-400 hover:underline flex items-center gap-1"
                                    >
                                        <i data-lucide="external-link" class="icon-xs"></i>
                                        "Open in full view"
                                    </a>
                                </div>
                            </div>
                        }.into_any())
                    } else {
                        // Not found
                        Some(view! {
                            <div class="flex-1 flex items-center justify-center">
                                <div class="text-center">
                                    <i data-lucide="mail-x" class="icon-2xl text-charcoal-400 mb-4"></i>
                                    <p class="text-charcoal-500">"Message not found"</p>
                                </div>
                            </div>
                        }.into_any())
                    }
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
    fn test_get_importance_badge_high() {
        assert_eq!(get_importance_badge("high"), "badge-red");
    }

    #[test]
    fn test_get_importance_badge_low() {
        assert!(get_importance_badge("low").contains("charcoal"));
    }

    #[test]
    fn test_get_importance_badge_normal() {
        assert_eq!(get_importance_badge("normal"), "badge-teal");
    }

    #[test]
    fn test_get_importance_badge_unknown() {
        assert_eq!(get_importance_badge("unknown"), "badge-teal");
    }
}
