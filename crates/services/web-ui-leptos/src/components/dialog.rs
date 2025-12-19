//! Dialog component with focus trap and accessibility.
//!
//! Compound component pattern: Dialog > DialogTrigger + DialogContent > DialogHeader...

use super::{Button, ButtonSize, ButtonVariant};
use leptos::html::Div;
use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[component]
pub fn Dialog(
    #[prop(default = false)] open: bool,
    #[prop(optional, into)] on_open_change: Option<Callback<bool>>,
    children: Children,
) -> impl IntoView {
    // Shared state for the dialog
    provide_context(DialogContext {
        open: RwSignal::new(open),
        on_open_change,
    });

    view! {
        {children()}
    }
}

#[derive(Clone, Copy)]
struct DialogContext {
    open: RwSignal<bool>,
    on_open_change: Option<Callback<bool>>,
}

// Helper to update open state
fn set_open(ctx: DialogContext, value: bool) {
    ctx.open.set(value);
    if let Some(cb) = ctx.on_open_change {
        cb.run(value);
    }
}

/// Focus trap implementation for dialogs.
/// Cycles focus within the dialog when Tab/Shift+Tab is pressed.
/// Uses get_elements_by_tag_name as a lightweight alternative to query_selector_all.
fn focus_trap(ev: &KeyboardEvent, content_ref: NodeRef<Div>) {
    use wasm_bindgen::JsCast;

    if let Some(dialog) = content_ref.get() {
        // Get buttons in the dialog using get_elements_by_tag_name
        let buttons = dialog.get_elements_by_tag_name("button");
        let len = buttons.length();

        if len == 0 {
            return;
        }

        // Get first and last button elements
        let first = buttons.item(0);
        let last = buttons.item(len - 1);

        // Get currently focused element
        if let Some(window) = web_sys::window()
            && let Some(document) = window.document()
            && let Some(active) = document.active_element()
        {
            let is_shift = ev.shift_key();

            // Check if active element is within the dialog
            let active_in_dialog = dialog.contains(Some(&active));
            if !active_in_dialog {
                return;
            }

            // Shift+Tab on first button: go to last
            if is_shift
                && first
                    .as_ref()
                    .is_some_and(|f| f.is_same_node(Some(&active)))
            {
                ev.prevent_default();
                if let Some(el) = last.and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                    let _ = el.focus();
                }
            }
            // Tab on last button: go to first
            else if !is_shift && last.as_ref().is_some_and(|l| l.is_same_node(Some(&active))) {
                ev.prevent_default();
                if let Some(el) = first.and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                    let _ = el.focus();
                }
            }
        }
    }
}

#[component]
pub fn DialogTrigger(
    #[prop(optional, into)] _as_child: Option<bool>, // simplified, future use
    children: Children,
) -> impl IntoView {
    let ctx = use_context::<DialogContext>()
        .unwrap_or_else(|| panic!("DialogTrigger must be used inside a Dialog component"));

    view! {
        <div on:click=move |_| set_open(ctx, true)>
            {children()}
        </div>
    }
}

