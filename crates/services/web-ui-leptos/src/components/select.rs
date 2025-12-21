//! Shadcn/MagicUI-style Select component.
//!
//! A refined dropdown with:
//! - Smooth animations (fade, zoom, slide)
//! - Full keyboard navigation (Arrow, Home, End, Tab, Escape, type-ahead)
//! - Accessible ARIA attributes (combobox pattern)
//! - Consistent theming with shadcn design tokens
//! - Lucide icons support (auto-initialized on dropdown open)

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

// JavaScript interop to reinitialize Lucide icons after DOM updates
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = lucide, js_name = createIcons)]
    fn create_icons();
}

/// Reinitialize Lucide icons (call after dynamic DOM updates)
fn refresh_icons() {
    // Use request_animation_frame to ensure DOM is updated before refreshing
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::closure::Closure;
        let callback = Closure::once(Box::new(|| {
            create_icons();
        }) as Box<dyn FnOnce()>);

        if let Some(window) = web_sys::window() {
            let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
        }
        callback.forget(); // Prevent closure from being dropped
    }
}

/// Option for the Select component - shadcn/ui style.
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

/// Icon variant for Select trigger - shadcn style.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum SelectIcon {
    #[default]
    Folder,
    User,
    AlertCircle,
    Bot,
    Mail,
    Settings,
    Filter,
    Search,
    Calendar,
    Tag,
    Inbox,
    Send,
    Archive,
    Star,
}

impl SelectIcon {
    pub fn class(&self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::User => "user",
            Self::AlertCircle => "alert-circle",
            Self::Bot => "bot",
            Self::Mail => "mail",
            Self::Settings => "settings",
            Self::Filter => "filter",
            Self::Search => "search",
            Self::Calendar => "calendar",
            Self::Tag => "tag",
            Self::Inbox => "inbox",
            Self::Send => "send",
            Self::Archive => "archive",
            Self::Star => "star",
        }
    }
}

