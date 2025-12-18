//! Split View Layout component for Gmail-style inbox.
//!
//! Provides a two-column layout with message list (35%) and detail panel (65%).
//! Responsive design with mobile single-column fallback.

use leptos::prelude::*;

/// Props for message list items
#[derive(Debug, Clone, PartialEq)]
pub struct MessageListItem {
    /// Unique message ID
    pub id: i64,
    /// Sender name
    pub sender: String,
    /// Message subject
    pub subject: String,
    /// Timestamp string
    pub timestamp: String,
    /// Whether message is unread
    pub unread: bool,
    /// Importance level
    pub importance: String,
    /// Project slug
    pub project_slug: String,
}

/// Empty state placeholder for the detail panel
#[component]
pub fn EmptyDetailPanel() -> impl IntoView {
    view! {
        <div class="h-full flex flex-col items-center justify-center text-charcoal-400 dark:text-charcoal-500">
            <i data-lucide="mail-open" class="w-16 h-16 mb-4 opacity-50"></i>
            <p class="text-lg font-medium">"Select a message"</p>
            <p class="text-sm mt-1">"Choose a message from the list to view its contents"</p>
        </div>
    }
}

/// Message list item component
#[component]
pub fn MessageListItemView(
    /// The message item data
    item: MessageListItem,
    /// Whether this item is currently selected
    #[prop(into)]
    selected: Signal<bool>,
    /// Callback when item is clicked
    on_click: Callback<i64>,
) -> impl IntoView {
    let id = item.id;
    let sender = item.sender.clone();
    let subject = item.subject.clone();
    let timestamp = item.timestamp.clone();
    let unread = item.unread;
    let importance = item.importance.clone();

    view! {
        <button
            class={move || format!(
                "w-full text-left p-4 border-b border-cream-200 dark:border-charcoal-700 \
                 hover:bg-cream-50 dark:hover:bg-charcoal-800 transition-colors \
                 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-amber-500 \
                 {} {}",
                if selected.get() {
                    "bg-amber-50 dark:bg-amber-900/20 border-l-4 border-l-amber-500"
                } else {
                    "border-l-4 border-l-transparent"
                },
                if unread { "font-semibold" } else { "" }
            )}
            on:click=move |_| on_click.run(id)
        >
            <div class="flex items-start justify-between gap-2">
                <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2">
                        {if unread {
                            Some(view! {
                                <span class="w-2 h-2 bg-amber-500 rounded-full flex-shrink-0"></span>
                            })
                        } else {
                            None
                        }}
                        <span class="truncate text-charcoal-700 dark:text-cream-200">
                            {sender}
                        </span>
                        {if importance == "high" {
                            Some(view! {
                                <i data-lucide="alert-circle" class="icon-xs text-rose-500 flex-shrink-0"></i>
                            })
                        } else {
                            None
                        }}
                    </div>
                    <p class="text-sm text-charcoal-600 dark:text-charcoal-300 truncate mt-1">
                        {subject}
                    </p>
                </div>
                <span class="text-xs text-charcoal-400 dark:text-charcoal-500 whitespace-nowrap">
                    {timestamp}
                </span>
            </div>
        </button>
    }
}