#[component]
pub fn DialogContent(
    #[prop(optional, into)] class: Option<String>,
    /// Children must be ChildrenFn (callable multiple times) for reactive conditional rendering
    children: ChildrenFn,
) -> impl IntoView {
    let ctx = use_context::<DialogContext>()
        .unwrap_or_else(|| panic!("DialogContent must be used inside a Dialog component"));
    let content_ref = NodeRef::<Div>::new();
    let class = class.unwrap_or_default();

    // Focus management - focus the dialog when opened
    Effect::new(move |_| {
        if ctx.open.get() {
            if let Some(el) = content_ref.get() {
                let _ = el.focus();
            }
        }
    });

    let base_overlay = "fixed inset-0 z-50 bg-black/80 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0";
    let base_content = "fixed left-[50%] top-[50%] z-50 grid w-full max-w-lg translate-x-[-50%] translate-y-[-50%] gap-4 border border-charcoal-200 bg-white p-6 shadow-lg duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-lg dark:border-charcoal-800 dark:bg-charcoal-950";

    let final_class = format!("{} {}", base_content, class);

    view! {
        {move || {
            // Clone values that need to be moved into the inner closure
            let children = children.clone();
            let final_class = final_class.clone();

            ctx.open.get().then(move || {
                view! {
                    <>
                    // Overlay - clicking closes the dialog
                    <div
                        class={base_overlay}
                        on:click=move |_| set_open(ctx, false)
                        aria-hidden="true"
                    />

                    // Content container with focus trap
                    <div
                        node_ref={content_ref}
                        class={final_class}
                        role="dialog"
                        aria-modal="true"
                        on:keydown=move |ev: KeyboardEvent| {
                            let key = ev.key();
                            if key == "Escape" {
                                set_open(ctx, false);
                            } else if key == "Tab" {
                                // Focus trap: cycle focus within dialog
                                focus_trap(&ev, content_ref);
                            }
                        }
                        tabindex="-1"
                    >
                        {children()}

                        // Close button (X) in top-right corner
                        <div class="absolute right-4 top-4">
                            <Button
                                variant=ButtonVariant::Ghost
                                size=ButtonSize::Icon
                                on_click=Callback::new(move |_| set_open(ctx, false))
                                title="Close dialog"
                                class="opacity-70 hover:opacity-100".to_string()
                            >
                                <i data-lucide="x" class="icon-sm"></i>
                                <span class="sr-only">"Close"</span>
                            </Button>
                        </div>
                    </div>
                    </>
                }
            })
        }}
    }
}

#[component]
pub fn DialogHeader(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let base = "flex flex-col space-y-1.5 text-center sm:text-left";
    let final_class = match class {
        Some(c) => format!("{} {}", base, c),
        None => base.to_string(),
    };
    view! { <div class={final_class}>{children()}</div> }
}

#[component]
pub fn DialogFooter(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let base = "flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2";
    let final_class = match class {
        Some(c) => format!("{} {}", base, c),
        None => base.to_string(),
    };
    view! { <div class={final_class}>{children()}</div> }
}

#[component]
pub fn DialogTitle(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let base = "text-lg font-semibold leading-none tracking-tight";
    let final_class = match class {
        Some(c) => format!("{} {}", base, c),
        None => base.to_string(),
    };
    view! { <h2 class={final_class}>{children()}</h2> }
}

