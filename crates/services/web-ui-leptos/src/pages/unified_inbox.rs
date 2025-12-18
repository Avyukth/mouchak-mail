//! Unified Inbox page - Gmail-style view of ALL messages across ALL projects.
//! Uses the /mail/api/unified-inbox API from s0j task.
//!
//! Features:
//! - SplitViewLayout for Gmail-style two-column view on desktop
//! - FilterBar with search, project, sender, importance filters
//! - InlineMessageDetail for viewing messages without navigation
//! - Mobile fallback with card-based list

use crate::api::client::{self, UnifiedInboxMessage};
use crate::components::{
    Alert, AlertDescription, AlertTitle, AlertVariant, EmptyDetailPanel, FilterBar, FilterState,
    InlineMessageDetail, MessageListItem, Skeleton, SplitViewLayout,
};
use leptos::prelude::*;

/// Unified Inbox page component.
#[component]
pub fn UnifiedInbox() -> impl IntoView {
    // State
    let messages = RwSignal::new(Vec::<UnifiedInboxMessage>::new());
    let all_messages = RwSignal::new(Vec::<UnifiedInboxMessage>::new()); // Unfiltered for extracting options
    let loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);
    let filter_state = RwSignal::new(FilterState::new());
    let selected_id = RwSignal::new(Option::<i64>::None);

    // Load all messages once on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            loading.set(true);
            error.set(None);

            match client::get_unified_inbox(None, Some(100)).await {
                Ok(msgs) => {
                    // Auto-select first message if nothing selected
                    if selected_id.get_untracked().is_none() {
                        if let Some(first) = msgs.first() {
                            selected_id.set(Some(first.id));
                        }
                    }
                    all_messages.set(msgs.clone());
                    messages.set(msgs);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.message));
                    loading.set(false);
                }
            }
        });
    });

    // Apply filters reactively
    Effect::new(move |_| {
        let filter = filter_state.get();
        let all = all_messages.get();

        let filtered: Vec<UnifiedInboxMessage> = all
            .into_iter()
            .filter(|msg| {
                // Search query filter
                if !filter.query.is_empty() {
                    let q = filter.query.to_lowercase();
                    let matches = msg.subject.to_lowercase().contains(&q)
                        || msg.sender_name.to_lowercase().contains(&q)
                        || msg
                            .thread_id
                            .as_ref()
                            .is_some_and(|t| t.to_lowercase().contains(&q));
                    if !matches {
                        return false;
                    }
                }

                // Importance filter
                if let Some(ref imp) = filter.importance {
                    if msg.importance != *imp {
                        return false;
                    }
                }

                // Sender filter
                if let Some(ref sender) = filter.sender {
                    if msg.sender_name != *sender {
                        return false;
                    }
                }

                // Project filter
                if let Some(ref project) = filter.project {
                    if msg.project_id.to_string() != *project {
                        return false;
                    }
                }

                true
            })
            .collect();

        // If current selection is no longer visible, select first filtered message
        if let Some(current_id) = selected_id.get_untracked() {
            let still_visible = filtered.iter().any(|m| m.id == current_id);
            if !still_visible {
                if let Some(first) = filtered.first() {
                    selected_id.set(Some(first.id));
                }
            }
        } else if let Some(first) = filtered.first() {
            // No selection, select first
            selected_id.set(Some(first.id));
        }

        messages.set(filtered);
    });

    // Extract unique senders for filter dropdown
    let senders = Signal::derive(move || {
        let mut senders: Vec<String> = all_messages
            .get()
            .iter()
            .map(|m| m.sender_name.clone())
            .collect();
        senders.sort();
        senders.dedup();
        senders
    });

    // Extract unique project IDs for filter dropdown
    let projects = Signal::derive(move || {
        let mut projects: Vec<String> = all_messages
            .get()
            .iter()
            .map(|m| m.project_id.to_string())
            .collect();
        projects.sort();
        projects.dedup();
        projects
    });

    // Message count for FilterBar
    let message_count = Signal::derive(move || messages.get().len());

    // Convert messages to MessageListItem format for SplitViewLayout
    let message_list_items = Signal::derive(move || {
        messages
            .get()
            .iter()
            .map(|msg| MessageListItem {
                id: msg.id,
                sender: msg.sender_name.clone(),
                subject: msg.subject.clone(),
                timestamp: format_date(&msg.created_ts),
                unread: false, // TODO: Track read state
                importance: msg.importance.clone(),
                project_slug: msg.project_id.to_string(), // Use project_id as identifier
            })
            .collect::<Vec<_>>()
    });

    // Get project ID for selected message (used for InlineMessageDetail)
    let selected_project = Signal::derive(move || {
        if let Some(id) = selected_id.get() {
            messages
                .get()
                .iter()
                .find(|m| m.id == id)
                .map(|m| m.project_id.to_string())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    // Handle message selection
    let on_select = Callback::new(move |id: i64| {
        selected_id.set(Some(id));
    });

    view! {
        <div class="space-y-6">
            // Header
            <div class="mb-2">
                <h1 class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100 flex items-center gap-3">
                    <i data-lucide="inbox" class="icon-xl text-amber-500"></i>
                    "Unified Inbox"
                </h1>
                <p class="text-sm text-charcoal-500 dark:text-charcoal-400 mt-1">
                    "All messages across all projects"
                </p>
            </div>

            // Filter Bar
            {move || {
                view! {
                    <FilterBar
                        filter_state=filter_state
                        message_count=message_count
                        projects=projects.get()
                        senders=senders.get()
                    />
                }
            }}

            // Error
            {move || {
                error.get().map(|e| view! {
                    <Alert variant=AlertVariant::Destructive>
                        <AlertTitle>"Error loading messages"</AlertTitle>
                        <AlertDescription>{e}</AlertDescription>
                    </Alert>
                })
            }}

            // Loading
            {move || {
                if loading.get() {
                    Some(view! {
                        <div class="space-y-4">
                            <div class="h-16 w-full">
                                <Skeleton class="h-full w-full" />
                            </div>
                            <div class="flex flex-col lg:flex-row gap-4 h-[calc(100vh-14rem)]">
                                <Skeleton class="h-full w-full lg:w-[35%]" />
                                <Skeleton class="hidden lg:block h-full w-[65%]" />
                            </div>
                        </div>
                    })
                } else {
                    None
                }
            }}

            // SplitViewLayout - Gmail-style two-column view
            {move || {
                if !loading.get() {
                    let items = message_list_items.get();
                    let selected_signal: Signal<Option<i64>> = selected_id.into();
                    Some(view! {
                        <SplitViewLayout
                            messages=items
                            selected_id=selected_signal
                            on_select=on_select
                        >
                            {move || {
                                if let Some(id) = selected_id.get() {
                                    view! {
                                        <InlineMessageDetail
                                            message_id=Signal::derive(move || id)
                                            project_slug=selected_project
                                        />
                                    }.into_any()
                                } else {
                                    view! { <EmptyDetailPanel /> }.into_any()
                                }
                            }}
                        </SplitViewLayout>
                    })
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