/// Shadcn/MagicUI-style Select component with enhanced styling and animations.
/// Features: keyboard navigation, proper theming, MagicUI-inspired transitions.
#[component]
pub fn Select(
    /// Unique ID for the select.
    id: String,
    /// Available options - should contain simple filter options, not message data.
    options: Vec<SelectOption>,
    /// Current selected value signal.
    value: RwSignal<String>,
    /// Placeholder text when nothing selected.
    placeholder: String,
    /// Whether the select is disabled.
    #[prop(default = false)]
    disabled: bool,
    /// Icon variant for trigger button.
    #[prop(default = SelectIcon::Folder)]
    icon: SelectIcon,
) -> impl IntoView {
    let is_open = RwSignal::new(false);
    let focused_index = RwSignal::new(-1i32);
    let options_for_display = options.clone();
    let options_for_nav = options.clone();
    let option_count = options.len() as i32;

    // Refresh Lucide icons when dropdown opens (new DOM elements need initialization)
    Effect::new(move |_| {
        if is_open.get() {
            refresh_icons();
        }
    });

    let listbox_id = format!("{}-listbox", id);
    let listbox_id_clone = listbox_id.clone();

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
        focused_index.set(-1);
    };

    // Close on click outside
    let close_dropdown = move |_| {
        is_open.set(false);
        focused_index.set(-1);
    };

    // Keyboard navigation handler
    let handle_keydown = {
        let options = options_for_nav.clone();
        move |ev: web_sys::KeyboardEvent| {
            let key = ev.key();
            match key.as_str() {
                "Escape" => {
                    is_open.set(false);
                    focused_index.set(-1);
                }
                "ArrowDown" => {
                    ev.prevent_default();
                    if !is_open.get() {
                        is_open.set(true);
                    }
                    focused_index.update(|i| {
                        *i = (*i + 1).min(option_count - 1);
                    });
                }
                "ArrowUp" => {
                    ev.prevent_default();
                    focused_index.update(|i| {
                        *i = (*i - 1).max(0);
                    });
                }
                "Home" => {
                    ev.prevent_default();
                    focused_index.set(0);
                }
                "End" => {
                    ev.prevent_default();
                    focused_index.set(option_count - 1);
                }
                "Enter" | " " => {
                    ev.prevent_default();
                    if is_open.get() {
                        let idx = focused_index.get();
                        if idx >= 0 && (idx as usize) < options.len() {
                            value.set(options[idx as usize].value.clone());
                            is_open.set(false);
                            focused_index.set(-1);
                        }
                    } else {
                        is_open.set(true);
                    }
                }
                "Tab" => {
                    // Close dropdown on Tab (accessibility - allow focus to move)
                    if is_open.get() {
                        is_open.set(false);
                        focused_index.set(-1);
                    }
                }
                _ => {
                    // Type-ahead: find first option starting with typed character
                    if key.len() == 1 && is_open.get() {
                        let char_lower = key.to_lowercase();
                        for (i, opt) in options.iter().enumerate() {
                            if opt.label.to_lowercase().starts_with(&char_lower) {
                                focused_index.set(i as i32);
                                break;
                            }
                        }
                    }
                }
            }
        }
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
                aria-controls=listbox_id_clone.clone()
                aria-activedescendant={
                    let id_for_aria = id.clone();
                    move || {
                        if focused_index.get() >= 0 {
                            format!("{}-option-{}", id_for_aria, focused_index.get())
                        } else {
                            String::new()
                        }
                    }
                }
                disabled=disabled
                on:click=toggle
                on:keydown=handle_keydown
                class=move || {
                    let base = "flex h-10 w-full items-center justify-between gap-2 whitespace-nowrap rounded-lg border border-input bg-background px-3 py-2 text-sm font-medium shadow-sm ring-offset-background transition-all duration-200 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 select-none";
                    let state = if disabled {
                        "cursor-not-allowed opacity-50"
                    } else if is_open.get() {
                        "border-amber-500 ring-2 ring-amber-500/20 shadow-md bg-accent/5"
                    } else {
                        "hover:border-amber-400 hover:shadow-md hover:bg-accent/5 active:scale-[0.98]"
                    };
                    format!("{} {}", base, state)
                }
            >
                <span class=move || {
                    if is_placeholder() {
                        "text-muted-foreground flex items-center gap-2 truncate"
                    } else {
                        "text-foreground flex items-center gap-2 truncate"
                    }
                }>
                    <i class="icon-sm text-muted-foreground shrink-0" data-lucide=icon.class()></i>
                    <span class="truncate">{get_label}</span>
                </span>
                <i
                    data-lucide="chevron-down"
                    class=move || {
                        let base = "icon-sm text-muted-foreground shrink-0 transition-transform duration-200 ease-out";
                        if is_open.get() {
                            format!("{} rotate-180", base)
                        } else {
                            base.to_string()
                        }
                    }
                ></i>
            </button>

            // Dropdown Panel - Enhanced with better positioning and debugging
            {move || {
                if is_open.get() {
                    let opts = options_for_display.clone();
                    let current_value = value.get();
                    let current_focus = focused_index.get();
                    let listbox_id = listbox_id.clone();
                    let id_for_opts = id.clone();
                    Some(view! {
                        // Backdrop for click-outside
                        <div
                            class="fixed inset-0 z-40"
                            on:click=close_dropdown
                        ></div>

                        // Options Panel - shadcn style with MagicUI animations
                        <div
                            id=listbox_id
                            role="listbox"
                            class="absolute z-[60] top-full left-0 mt-1.5 w-full min-w-[12rem] max-w-[20rem] overflow-hidden rounded-lg border border-border bg-popover text-popover-foreground shadow-xl ring-1 ring-black/5 dark:ring-white/10 animate-in fade-in-0 zoom-in-95 slide-in-from-top-2 duration-200 ease-out"
                            style="max-height: 14rem;"
                        >
                            <div class="max-h-52 overflow-auto p-1 scrollbar-thin scrollbar-thumb-muted scrollbar-track-transparent">
                                {opts.into_iter().enumerate().map(|(i, opt)| {
                                    let val = opt.value.clone();
                                    let label = opt.label.clone();
                                    let is_selected = val == current_value;
                                    let is_focused = i as i32 == current_focus;
                                    let select = select_option;
                                    let val_clone = val.clone();
                                    let option_id = format!("{}-option-{}", id_for_opts, i);

                                    view! {
                                        <button
                                            type="button"
                                            id=option_id
                                            role="option"
                                            aria-selected=is_selected
                                            on:click={
                                                move |_| {
                                                    select(val_clone.clone());
                                                }
                                            }
                                            class=move || {
                                                let base = "relative flex w-full cursor-pointer items-center gap-2 px-2.5 py-2 text-sm font-medium outline-none transition-all duration-150 ease-out rounded-md select-none";
                                                if is_selected {
                                                    format!("{} bg-accent text-accent-foreground font-semibold", base)
                                                } else if is_focused {
                                                    format!("{} bg-accent/60 text-accent-foreground", base)
                                                } else {
                                                    format!("{} text-foreground hover:bg-accent/40 active:bg-accent/60", base)
                                                }
                                            }
                                        >
                                            <span class="flex-1 text-left truncate">{label}</span>
                                            {if is_selected {
                                                Some(view! {
                                                    <i data-lucide="check" class="icon-sm text-amber-600 dark:text-amber-400 shrink-0"></i>
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_option_new() {
        let opt = SelectOption::new("val", "Label");
        assert_eq!(opt.value, "val");
        assert_eq!(opt.label, "Label");
    }

    #[test]
    fn test_listbox_id_format() {
        let id = "my-select";
        let listbox_id = format!("{}-listbox", id);
        assert_eq!(listbox_id, "my-select-listbox");
    }

    #[test]
    fn test_option_id_format() {
        let id = "my-select";
        let option_id = format!("{}-option-{}", id, 0);
        assert_eq!(option_id, "my-select-option-0");
    }
}