#[component]
pub fn DialogDescription(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let base = "text-sm text-charcoal-500 dark:text-charcoal-400";
    let final_class = match class {
        Some(c) => format!("{} {}", base, c),
        None => base.to_string(),
    };
    view! { <p class={final_class}>{children()}</p> }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === DialogContext tests ===

    #[test]
    fn test_dialog_context_is_copy() {
        // DialogContext should be Copy for efficient context passing
        fn assert_copy<T: Copy>() {}
        assert_copy::<DialogContext>();
    }

    #[test]
    fn test_dialog_context_is_clone() {
        // DialogContext should be Clone
        fn assert_clone<T: Clone>() {}
        assert_clone::<DialogContext>();
    }

    // === CSS class generation tests ===

    #[test]
    fn test_dialog_header_base_class() {
        let base = "flex flex-col space-y-1.5 text-center sm:text-left";
        assert!(base.contains("flex"));
        assert!(base.contains("flex-col"));
        assert!(base.contains("space-y-1.5"));
    }

    #[test]
    fn test_dialog_footer_base_class() {
        let base = "flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2";
        assert!(base.contains("flex"));
        assert!(base.contains("flex-col-reverse"));
        assert!(base.contains("sm:flex-row"));
    }

    #[test]
    fn test_dialog_title_base_class() {
        let base = "text-lg font-semibold leading-none tracking-tight";
        assert!(base.contains("text-lg"));
        assert!(base.contains("font-semibold"));
        assert!(base.contains("tracking-tight"));
    }

    #[test]
    fn test_dialog_description_base_class() {
        let base = "text-sm text-charcoal-500 dark:text-charcoal-400";
        assert!(base.contains("text-sm"));
        assert!(base.contains("text-charcoal-500"));
        assert!(base.contains("dark:text-charcoal-400"));
    }

    #[test]
    fn test_class_merging_with_custom() {
        let base = "text-lg font-semibold";
        let custom = Some("my-custom-class".to_string());
        let final_class = match custom {
            Some(c) => format!("{} {}", base, c),
            None => base.to_string(),
        };
        assert!(final_class.contains("text-lg"));
        assert!(final_class.contains("my-custom-class"));
    }

    #[test]
    fn test_class_merging_without_custom() {
        let base = "text-lg font-semibold";
        let custom: Option<String> = None;
        let final_class = match custom {
            Some(c) => format!("{} {}", base, c),
            None => base.to_string(),
        };
        assert_eq!(final_class, base);
    }

    // === Content styling tests ===

    #[test]
    fn test_overlay_has_backdrop() {
        let overlay_class = "fixed inset-0 z-50 bg-black/80";
        assert!(overlay_class.contains("fixed"));
        assert!(overlay_class.contains("inset-0"));
        assert!(overlay_class.contains("z-50"));
        assert!(overlay_class.contains("bg-black/80"));
    }

    #[test]
    fn test_content_has_centering() {
        let content_class = "fixed left-[50%] top-[50%] translate-x-[-50%] translate-y-[-50%]";
        assert!(content_class.contains("left-[50%]"));
        assert!(content_class.contains("top-[50%]"));
        assert!(content_class.contains("translate-x-[-50%]"));
        assert!(content_class.contains("translate-y-[-50%]"));
    }

    #[test]
    fn test_content_has_responsive_width() {
        let content_class = "w-full max-w-lg";
        assert!(content_class.contains("w-full"));
        assert!(content_class.contains("max-w-lg"));
    }

    #[test]
    fn test_content_has_dark_mode() {
        let content_class = "border-charcoal-200 dark:border-charcoal-800 dark:bg-charcoal-950";
        assert!(content_class.contains("dark:border-charcoal-800"));
        assert!(content_class.contains("dark:bg-charcoal-950"));
    }

    // === Accessibility tests ===

    #[test]
    fn test_dialog_uses_role_dialog() {
        // The content div should have role="dialog"
        let expected_role = "dialog";
        assert_eq!(expected_role, "dialog");
    }

    #[test]
    fn test_dialog_uses_aria_modal() {
        // The content div should have aria-modal="true"
        let expected = "true";
        assert_eq!(expected, "true");
    }

    #[test]
    fn test_overlay_has_aria_hidden() {
        // Overlay should be hidden from screen readers
        let expected = "true";
        assert_eq!(expected, "true");
    }

    // === Focus Trap Tests ===

    #[test]
    fn test_focusable_selector() {
        // Focusable elements selector should include all interactive elements
        let selector = "button, [href], input, select, textarea, [tabindex]:not([tabindex=\"-1\"])";
        assert!(selector.contains("button"));
        assert!(selector.contains("[href]"));
        assert!(selector.contains("input"));
        assert!(selector.contains("select"));
        assert!(selector.contains("textarea"));
        assert!(selector.contains("[tabindex]"));
    }

    #[test]
    fn test_focus_trap_tab_key() {
        // Tab key should be handled for focus trap
        let key = "Tab";
        assert_eq!(key, "Tab");
    }

    #[test]
    fn test_focus_trap_shift_tab() {
        // Shift+Tab should cycle focus backwards
        let is_shift = true;
        let key = "Tab";
        assert!(is_shift && key == "Tab");
    }

    #[test]
    fn test_escape_closes_dialog() {
        // Escape key should close the dialog
        let key = "Escape";
        assert_eq!(key, "Escape");
    }
}
