//! ComposeMessage modal component.

use leptos::prelude::*;
use crate::api::client::{self, Agent};
use super::{Select, SelectOption};

/// Props for ComposeMessage component.
#[derive(Clone)]
pub struct ComposeProps {
    pub project_slug: String,
    pub sender_name: String,
    pub agents: Vec<Agent>,
    pub reply_to: Option<ReplyTo>,
}

#[derive(Clone)]
pub struct ReplyTo {
    pub thread_id: Option<String>,
    pub subject: String,
    pub recipient_name: Option<String>,
}

/// ComposeMessage modal component.
#[component]
pub fn ComposeMessage(
    props: ComposeProps,
    on_close: Callback<()>,
    on_sent: Callback<()>,
) -> impl IntoView {
    // Form state
    let recipients = RwSignal::new(Vec::<String>::new());
    let subject = RwSignal::new(String::new());
    let body = RwSignal::new(String::new());
    let importance = RwSignal::new("normal".to_string());
    let ack_required = RwSignal::new(false);
    let thread_id = RwSignal::new(String::new());
    
    let sending = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // Initialize from reply_to if present
    let is_reply = props.reply_to.is_some();
    if let Some(ref reply) = props.reply_to {
        if let Some(ref recipient) = reply.recipient_name {
            recipients.set(vec![recipient.clone()]);
        }
        subject.set(format!("Re: {}", reply.subject.trim_start_matches("Re: ")));
        if let Some(ref tid) = reply.thread_id {
            thread_id.set(tid.clone());
        }
    }

    let project_slug = props.project_slug.clone();
    let sender_name = props.sender_name.clone();
    
    // Available recipients (exclude sender)
    let available_recipients: Vec<Agent> = props.agents
        .iter()
        .filter(|a| a.name != sender_name)
        .cloned()
        .collect();

    // Toggle recipient selection
    let toggle_recipient = move |name: String| {
        let mut current = recipients.get();
        if current.contains(&name) {
            current.retain(|r| r != &name);
        } else {
            current.push(name);
        }
        recipients.set(current);
    };

    // Send message handler
    let handle_submit = {
        let project_slug = project_slug.clone();
        let sender_name = sender_name.clone();
        move |_| {
            let recips = recipients.get();
            let subj = subject.get();
            let bod = body.get();
            
            if recips.is_empty() {
                error.set(Some("Please select at least one recipient".to_string()));
                return;
            }
            if subj.trim().is_empty() {
                error.set(Some("Please enter a subject".to_string()));
                return;
            }
            if bod.trim().is_empty() {
                error.set(Some("Please enter a message body".to_string()));
                return;
            }

            sending.set(true);
            error.set(None);

            let project = project_slug.clone();
            let sender = sender_name.clone();
            let tid = thread_id.get();
            let imp = importance.get();
            let ack = ack_required.get();
            let on_sent = on_sent;

            leptos::task::spawn_local(async move {
                match client::send_message(
                    &project,
                    &sender,
                    &recips,
                    &subj,
                    &bod,
                    if tid.is_empty() { None } else { Some(tid.as_str()) },
                    &imp,
                    ack,
                ).await {
                    Ok(_) => {
                        on_sent.run(());
                    }
                    Err(e) => {
                        error.set(Some(e.message));
                        sending.set(false);
                    }
                }
            });
        }
    };

    view! {
        <div class="flex flex-col h-full max-h-[90vh]">
            // Header
            <div class="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                <h2 class="text-lg font-semibold text-gray-900 dark:text-white">
                    {if is_reply { "Reply" } else { "New Message" }}
                </h2>
                <button
                    on:click=move |_| on_close.run(())
                    class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                >
                    <span class="text-xl">"×"</span>
                </button>
            </div>

            // Form
            <div class="flex-1 overflow-y-auto p-4 space-y-4">
                // From (readonly)
                <div>
                    <span class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "From"
                    </span>
                    <div class="px-4 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg text-gray-700 dark:text-gray-300">
                        {sender_name.clone()}
                        <span class="text-gray-500 dark:text-gray-400 text-sm ml-2">
                            "(" {project_slug.clone()} ")"
                        </span>
                    </div>
                </div>

                // Recipients
                <div>
                    <span class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "To *"
                    </span>
                    {if available_recipients.is_empty() {
                        view! {
                            <p class="text-sm text-gray-500 dark:text-gray-400 italic">
                                "No other agents in this project. Register more agents to send messages."
                            </p>
                        }.into_any()
                    } else {
                        view! {
                            <div class="flex flex-wrap gap-2">
                                {available_recipients.iter().map(|agent| {
                                    let name = agent.name.clone();
                                    let name_display = name.clone();
                                    let toggle = toggle_recipient;
                                    view! {
                                        <button
                                            type="button"
                                            on:click=move |_| toggle(name.clone())
                                            class=move || {
                                                if recipients.get().contains(&name_display) {
                                                    "px-3 py-1.5 rounded-full text-sm transition-colors bg-primary-600 text-white"
                                                } else {
                                                    "px-3 py-1.5 rounded-full text-sm transition-colors bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600"
                                                }
                                            }
                                        >
                                            {name_display.clone()}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                            {move || {
                                let recips = recipients.get();
                                if !recips.is_empty() {
                                    Some(view! {
                                        <p class="mt-2 text-sm text-gray-500 dark:text-gray-400">
                                            "Selected: " {recips.join(", ")}
                                        </p>
                                    })
                                } else {
                                    None
                                }
                            }}
                        }.into_any()
                    }}
                </div>

                // Subject
                <div>
                    <label for="subject" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "Subject *"
                    </label>
                    <input
                        id="subject"
                        type="text"
                        prop:value=move || subject.get()
                        on:input=move |ev| subject.set(event_target_value(&ev))
                        placeholder="Enter subject..."
                        class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                    />
                </div>

                // Thread ID (only for new messages)
                {if !is_reply {
                    Some(view! {
                        <div>
                            <label for="threadId" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                "Thread ID "
                                <span class="text-gray-400 font-normal">"(optional)"</span>
                            </label>
                            <input
                                id="threadId"
                                type="text"
                                prop:value=move || thread_id.get()
                                on:input=move |ev| thread_id.set(event_target_value(&ev))
                                placeholder="Leave empty for new thread"
                                class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                            />
                        </div>
                    })
                } else {
                    None
                }}

                // Body
                <div>
                    <label for="body" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "Message * "
                        <span class="text-gray-400 font-normal">"(Markdown supported)"</span>
                    </label>
                    <textarea
                        id="body"
                        prop:value=move || body.get()
                        on:input=move |ev| body.set(event_target_value(&ev))
                        rows="8"
                        placeholder="Write your message..."
                        class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent resize-none font-mono text-sm"
                    ></textarea>
                </div>

                // Options
                <div class="flex flex-wrap gap-4">
                    // Importance
                    <div class="w-40">
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Importance"
                        </label>
                        <Select
                            id="importance".to_string()
                            options=vec![
                                SelectOption::new("low", "Low"),
                                SelectOption::new("normal", "Normal"),
                                SelectOption::new("high", "High"),
                            ]
                            value=importance
                            placeholder="Select...".to_string()
                            disabled=false
                        />
                    </div>

                    // Ack Required
                    <div class="flex items-center pt-6">
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input
                                type="checkbox"
                                prop:checked=move || ack_required.get()
                                on:change=move |ev| ack_required.set(event_target_checked(&ev))
                                class="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                            />
                            <span class="text-sm text-gray-700 dark:text-gray-300">
                                "Require acknowledgment"
                            </span>
                        </label>
                    </div>
                </div>

                // Error
                {move || {
                    error.get().map(|e| view! {
                        <div class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                            <p class="text-red-700 dark:text-red-400 text-sm">{e}</p>
                        </div>
                    })
                }}
            </div>

            // Footer
            <div class="p-4 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                <button
                    type="button"
                    on:click=move |_| on_close.run(())
                    class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                >
                    "Cancel"
                </button>
                <button
                    on:click=handle_submit
                    disabled=move || sending.get() || recipients.get().is_empty()
                    class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
                >
                    {move || {
                        if sending.get() {
                            view! {
                                <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                                <span>"Sending..."</span>
                            }.into_any()
                        } else {
                            view! { <span>"Send Message"</span> }.into_any()
                        }
                    }}
                </button>
            </div>
        </div>
    }
}

fn event_target_checked(ev: &web_sys::Event) -> bool {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|el| el.checked())
        .unwrap_or(false)
}