/// Split view layout container component.
///
/// # Props
/// - `messages`: List of messages to display
/// - `selected_id`: Signal for currently selected message ID
/// - `on_select`: Callback when a message is selected
/// - `detail_content`: Content to show in detail panel (slot)
///
/// # Example
/// ```rust,ignore
/// let selected = RwSignal::new(None::<i64>);
/// view! {
///     <SplitViewLayout
///         messages=messages
///         selected_id=selected.into()
///         on_select=Callback::new(move |id| selected.set(Some(id)))
///     >
///         {move || match selected.get() {
///             Some(id) => view! { <MessageDetail id=id /> }.into_any(),
///             None => view! { <EmptyDetailPanel /> }.into_any(),
///         }}
///     </SplitViewLayout>
/// }
/// ```
#[component]
pub fn SplitViewLayout(
    /// Messages to display in the list
    messages: Vec<MessageListItem>,
    /// Currently selected message ID
    #[prop(into)]
    selected_id: Signal<Option<i64>>,
    /// Callback when a message is selected
    on_select: Callback<i64>,
    /// Content for the detail panel
    children: Children,
) -> impl IntoView {
    // Clone for keyboard handler
    let messages_for_keyboard = messages.clone();
    let on_select_keyboard = on_select;

    // Keyboard navigation handler
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        if key == "ArrowDown" || key == "ArrowUp" || key == "Enter" {
            ev.prevent_default();

            if messages_for_keyboard.is_empty() {
                return;
            }

            let current_id = selected_id.get();
            let current_idx =
                current_id.and_then(|id| messages_for_keyboard.iter().position(|m| m.id == id));

            match key.as_str() {
                "ArrowDown" => {
                    let next_idx = match current_idx {
                        Some(idx) if idx + 1 < messages_for_keyboard.len() => idx + 1,
                        None => 0,
                        _ => return,
                    };
                    on_select_keyboard.run(messages_for_keyboard[next_idx].id);
                }
                "ArrowUp" => {
                    let prev_idx = match current_idx {
                        Some(idx) if idx > 0 => idx - 1,
                        None => messages_for_keyboard.len() - 1,
                        _ => return,
                    };
                    on_select_keyboard.run(messages_for_keyboard[prev_idx].id);
                }
                "Enter" => {
                    // Enter already handled by selection
                }
                _ => {}
            }
        }
    };

    view! {
        // Desktop: Two-column split view
        <div
            class="hidden lg:grid lg:grid-cols-[35%_65%] h-[calc(100vh-12rem)] border border-cream-200 dark:border-charcoal-700 rounded-xl overflow-hidden"
            tabindex="0"
            on:keydown=on_keydown
        >
            // Message List Panel
            <div class="border-r border-cream-200 dark:border-charcoal-700 overflow-y-auto bg-white dark:bg-charcoal-900">
                {if messages.is_empty() {
                    view! {
                        <div class="p-8 text-center text-charcoal-400">
                            <i data-lucide="inbox" class="w-12 h-12 mx-auto mb-3 opacity-50"></i>
                            <p>"No messages"</p>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="divide-y divide-cream-200 dark:divide-charcoal-700">
                            {messages.iter().map(|msg| {
                                let msg_id = msg.id;
                                let is_selected = Signal::derive(move || selected_id.get() == Some(msg_id));
                                view! {
                                    <MessageListItemView
                                        item=msg.clone()
                                        selected=is_selected
                                        on_click=on_select
                                    />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }}
            </div>

            // Detail Panel
            <div class="overflow-y-auto bg-cream-50 dark:bg-charcoal-800">
                {children()}
            </div>
        </div>

        // Mobile: Single column (list only)
        <div class="lg:hidden">
            {if messages.is_empty() {
                view! {
                    <div class="p-8 text-center text-charcoal-400 card-elevated">
                        <i data-lucide="inbox" class="w-12 h-12 mx-auto mb-3 opacity-50"></i>
                        <p>"No messages"</p>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="card-elevated overflow-hidden divide-y divide-cream-200 dark:divide-charcoal-700">
                        {messages.iter().map(|msg| {
                            let project = msg.project_slug.clone();
                            let id = msg.id;
                            view! {
                                <a
                                    href={format!("/inbox/{}?project={}", id, project)}
                                    class="block p-4 hover:bg-cream-50 dark:hover:bg-charcoal-800 transition-colors"
                                >
                                    <div class="flex items-start justify-between gap-2">
                                        <div class="flex-1 min-w-0">
                                            <span class="font-medium text-charcoal-700 dark:text-cream-200 truncate block">
                                                {msg.sender.clone()}
                                            </span>
                                            <p class="text-sm text-charcoal-600 dark:text-charcoal-300 truncate">
                                                {msg.subject.clone()}
                                            </p>
                                        </div>
                                        <span class="text-xs text-charcoal-400 whitespace-nowrap">
                                            {msg.timestamp.clone()}
                                        </span>
                                    </div>
                                </a>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_list_item_creation() {
        let item = MessageListItem {
            id: 1,
            sender: "worker-1".to_string(),
            subject: "Test Subject".to_string(),
            timestamp: "10:30 AM".to_string(),
            unread: true,
            importance: "normal".to_string(),
            project_slug: "my-project".to_string(),
        };

        assert_eq!(item.id, 1);
        assert_eq!(item.sender, "worker-1");
        assert!(item.unread);
    }

    #[test]
    fn test_message_list_item_high_importance() {
        let item = MessageListItem {
            id: 2,
            sender: "urgent-sender".to_string(),
            subject: "Urgent".to_string(),
            timestamp: "Now".to_string(),
            unread: false,
            importance: "high".to_string(),
            project_slug: "proj".to_string(),
        };

        assert_eq!(item.importance, "high");
    }
}
