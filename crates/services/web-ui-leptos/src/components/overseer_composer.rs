//! OverseerComposer modal component.
//! Specialized composer for Human Overseer interventions.

use super::{Button, ButtonVariant, Input, Select, SelectOption};
use crate::api::client::{self, Agent};
use leptos::prelude::*;

/// Props for OverseerComposer component.
#[derive(Clone)]
pub struct OverseerComposeProps {
    pub project_slug: String,
    pub agents: Vec<Agent>,
    // Reply context (optional)
    pub reply_to_thread_id: Option<String>,
    pub reply_to_recipient: Option<String>,
    pub reply_subject: Option<String>,
}

/// specialized composer for "Overseer" commands.
#[component]
pub fn OverseerComposer(
    props: OverseerComposeProps,
    on_close: Callback<()>,
    on_sent: Callback<()>,
) -> impl IntoView {
    // Form state
    let recipients = RwSignal::new(Vec::<String>::new());
    let subject = RwSignal::new(String::new());
    let body = RwSignal::new(String::new());
    let importance = RwSignal::new("high".to_string()); // Default to High for Overseer
    let ack_required = RwSignal::new(true); // Default to True for Overseer
    let thread_id = RwSignal::new(String::new());

    let sending = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // Initialize from props
    if let Some(ref r) = props.reply_to_recipient {
        recipients.set(vec![r.clone()]);
    }
    if let Some(ref s) = props.reply_subject {
        subject.set(format!("OVERSEER: {}", s.trim_start_matches("re: ")));
    }
    if let Some(ref t) = props.reply_to_thread_id {
        thread_id.set(t.clone());
    }

    let project_slug = props.project_slug.clone();
    // Hardcoded sender for Overseer Mode
    let sender_name = "Overseer".to_string();

    let all_agents = props.agents.clone();

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

    // Toggle All Candidates
    let all_agents_clone = all_agents.clone();
    let toggle_all = move |_| {
        let current_len = recipients.get().len();
        if current_len == all_agents_clone.len() {
            recipients.set(vec![]);
        } else {
            recipients.set(all_agents_clone.iter().map(|a| a.name.clone()).collect());
        }
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
                error.set(Some("Target at least one agent.".to_string()));
                return;
            }
            if subj.trim().is_empty() {
                error.set(Some("Command subject required.".to_string()));
                return;
            }
            if bod.trim().is_empty() {
                error.set(Some("Command instructions required.".to_string()));
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
                    if tid.is_empty() {
                        None
                    } else {
                        Some(tid.as_str())
                    },
                    &imp,
                    ack,
                )
                .await
                {
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
        <div class="flex flex-col h-full max-h-[90vh] bg-red-50/50 dark:bg-red-950/10 border-2 border-red-200 dark:border-red-900 rounded-xl overflow-hidden shadow-2xl">
            // Header - Overseer Style
            <div class="p-4 bg-red-100 dark:bg-red-900/30 border-b border-red-200 dark:border-red-800 flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <div class="p-2 bg-red-200 dark:bg-red-800 rounded-lg">
                        <i data-lucide="shield-alert" class="icon-md text-red-700 dark:text-red-200"></i>
                    </div>
                    <div>
                        <h2 class="text-lg font-bold text-red-900 dark:text-red-100">
                            "Overseer Intervention"
                        </h2>
                        <p class="text-xs text-red-700 dark:text-red-300">
                            "Issuing authoritative commands as 'Overseer'"
                        </p>
                    </div>
                </div>
                <Button
                    variant=ButtonVariant::Ghost
                    size=super::ButtonSize::Icon
                    on_click=Callback::new(move |_| on_close.run(()))
                >
                    <i data-lucide="x" class="icon-sm text-red-800 dark:text-red-200"></i>
                </Button>
            </div>

            // Form
            <div class="flex-1 overflow-y-auto p-4 space-y-5">
                // Broadcaster / Target Selection
                <div>
                     <div class="flex justify-between items-center mb-2">
                        <span class="text-sm font-bold text-charcoal-700 dark:text-charcoal-200">
                            "Target Agents"
                        </span>
                        <button
                            class="text-xs text-red-600 dark:text-red-400 hover:underline font-medium"
                            on:click=toggle_all
                        >
                            {
                                let total_len = all_agents.len();
                                move || if recipients.get().len() == total_len { "Deselect All" } else { "Select All" }
                            }
                        </button>
                    </div>

                    {if all_agents.is_empty() {
                        view! { <p class="text-sm italic text-gray-500">"No agents available."</p> }.into_any()
                    } else {
                        view! {
                            <div class="flex flex-wrap gap-2">
                                {all_agents.iter().map(|agent| {
                                    let name = agent.name.clone();
                                    let name_display = name.clone();
                                    let toggle = toggle_recipient;
                                    view! {
                                        <button
                                            type="button"
                                            on:click=move |_| toggle(name.clone())
                                            class=move || {
                                                if recipients.get().contains(&name_display) {
                                                    "px-3 py-1.5 rounded-md text-sm font-bold transition-colors bg-red-600 text-white shadow-sm ring-2 ring-red-600 ring-offset-1 dark:ring-offset-gray-900"
                                                } else {
                                                    "px-3 py-1.5 rounded-md text-sm font-medium transition-colors bg-white dark:bg-charcoal-800 text-charcoal-600 dark:text-charcoal-300 border border-charcoal-200 dark:border-charcoal-600 hover:border-red-400"
                                                }
                                            }
                                        >
                                            <div class="flex items-center gap-2">
                                                <i data-lucide="bot" class="w-3 h-3"></i>
                                                {name_display.clone()}
                                            </div>
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }}
                </div>

                // Subject / Directive
                <div>
                    <label for="subject" class="block text-sm font-bold text-charcoal-700 dark:text-charcoal-200 mb-1">
                        "Directive / Subject"
                    </label>
                    <Input
                        id="subject".to_string()
                        value=subject
                        placeholder="e.g., STOP IMMEDIATELY, UPDATE PRIORITY...".to_string()
                    />
                </div>

                // Metadata Details (Thread, Importance) - Compact Row
                <div class="grid grid-cols-2 gap-4">
                     <div>
                        <label class="block text-sm font-medium text-charcoal-700 dark:text-charcoal-300 mb-1">
                            "Importance"
                        </label>
                        <Select
                            id="importance".to_string()
                            options=vec![
                                SelectOption::new("normal", "Normal"),
                                SelectOption::new("high", "High (Priority)"),
                            ]
                            value=importance
                            placeholder="Select...".to_string()
                            disabled=false
                        />
                    </div>
                    <div>
                         <label class="block text-sm font-medium text-charcoal-700 dark:text-charcoal-300 mb-1">
                             "Thread Context"
                         </label>
                         <Input
                             id="threadId".to_string()
                             value=thread_id
                             placeholder="New Thread".to_string()
                         />
                    </div>
                </div>

                // Body / Instructions
                <div>
                    <label for="body" class="block text-sm font-bold text-charcoal-700 dark:text-charcoal-200 mb-1">
                        "Instructions"
                    </label>
                    <textarea
                        id="body"
                        prop:value=move || body.get()
                        on:input=move |ev| body.set(event_target_value(&ev))
                        rows="6"
                        placeholder="Detailed instructions for the agents..."
                        class="w-full px-3 py-2 bg-white dark:bg-charcoal-800 border border-gray-300 dark:border-charcoal-600 rounded-lg focus:ring-2 focus:ring-red-500 focus:border-red-500 text-charcoal-900 dark:text-cream-100 font-mono text-sm resize-none"
                    ></textarea>
                </div>

                 // Ack Required (Locked Checked for visual reinforcement, though technically toggleable)
                <div class="flex items-center p-3 bg-amber-50 dark:bg-amber-900/20 rounded-lg border border-amber-200 dark:border-amber-800">
                    <label class="flex items-center gap-2 cursor-pointer w-full">
                        <input
                            type="checkbox"
                            prop:checked=move || ack_required.get()
                            on:change=move |ev| ack_required.set(event_target_checked(&ev))
                            class="w-4 h-4 text-amber-600 border-charcoal-300 rounded focus:ring-amber-500"
                        />
                        <div class="flex flex-col">
                            <span class="text-sm font-semibold text-charcoal-800 dark:text-charcoal-200">
                                "Require Explicit Acknowledgment"
                            </span>
                            <span class="text-xs text-charcoal-500 dark:text-charcoal-400">
                                "Agents must confirm receipt of this directive."
                            </span>
                        </div>
                    </label>
                </div>

                // Error
                {move || {
                    error.get().map(|e| view! {
                        <div class="p-3 bg-red-100 dark:bg-red-900/50 border border-red-300 dark:border-red-700 rounded-lg flex gap-2 items-center">
                            <i data-lucide="alert-circle" class="icon-sm text-red-600 dark:text-red-300"></i>
                            <p class="text-red-800 dark:text-red-200 text-sm font-medium">{e}</p>
                        </div>
                    })
                }}
            </div>

            // Footer
            <div class="p-4 bg-gray-50 dark:bg-charcoal-800/50 border-t border-gray-200 dark:border-charcoal-700 flex justify-end gap-3">
                 <Button
                    variant=ButtonVariant::Ghost
                    on_click=Callback::new(move |_| on_close.run(()))
                >
                    <span>"Cancel"</span>
                </Button>
                <button
                    class="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 bg-red-600 text-white hover:bg-red-700 h-10 px-4 py-2"
                    on:click=move |_| handle_submit(())
                    disabled=move || sending.get() || recipients.get().is_empty()
                >
                    {move || {
                        if sending.get() {
                            view! {
                                <i data-lucide="loader-2" class="icon-sm animate-spin"></i>
                                <span>"Transmitting..."</span>
                            }.into_any()
                        } else {
                            view! {
                                <i data-lucide="megaphone" class="icon-sm"></i>
                                <span>"Broadcast Directive"</span>
                            }.into_any()
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

fn event_target_value(ev: &web_sys::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
}
