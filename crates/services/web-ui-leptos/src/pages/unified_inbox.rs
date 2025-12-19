//! Unified Inbox page - Gmail-style view of ALL messages across ALL projects.
//! Uses the /api/unified-inbox endpoint.
//!
//! Features:
//! - SplitViewLayout for Gmail-style two-column view on desktop
//! - FilterBar with search, project, sender, importance filters
//! - InlineMessageDetail for viewing messages without navigation
//! - Mobile fallback with card-based list

use crate::api::client::{self, Agent, UnifiedInboxMessage};
use crate::components::{
    Alert, AlertDescription, AlertTitle, AlertVariant, Button, ButtonVariant, EmptyDetailPanel,
    FilterBar, FilterState, InlineMessageDetail, MessageListItem, OverseerComposeProps,
    OverseerComposer, Skeleton, SplitViewLayout,
};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

/// Unified Inbox page component.
#[component]
pub fn UnifiedInbox() -> impl IntoView {
    let query = use_query_map();

    // State
    let messages = RwSignal::new(Vec::<UnifiedInboxMessage>::new());
    let all_messages = RwSignal::new(Vec::<UnifiedInboxMessage>::new()); // Unfiltered for extracting options
    let loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);
    let filter_state = RwSignal::new(query.with_untracked(FilterState::from_params_map));
    let selected_id = RwSignal::new(Option::<i64>::None);

    // Overseer Composer state
    let show_overseer = RwSignal::new(false);
    let overseer_agents = RwSignal::new(Vec::<Agent>::new());
    let overseer_project = RwSignal::new(String::new());

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

                // Project filter (uses project_slug for display-friendly matching)
                if let Some(ref project) = filter.project {
                    if msg.project_slug != *project {
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

    // Extract unique project slugs for filter dropdown
    let projects = Signal::derive(move || {
        let mut projects: Vec<String> = all_messages
            .get()
            .iter()
            .map(|m| m.project_slug.clone())
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
                unread: false, // Read state not yet tracked.
                importance: msg.importance.clone(),
                project_slug: msg.project_slug.clone(),
            })
            .collect::<Vec<_>>()
    });

    // Get project slug for selected message (used for InlineMessageDetail)
    let selected_project = Signal::derive(move || {
        if let Some(id) = selected_id.get() {
            messages
                .get()
                .iter()
                .find(|m| m.id == id)
                .map(|m| m.project_slug.clone())
                .unwrap_or_default()
        } else {
            String::new()
        }
    });

    // Handle message selection
    let on_select = Callback::new(move |id: i64| {
        selected_id.set(Some(id));
    });

    // Handle Overseer button click
    let open_overseer = move |_| {
        let project_slug = selected_project.get();
        if project_slug.is_empty() {
            error.set(Some(
                "Select a message first to use Overseer mode.".to_string(),
            ));
            return;
        }
        // Fetch agents for the selected project
        overseer_project.set(project_slug.clone());
        leptos::task::spawn_local(async move {
            match client::get_agents(&project_slug).await {
                Ok(agents) => {
                    overseer_agents.set(agents);
                    show_overseer.set(true);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load agents: {}", e.message)));
                }
            }
        });
    };

    // Refresh messages after sending
    let refresh_messages = move || {
        leptos::task::spawn_local(async move {
            if let Ok(msgs) = client::get_unified_inbox(None, Some(100)).await {
                all_messages.set(msgs.clone());
                messages.set(msgs);
            }
        });
    };

    view! {
        <div class="space-y-6">
            // Overseer Composer Modal
            {move || {
                if show_overseer.get() {
                    let agents = overseer_agents.get();
                    let project = overseer_project.get();
                    Some(view! {
                        <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
                            <div
                                class="fixed inset-0 bg-charcoal-900/50 backdrop-blur-sm"
                                on:click=move |_| show_overseer.set(false)
                            ></div>
                            <div class="relative w-full max-w-2xl animate-scale-in">
                                <OverseerComposer
                                    props=OverseerComposeProps {
                                        project_slug: project,
                                        agents,
                                        reply_to_thread_id: None,
                                        reply_to_recipient: None,
                                        reply_subject: None,
                                    }
                                    on_close=Callback::new(move |_| show_overseer.set(false))
                                    on_sent=Callback::new(move |_| {
                                        show_overseer.set(false);
                                        refresh_messages();
                                    })
                                />
                            </div>
                        </div>
                    })
                } else {
                    None
                }
            }}

            // Header
            <div class="flex items-center justify-between mb-2">
                <div>
                    <h1 class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100 flex items-center gap-3">
                        <i data-lucide="inbox" class="icon-xl text-amber-500"></i>
                        "Unified Inbox"
                    </h1>
                    <p class="text-sm text-charcoal-500 dark:text-charcoal-400 mt-1">
                        "All messages across all projects"
                    </p>
                </div>
                <Button
                    variant=ButtonVariant::Destructive
                    on_click=Callback::new(open_overseer)
                >
                    <i data-lucide="shield-alert" class="icon-sm mr-2"></i>
                    "Overseer Mode"
                </Button>
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
