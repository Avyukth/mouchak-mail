//! Shadcn-style Select component.
//! Custom dropdown with button trigger and floating options panel.

use leptos::prelude::*;

/// Option for the Select component.
#[derive(Clone, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// Shadcn-style Select component.
#[component]
pub fn Select(
    /// Unique ID for the select.
    id: String,
    /// Available options.
    options: Vec<SelectOption>,
    /// Current selected value signal.
    value: RwSignal<String>,
    /// Placeholder text when nothing selected.
    placeholder: String,
    /// Whether the select is disabled.
    #[prop(default = false)]
    disabled: bool,
    /// Optional icon name (lucide).
    #[prop(optional)]
    icon: Option<&'static str>,
) -> impl IntoView {
    let is_open = RwSignal::new(false);
    let options_for_display = options.clone();

    // Get current label
    let get_label = {
        let options = options.clone();
        let placeholder = placeholder.clone();
        move || {
            let val = value.get();
            if val.is_empty() {
                placeholder.clone()
            } else {
                options
                    .iter()
                    .find(|o| o.value == val)
                    .map(|o| o.label.clone())
                    .unwrap_or(val)
            }
        }
    };

    // Check if placeholder is showing
    let is_placeholder = move || value.get().is_empty();

    // Toggle dropdown
    let toggle = move |_| {
        if !disabled {
            is_open.update(|v| *v = !*v);
        }
    };

    // Select option
    let select_option = move |val: String| {
        value.set(val);
        is_open.set(false);
    };

    // Close on click outside
    let close_dropdown = move |_| {
        is_open.set(false);
    };

    view! {
        <div class="relative">
            // Trigger Button
            <button
                type="button"
                id=id.clone()
                role="combobox"
                aria-expanded=move || is_open.get()
                aria-haspopup="listbox"
                disabled=disabled
                on:click=toggle
                class=move || {
                    let base = "flex h-10 w-full items-center justify-between gap-2 whitespace-nowrap rounded-lg border bg-white dark:bg-charcoal-800 px-3 py-2 text-sm shadow-sm ring-offset-white transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2";
                    let state = if disabled {
                        "cursor-not-allowed opacity-50 border-cream-200 dark:border-charcoal-700"
                    } else if is_open.get() {
                        "border-amber-400 ring-2 ring-amber-400/20"
                    } else {
                        "border-cream-200 dark:border-charcoal-700 hover:border-amber-300 dark:hover:border-amber-700"
                    };
                    format!("{} {}", base, state)
                }
            >
                <span class=move || {
                    if is_placeholder() {
                        "text-charcoal-400 dark:text-charcoal-500 flex items-center gap-2"
                    } else {
                        "text-charcoal-800 dark:text-cream-100 flex items-center gap-2"
                    }
                }>
                    {icon.map(|icon| view! {
                        <i data-lucide=icon class="icon-sm text-charcoal-400"></i>
                    })}
                    {get_label}
                </span>
                <i
                    data-lucide="chevron-down"
                    class=move || {
                        let base = "icon-sm text-charcoal-400 transition-transform duration-200";
                        if is_open.get() {
                            format!("{} rotate-180", base)
                        } else {
                            base.to_string()
                        }
                    }
                ></i>
            </button>

            // Dropdown Panel
            {move || {
                if is_open.get() {
                    let opts = options_for_display.clone();
                    let current_value = value.get();
                    Some(view! {
                        // Backdrop for click-outside
                        <div
                            class="fixed inset-0 z-40"
                            on:click=close_dropdown
                        ></div>

                        // Options Panel
                        <div class="absolute z-50 mt-1 w-full min-w-[8rem] overflow-hidden rounded-lg border border-cream-200 dark:border-charcoal-700 bg-white dark:bg-charcoal-800 shadow-lg animate-slide-up">
                            <div class="max-h-60 overflow-auto py-1" role="listbox">
                                {opts.into_iter().map(|opt| {
                                    let val = opt.value.clone();
                                    let label = opt.label.clone();
                                    let is_selected = val == current_value;
                                    let select = select_option.clone();
                                    let val_clone = val.clone();

                                    view! {
                                        <button
                                            type="button"
                                            role="option"
                                            aria-selected=is_selected
                                            on:click=move |_| select(val_clone.clone())
                                            class=move || {
                                                let base = "relative flex w-full cursor-pointer items-center px-3 py-2 text-sm outline-none transition-colors";
                                                if is_selected {
                                                    format!("{} bg-amber-50 dark:bg-amber-900/20 text-amber-700 dark:text-amber-300", base)
                                                } else {
                                                    format!("{} text-charcoal-700 dark:text-charcoal-300 hover:bg-cream-100 dark:hover:bg-charcoal-700", base)
                                                }
                                            }
                                        >
                                            <span class="flex-1 text-left">{label}</span>
                                            {if is_selected {
                                                Some(view! {
                                                    <i data-lucide="check" class="icon-sm text-amber-600 dark:text-amber-400 ml-2"></i>
                                                })
                                            } else {
                                                None
                                            }}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}
