//! Info Banner component for displaying contextual information.

use leptos::prelude::*;

/// Info banner variants for styling
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum BannerVariant {
    /// Blue info banner
    #[default]
    Info,
    /// Green success banner
    Success,
    /// Yellow warning banner
    Warning,
    /// Red error banner
    Error,
}

impl BannerVariant {
    /// Get CSS classes for the variant
    pub fn classes(&self) -> &'static str {
        match self {
            BannerVariant::Info => "bg-sky-50 dark:bg-sky-900/20 border-sky-200 dark:border-sky-800 text-sky-700 dark:text-sky-300",
            BannerVariant::Success => "bg-emerald-50 dark:bg-emerald-900/20 border-emerald-200 dark:border-emerald-800 text-emerald-700 dark:text-emerald-300",
            BannerVariant::Warning => "bg-amber-50 dark:bg-amber-900/20 border-amber-200 dark:border-amber-800 text-amber-700 dark:text-amber-300",
            BannerVariant::Error => "bg-rose-50 dark:bg-rose-900/20 border-rose-200 dark:border-rose-800 text-rose-700 dark:text-rose-300",
        }
    }

    /// Get icon name for the variant
    pub fn icon(&self) -> &'static str {
        match self {
            BannerVariant::Info => "info",
            BannerVariant::Success => "check-circle",
            BannerVariant::Warning => "alert-triangle",
            BannerVariant::Error => "alert-circle",
        }
    }
}

/// Info banner component for displaying contextual information.
///
/// # Example
/// ```rust,ignore
/// view! {
///     <InfoBanner variant=BannerVariant::Info>
///         "This is an informational message."
///     </InfoBanner>
/// }
/// ```
#[component]
pub fn InfoBanner(
    /// Visual variant
    #[prop(default = BannerVariant::Info)]
    variant: BannerVariant,
    /// Banner content
    children: Children,
) -> impl IntoView {
    let classes = variant.classes();
    let icon = variant.icon();

    view! {
        <div class={format!(
            "flex items-start gap-3 p-4 rounded-lg border {}",
            classes
        )}>
            <i data-lucide={icon} class="icon-sm flex-shrink-0 mt-0.5"></i>
            <div class="text-sm">
                {children()}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_banner_variant_classes() {
        assert!(BannerVariant::Info.classes().contains("sky"));
        assert!(BannerVariant::Success.classes().contains("emerald"));
        assert!(BannerVariant::Warning.classes().contains("amber"));
        assert!(BannerVariant::Error.classes().contains("rose"));
    }

    #[test]
    fn test_banner_variant_icons() {
        assert_eq!(BannerVariant::Info.icon(), "info");
        assert_eq!(BannerVariant::Success.icon(), "check-circle");
        assert_eq!(BannerVariant::Warning.icon(), "alert-triangle");
        assert_eq!(BannerVariant::Error.icon(), "alert-circle");
    }
}
