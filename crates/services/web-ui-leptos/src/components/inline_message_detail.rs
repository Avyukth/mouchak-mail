//! Inline Message Detail component for SplitViewLayout.
//!
//! Displays message details without navigation elements, designed to be
//! embedded in a split view panel.

use crate::api::client::{self, Message};
use crate::components::{
    Alert, AlertDescription, AlertTitle, AlertVariant, Badge, BadgeVariant, MessageDetailHeader,
    Skeleton,
};
use leptos::prelude::*;

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
                    <div class="p-4">
                        <Alert variant=AlertVariant::Destructive>
                            <AlertTitle>"Error loading message"</AlertTitle>
                            <AlertDescription>{e}</AlertDescription>
                        </Alert>
                    </div>
                })
            }}

            // Loading
            {move || {
                if loading.get() {
                    Some(view! {
                        <div class="flex-1 p-6 space-y-6">
                            <div class="flex items-start gap-4">
                                <Skeleton class="w-12 h-12 rounded-full" />
                                <div class="space-y-2 flex-1">
                                    <Skeleton class="h-6 w-3/4" />
                                    <Skeleton class="h-4 w-1/2" />
                                </div>
                            </div>
                            <div class="space-y-2">
                                <Skeleton class="h-4 w-full" />
                                <Skeleton class="h-4 w-full" />
                                <Skeleton class="h-4 w-2/3" />
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
                                        let variant = if importance == "high" { BadgeVariant::Destructive } else { BadgeVariant::Secondary };
                                        Some(view! {
                                            <Badge variant=variant class="flex items-center gap-1">
                                                <i data-lucide={if importance == "high" { "alert-circle" } else { "minus-circle" }} class="icon-xs"></i>
                                                {importance.clone()} " priority"
                                            </Badge>
                                        })
                                    } else {
                                        None
                                    }}
                                    {if ack_required {
                                        Some(view! {
                                            <Badge variant=BadgeVariant::Warning class="flex items-center gap-1">
                                                <i data-lucide="check-circle" class="icon-xs"></i>
                                                "Ack required"
                                            </Badge>
                                        })
                                    } else {
                                        None
                                    }}
                                    {thread_id.clone().map(|tid| view! {
                                        <Badge variant=BadgeVariant::Outline class="flex items-center gap-1">
                                            <i data-lucide="git-branch" class="icon-xs"></i>
                                            {tid}
                                        </Badge>
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
