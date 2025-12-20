//! Button component with CVA-style variants and accessibility.
//!
//! Follows shadcn/ui patterns with focus ring and disabled states.
//! Uses CVA patterns from cva.rs for consistency.

use leptos::prelude::*;

// Re-export CVA types for convenience
pub use super::cva::{BUTTON_BASE, ButtonSize, ButtonVariant, button_class};

/// Button component with variants and accessibility.
///
/// # Props
/// - `variant`: Button style variant (Default, Destructive, Outline, etc.)
/// - `size`: Button size (Sm, Default, Lg, Icon)
/// - `disabled`: Whether button is disabled
/// - `class`: Additional CSS classes
/// - `on_click`: Click handler callback
/// - `children`: Button content
///
/// # Example
/// ```rust,ignore
/// view! {
///     <Button
///         variant=ButtonVariant::Default
///         size=ButtonSize::Default
///         on_click=Callback::new(|_| log::info!("Clicked!"))
///     >
///         "Click me"
///     </Button>
/// }
/// ```
#[component]
pub fn Button(
    /// Button style variant
    #[prop(default = ButtonVariant::Default)]
    variant: ButtonVariant,
    /// Button size
    #[prop(default = ButtonSize::Default)]
    size: ButtonSize,
    /// Whether button is disabled (reactive)
    #[prop(into, default = Signal::derive(|| false))]
    disabled: Signal<bool>,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: Option<String>,
    /// Button type attribute (button, submit, reset)
    #[prop(default = "button".to_string(), into)]
    button_type: String,
    /// Tooltip title for accessibility
    #[prop(optional, into)]
    title: Option<String>,
    /// Accessible label (for icon-only buttons)
    #[prop(optional, into)]
    aria_label: Option<String>,
    /// Whether the controlled element is expanded (for toggles)
    #[prop(optional, into)]
    aria_expanded: Option<Signal<String>>,
    /// ID of the element this button controls
    #[prop(optional, into)]
    aria_controls: Option<String>,
    /// Click handler
    #[prop(optional)]
    on_click: Option<Callback<()>>,
    /// Button content
    children: Children,
) -> impl IntoView {
    // Use CVA function for class merging
    let final_class = button_class(variant, size, class.as_deref());

    view! {
        <button
            type={button_type}
            class={final_class}
            disabled=move || disabled.get()
            title={title}
            aria-label={aria_label}
            aria-expanded=move || aria_expanded.as_ref().map(|s| s.get())
            aria-controls={aria_controls}
            on:click=move |_| {
                if let Some(cb) = on_click.as_ref() {
                    cb.run(());
                }
            }
        >
            {children()}
        </button>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_variant_default() {
        assert!(ButtonVariant::Default.class().contains("bg-primary"));
    }

    #[test]
    fn test_button_variant_destructive() {
        assert!(
            ButtonVariant::Destructive
                .class()
                .contains("bg-destructive")
        );
    }

    #[test]
    fn test_button_variant_outline() {
        assert!(ButtonVariant::Outline.class().contains("border"));
    }

    #[test]
    fn test_button_variant_secondary() {
        assert!(ButtonVariant::Secondary.class().contains("bg-secondary"));
    }

    #[test]
    fn test_button_variant_ghost() {
        assert!(ButtonVariant::Ghost.class().contains("hover:bg-accent"));
    }

    #[test]
    fn test_button_variant_link() {
        assert!(ButtonVariant::Link.class().contains("underline"));
    }

    #[test]
    fn test_button_size_sm() {
        assert!(ButtonSize::Sm.class().contains("h-9"));
    }

    #[test]
    fn test_button_size_default() {
        assert!(ButtonSize::Default.class().contains("h-10"));
    }

    #[test]
    fn test_button_size_lg() {
        assert!(ButtonSize::Lg.class().contains("h-11"));
    }

    #[test]
    fn test_button_size_icon() {
        let classes = ButtonSize::Icon.class();
        assert!(classes.contains("h-10"));
        assert!(classes.contains("w-10"));
    }

    #[test]
    fn test_button_base_has_focus_ring() {
        assert!(BUTTON_BASE.contains("focus-visible:ring-2"));
    }

    #[test]
    fn test_button_base_has_disabled_state() {
        assert!(BUTTON_BASE.contains("disabled:opacity-50"));
        assert!(BUTTON_BASE.contains("disabled:pointer-events-none"));
    }

    #[test]
    fn test_button_class_function() {
        let class = button_class(
            ButtonVariant::Default,
            ButtonSize::Default,
            Some("custom-class"),
        );
        assert!(class.contains("bg-primary"));
        assert!(class.contains("h-10"));
        assert!(class.contains("custom-class"));
    }
}
