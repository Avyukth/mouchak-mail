//! Card compound components following shadcn/ui patterns.
//!
//! Provides composable card layout with semantic colors for light/dark mode.

use leptos::prelude::*;

const CARD_BASE: &str = "rounded-lg border border-charcoal-200 dark:border-charcoal-700 bg-white dark:bg-charcoal-800 text-charcoal-900 dark:text-charcoal-50 shadow-sm";
const HEADER_BASE: &str = "flex flex-col space-y-1.5 p-6";
const TITLE_BASE: &str = "text-2xl font-semibold leading-none tracking-tight";
const DESCRIPTION_BASE: &str = "text-sm text-charcoal-500 dark:text-charcoal-400";
const CONTENT_BASE: &str = "p-6 pt-0";
const FOOTER_BASE: &str = "flex items-center p-6 pt-0";

#[component]
pub fn Card(#[prop(optional, into)] class: Option<String>, children: Children) -> impl IntoView {
    let final_class = match class {
        Some(c) => format!("{} {}", CARD_BASE, c),
        None => CARD_BASE.to_string(),
    };

    view! {
        <div class={final_class}>
            {children()}
        </div>
    }
}

#[component]
pub fn CardHeader(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let final_class = match class {
        Some(c) => format!("{} {}", HEADER_BASE, c),
        None => HEADER_BASE.to_string(),
    };

    view! {
        <div class={final_class}>
            {children()}
        </div>
    }
}

#[component]
pub fn CardTitle(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let final_class = match class {
        Some(c) => format!("{} {}", TITLE_BASE, c),
        None => TITLE_BASE.to_string(),
    };

    view! {
        <h3 class={final_class}>
            {children()}
        </h3>
    }
}

#[component]
pub fn CardDescription(
    #[prop(optional, into)] class: Option<String>,
    #[prop(optional, into)] title: Option<String>,
    children: Children,
) -> impl IntoView {
    let final_class = match class {
        Some(c) => format!("{} {}", DESCRIPTION_BASE, c),
        None => DESCRIPTION_BASE.to_string(),
    };

    view! {
        <p class={final_class} title={title}>
            {children()}
        </p>
    }
}

#[component]
pub fn CardContent(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let final_class = match class {
        Some(c) => format!("{} {}", CONTENT_BASE, c),
        None => CONTENT_BASE.to_string(),
    };

    view! {
        <div class={final_class}>
            {children()}
        </div>
    }
}

#[component]
pub fn CardFooter(
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let final_class = match class {
        Some(c) => format!("{} {}", FOOTER_BASE, c),
        None => FOOTER_BASE.to_string(),
    };

    view! {
        <div class={final_class}>
            {children()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_base_has_rounded_border() {
        assert!(CARD_BASE.contains("rounded-lg"));
        assert!(CARD_BASE.contains("border"));
    }

    #[test]
    fn test_card_base_has_semantic_colors() {
        assert!(CARD_BASE.contains("bg-white"));
        assert!(CARD_BASE.contains("dark:bg-charcoal-800"));
    }

    #[test]
    fn test_card_base_has_shadow() {
        assert!(CARD_BASE.contains("shadow-sm"));
    }

    #[test]
    fn test_header_base_has_padding() {
        assert!(HEADER_BASE.contains("p-6"));
    }

    #[test]
    fn test_header_base_has_flex_column() {
        assert!(HEADER_BASE.contains("flex"));
        assert!(HEADER_BASE.contains("flex-col"));
    }

    #[test]
    fn test_title_base_has_typography() {
        assert!(TITLE_BASE.contains("text-2xl"));
        assert!(TITLE_BASE.contains("font-semibold"));
        assert!(TITLE_BASE.contains("tracking-tight"));
    }

    #[test]
    fn test_description_base_has_muted_color() {
        assert!(DESCRIPTION_BASE.contains("text-sm"));
        assert!(DESCRIPTION_BASE.contains("text-charcoal-500"));
        assert!(DESCRIPTION_BASE.contains("dark:text-charcoal-400"));
    }

    #[test]
    fn test_content_base_has_padding() {
        assert!(CONTENT_BASE.contains("p-6"));
        assert!(CONTENT_BASE.contains("pt-0"));
    }

    #[test]
    fn test_footer_base_has_flex_and_padding() {
        assert!(FOOTER_BASE.contains("flex"));
        assert!(FOOTER_BASE.contains("items-center"));
        assert!(FOOTER_BASE.contains("p-6"));
        assert!(FOOTER_BASE.contains("pt-0"));
    }

    #[test]
    fn test_card_dark_mode_support() {
        assert!(CARD_BASE.contains("dark:"));
    }

    #[test]
    fn test_description_dark_mode_support() {
        assert!(DESCRIPTION_BASE.contains("dark:text-charcoal-400"));
    }
}
