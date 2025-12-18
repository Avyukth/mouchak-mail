//! Button component with CVA-style variants and accessibility.
//!
//! Follows shadcn/ui patterns with focus ring and disabled states.

use leptos::prelude::*;

/// Button variant styles
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ButtonVariant {
    /// Primary button (gradient amber)
    #[default]
    Default,
    /// Destructive/danger button (red)
    Destructive,
    /// Outlined button
    Outline,
    /// Secondary button (subtle)
    Secondary,
    /// Ghost button (minimal, no background)
    Ghost,
    /// Link-style button (underline on hover)
    Link,
}

impl ButtonVariant {
    /// Get CSS classes for this variant
    pub fn classes(&self) -> &'static str {
        match self {
            ButtonVariant::Default => {
                "bg-gradient-to-r from-amber-600 to-amber-500 text-white hover:from-amber-700 hover:to-amber-600 shadow-sm"
            }
            ButtonVariant::Destructive => "bg-red-600 text-white hover:bg-red-700",
            ButtonVariant::Outline => {
                "border border-charcoal-300 dark:border-charcoal-600 bg-transparent hover:bg-charcoal-100 dark:hover:bg-charcoal-800"
            }
            ButtonVariant::Secondary => {
                "bg-charcoal-100 dark:bg-charcoal-700 text-charcoal-800 dark:text-charcoal-100 hover:bg-charcoal-200 dark:hover:bg-charcoal-600"
            }
            ButtonVariant::Ghost => {
                "bg-transparent hover:bg-charcoal-100 dark:hover:bg-charcoal-800 text-charcoal-700 dark:text-charcoal-300"
            }
            ButtonVariant::Link => {
                "bg-transparent text-amber-600 dark:text-amber-400 underline-offset-4 hover:underline p-0 h-auto"
            }
        }
    }
}

/// Button size variants
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ButtonSize {
    /// Small button (h-9, 36px)
    Sm,
    /// Default button (h-10, 40px)
    #[default]
    Default,
    /// Large button (h-11, 44px)
    Lg,
    /// Icon-only button (square, h-10 w-10)
    Icon,
}

impl ButtonSize {
    /// Get CSS classes for this size
    pub fn classes(&self) -> &'static str {
        match self {
            ButtonSize::Sm => "h-9 px-3 text-sm rounded-md",
            ButtonSize::Default => "h-10 px-4 py-2 text-sm rounded-md",
            ButtonSize::Lg => "h-11 px-8 text-base rounded-md",
            ButtonSize::Icon => "h-10 w-10 rounded-md",
        }
    }
}

/// Base button classes (always applied)
const BUTTON_BASE: &str = "inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-amber-500 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 gap-2";

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
    /// Whether button is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: Option<String>,
    /// Button type attribute (button, submit, reset)
    #[prop(default = "button".to_string(), into)]
    button_type: String,
    /// Click handler
    #[prop(optional)]
    on_click: Option<Callback<()>>,
    /// Button content
    children: Children,
) -> impl IntoView {
    // Build final class string
    let final_class = format!(
        "{} {} {} {}",
        BUTTON_BASE,
        variant.classes(),
        size.classes(),
        class.unwrap_or_default()
    );

    view! {
        <button
            type={button_type}
            class={final_class}
            disabled={disabled}
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
        assert!(ButtonVariant::Default.classes().contains("amber"));
    }

    #[test]
    fn test_button_variant_destructive() {
        assert!(ButtonVariant::Destructive.classes().contains("red"));
    }

    #[test]
    fn test_button_variant_outline() {
        assert!(ButtonVariant::Outline.classes().contains("border"));
    }

    #[test]
    fn test_button_variant_secondary() {
        assert!(ButtonVariant::Secondary.classes().contains("charcoal"));
    }

    #[test]
    fn test_button_variant_ghost() {
        assert!(ButtonVariant::Ghost.classes().contains("transparent"));
    }

    #[test]
    fn test_button_variant_link() {
        assert!(ButtonVariant::Link.classes().contains("underline"));
    }

    #[test]
    fn test_button_size_sm() {
        assert!(ButtonSize::Sm.classes().contains("h-9"));
    }

    #[test]
    fn test_button_size_default() {
        assert!(ButtonSize::Default.classes().contains("h-10"));
    }

    #[test]
    fn test_button_size_lg() {
        assert!(ButtonSize::Lg.classes().contains("h-11"));
    }

    #[test]
    fn test_button_size_icon() {
        let classes = ButtonSize::Icon.classes();
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
}
