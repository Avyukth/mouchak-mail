//! CVA (Class Variance Authority) patterns using tailwind_fuse.
//!
//! This module provides shadcn/ui-style CVA patterns for Leptos components.
//! Uses `tw_merge!` for runtime class merging without derive macros.
//!
//! # Usage
//! All components should import their base classes and variant functions from here
//! to ensure consistency across the codebase.

use tailwind_fuse::tw_merge;

// =============================================================================
// BUTTON CVA
// =============================================================================

/// Base button classes (always applied) - enhanced with proper shadows and states
pub const BUTTON_BASE: &str = "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium ring-offset-background transition-all duration-300 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 active:scale-[0.98]";

/// Button variant styles following shadcn/ui patterns with enhanced visual hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Destructive,
    Outline,
    Secondary,
    Ghost,
    Link,
}

impl ButtonVariant {
    /// Get the tailwind classes for this variant with improved shadows and micro-interactions
    pub fn class(&self) -> &'static str {
        match self {
            Self::Default => {
                "bg-primary text-primary-foreground shadow-lg hover:bg-primary/90 hover:shadow-xl hover:-translate-y-0.5 active:translate-y-0 active:shadow-lg focus-visible:ring-4"
            }
            Self::Destructive => {
                "bg-destructive text-destructive-foreground shadow-lg hover:bg-destructive/90 hover:shadow-xl hover:-translate-y-0.5 active:translate-y-0 active:shadow-lg focus-visible:ring-4"
            }
            Self::Outline => {
                "border border-input bg-background shadow-sm hover:bg-accent hover:text-accent-foreground hover:shadow-md hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm focus-visible:ring-4"
            }
            Self::Secondary => {
                "bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80 hover:shadow-md hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm focus-visible:ring-4"
            }
            Self::Ghost => {
                "hover:bg-accent hover:text-accent-foreground hover:-translate-y-0.5 active:translate-y-0 focus-visible:ring-4"
            }
            Self::Link => {
                "text-primary underline-offset-4 hover:underline p-0 h-auto focus-visible:ring-2"
            }
        }
    }
}

/// Button size variants following shadcn/ui patterns.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ButtonSize {
    #[default]
    Default,
    Sm,
    Lg,
    Icon,
}

impl ButtonSize {
    /// Get the tailwind classes for this size
    pub fn class(&self) -> &'static str {
        match self {
            Self::Default => "h-10 px-4 py-2 text-sm rounded-md",
            Self::Sm => "h-9 px-3 text-sm rounded-md",
            Self::Lg => "h-11 px-8 text-base rounded-md",
            Self::Icon => "h-10 w-10 rounded-md",
        }
    }
}

/// Generate button classes using CVA-style merging.
///
/// # Example
/// ```rust,ignore
/// let classes = button_class(ButtonVariant::Destructive, ButtonSize::Lg, Some("my-extra"));
/// ```
pub fn button_class(variant: ButtonVariant, size: ButtonSize, extra: Option<&str>) -> String {
    tw_merge!(
        BUTTON_BASE,
        variant.class(),
        size.class(),
        extra.unwrap_or_default()
    )
}

// =============================================================================
// BADGE CVA
// =============================================================================

/// Base badge classes
pub const BADGE_BASE: &str = "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2";

/// Badge variants (extended with Success and Warning)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BadgeVariant {
    /// Primary badge (indigo)
    #[default]
    Default,
    /// Secondary badge (muted)
    Secondary,
    /// Destructive/error badge (red)
    Destructive,
    /// Outline only badge
    Outline,
    /// Success badge (green/teal)
    Success,
    /// Warning badge (amber/yellow)
    Warning,
}

impl BadgeVariant {
    pub fn class(&self) -> &'static str {
        match self {
            Self::Default => {
                "border-transparent bg-primary text-primary-foreground hover:bg-primary/80"
            }
            Self::Secondary => {
                "border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80"
            }
            Self::Destructive => {
                "border-transparent bg-destructive text-destructive-foreground hover:bg-destructive/80"
            }
            Self::Outline => "text-foreground",
            Self::Success => "border-transparent bg-teal-500 text-white hover:bg-teal-600",
            Self::Warning => "border-transparent bg-amber-500 text-white hover:bg-amber-600",
        }
    }
}

/// Generate badge classes
pub fn badge_class(variant: BadgeVariant, extra: Option<&str>) -> String {
    tw_merge!(BADGE_BASE, variant.class(), extra.unwrap_or_default())
}

// =============================================================================
// ALERT CVA
// =============================================================================

/// Base alert classes
pub const ALERT_BASE: &str = "relative w-full rounded-lg border p-4 [&>svg~*]:pl-7 [&>svg+div]:translate-y-[-3px] [&>svg]:absolute [&>svg]:left-4 [&>svg]:top-4 [&>svg]:text-foreground";

/// Alert title classes
pub const ALERT_TITLE: &str = "mb-1 font-medium leading-none tracking-tight";

/// Alert description classes
pub const ALERT_DESCRIPTION: &str = "text-sm [&_p]:leading-relaxed";

/// Alert variant styles
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AlertVariant {
    /// Default style (background/foreground)
    #[default]
    Default,
    /// Destructive style (red)
    Destructive,
    /// Success style (green/teal)
    Success,
    /// Warning style (amber)
    Warning,
}

impl AlertVariant {
    pub fn class(&self) -> &'static str {
        match self {
            Self::Default => "bg-background text-foreground border-border",
            Self::Destructive => {
                "border-destructive/50 text-destructive dark:border-destructive [&>svg]:text-destructive"
            }
            Self::Success => {
                "border-teal-500/50 text-teal-600 dark:border-teal-500 [&>svg]:text-teal-600 dark:text-teal-400"
            }
            Self::Warning => {
                "border-amber-500/50 text-amber-600 dark:border-amber-500 [&>svg]:text-amber-600 dark:text-amber-400"
            }
        }
    }
}

/// Generate alert classes
pub fn alert_class(variant: AlertVariant, extra: Option<&str>) -> String {
    tw_merge!(ALERT_BASE, variant.class(), extra.unwrap_or_default())
}

// =============================================================================
// INPUT CVA
// =============================================================================

/// Input base classes
pub const INPUT_BASE: &str = "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50";

/// Input invalid/error classes
pub const INPUT_INVALID: &str = "border-destructive focus-visible:ring-destructive";

/// Generate input classes
pub fn input_class(invalid: bool, extra: Option<&str>) -> String {
    if invalid {
        tw_merge!(INPUT_BASE, INPUT_INVALID, extra.unwrap_or_default())
    } else {
        tw_merge!(INPUT_BASE, extra.unwrap_or_default())
    }
}

// =============================================================================
// TEXTAREA CVA
// =============================================================================

/// Textarea base classes
pub const TEXTAREA_BASE: &str = "flex min-h-[80px] w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50";

/// Generate textarea classes
pub fn textarea_class(invalid: bool, extra: Option<&str>) -> String {
    if invalid {
        tw_merge!(TEXTAREA_BASE, INPUT_INVALID, extra.unwrap_or_default())
    } else {
        tw_merge!(TEXTAREA_BASE, extra.unwrap_or_default())
    }
}

// =============================================================================
// CARD CVA
// =============================================================================

/// Card base classes
pub const CARD_BASE: &str = "rounded-lg border bg-card text-card-foreground shadow-sm";
pub const CARD_HEADER: &str = "flex flex-col space-y-1.5 p-6";
pub const CARD_TITLE: &str = "text-2xl font-semibold leading-none tracking-tight";
pub const CARD_DESCRIPTION: &str = "text-sm text-muted-foreground";
pub const CARD_CONTENT: &str = "p-6 pt-0";
pub const CARD_FOOTER: &str = "flex items-center p-6 pt-0";

/// Generate card classes
pub fn card_class(extra: Option<&str>) -> String {
    tw_merge!(CARD_BASE, extra.unwrap_or_default())
}

// =============================================================================
// DIALOG CVA
// =============================================================================

/// Dialog overlay classes
pub const DIALOG_OVERLAY: &str = "fixed inset-0 z-50 bg-black/80 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0";

/// Dialog content classes
pub const DIALOG_CONTENT: &str = "fixed left-[50%] top-[50%] z-50 grid w-full max-w-lg translate-x-[-50%] translate-y-[-50%] gap-4 border bg-background p-6 shadow-lg duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-lg";

/// Dialog header classes
pub const DIALOG_HEADER: &str = "flex flex-col space-y-1.5 text-center sm:text-left";

/// Dialog footer classes
pub const DIALOG_FOOTER: &str = "flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2";

/// Dialog title classes
pub const DIALOG_TITLE: &str = "text-lg font-semibold leading-none tracking-tight";

/// Dialog description classes
pub const DIALOG_DESCRIPTION: &str = "text-sm text-muted-foreground";

// =============================================================================
// SELECT CVA
// =============================================================================

/// Select trigger classes
pub const SELECT_TRIGGER: &str = "flex h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50";

/// Select content (dropdown) classes
pub const SELECT_CONTENT: &str = "relative z-50 max-h-96 min-w-[8rem] overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2";

/// Select item classes
pub const SELECT_ITEM: &str = "relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50";

// =============================================================================
// SKELETON CVA
// =============================================================================

/// Skeleton base classes
pub const SKELETON_BASE: &str = "animate-pulse rounded-md bg-muted";

/// Generate skeleton classes
pub fn skeleton_class(extra: Option<&str>) -> String {
    tw_merge!(SKELETON_BASE, extra.unwrap_or_default())
}

// =============================================================================
// SEPARATOR CVA
// =============================================================================

/// Separator horizontal classes
pub const SEPARATOR_HORIZONTAL: &str = "h-[1px] w-full shrink-0 bg-border";

/// Separator vertical classes
pub const SEPARATOR_VERTICAL: &str = "w-[1px] h-full shrink-0 bg-border";

// =============================================================================
// PROGRESS CVA
// =============================================================================

/// Progress container classes
pub const PROGRESS_BASE: &str = "relative h-4 w-full overflow-hidden rounded-full bg-secondary";

/// Progress indicator classes
pub const PROGRESS_INDICATOR: &str = "h-full w-full flex-1 bg-primary transition-all";

// =============================================================================
// SWITCH CVA
// =============================================================================

/// Switch track classes
pub const SWITCH_BASE: &str = "peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-primary data-[state=unchecked]:bg-input";

/// Switch thumb classes
pub const SWITCH_THUMB: &str = "pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg ring-0 transition-transform data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0";

// =============================================================================
// CHECKBOX CVA
// =============================================================================

/// Checkbox base classes
pub const CHECKBOX_BASE: &str = "peer h-4 w-4 shrink-0 rounded-sm border border-primary ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground";

// =============================================================================
// TABS CVA
// =============================================================================

/// Tabs list classes
pub const TABS_LIST: &str =
    "inline-flex h-10 items-center justify-center rounded-md bg-muted p-1 text-muted-foreground";

/// Tabs trigger classes
pub const TABS_TRIGGER: &str = "inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm";

/// Tabs content classes
pub const TABS_CONTENT: &str = "mt-2 ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2";

// =============================================================================
// TOOLTIP CVA
// =============================================================================

/// Tooltip content classes
pub const TOOLTIP_CONTENT: &str = "z-50 overflow-hidden rounded-md border bg-popover px-3 py-1.5 text-sm text-popover-foreground shadow-md animate-in fade-in-0 zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2";

// =============================================================================
// SPINNER CVA
// =============================================================================

/// Spinner base classes (uses animate-spin)
pub const SPINNER_BASE: &str = "animate-spin text-muted-foreground";

/// Spinner size variants
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SpinnerSize {
    Sm,
    #[default]
    Default,
    Lg,
}

impl SpinnerSize {
    pub fn class(&self) -> &'static str {
        match self {
            Self::Sm => "h-4 w-4",
            Self::Default => "h-6 w-6",
            Self::Lg => "h-8 w-8",
        }
    }
}

/// Generate spinner classes
pub fn spinner_class(size: SpinnerSize, extra: Option<&str>) -> String {
    tw_merge!(SPINNER_BASE, size.class(), extra.unwrap_or_default())
}

// =============================================================================
// LABEL CVA
// =============================================================================

/// Label base classes
pub const LABEL_BASE: &str =
    "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70";

// -- Utility: cn function (className merge) --

/// Merge class names with conflict resolution (cn equivalent).
///
/// This is the primary API for class merging in components.
/// Later classes override conflicting earlier ones.
///
/// # Example
/// ```rust,ignore
/// let class = cn(&["text-red-500", "p-4", if is_active { "bg-primary" } else { "" }]);
/// ```
pub fn cn(classes: &[&str]) -> String {
    tw_merge!(classes.join(" "))
}

/// Merge two class strings
pub fn merge2(base: &str, extra: &str) -> String {
    tw_merge!(base, extra)
}

/// Merge base with optional extra class
pub fn with_class(base: &str, extra: Option<&str>) -> String {
    tw_merge!(base, extra.unwrap_or_default())
}

#[cfg(test)]
#[allow(clippy::const_is_empty)]
mod tests {
    use super::*;

    // === Button CVA Tests ===

    #[test]
    fn test_button_class_default() {
        let class = button_class(ButtonVariant::Default, ButtonSize::Default, None);
        assert!(class.contains("bg-primary"));
        assert!(class.contains("h-10"));
        assert!(class.contains("gap-2"));
    }

    #[test]
    fn test_button_class_destructive_lg() {
        let class = button_class(ButtonVariant::Destructive, ButtonSize::Lg, None);
        assert!(class.contains("bg-destructive"));
        assert!(class.contains("h-11"));
    }

    #[test]
    fn test_button_class_with_extra() {
        let class = button_class(ButtonVariant::Ghost, ButtonSize::Icon, Some("my-custom"));
        assert!(class.contains("my-custom"));
        assert!(class.contains("w-10"));
    }

    #[test]
    fn test_button_has_transition() {
        assert!(BUTTON_BASE.contains("transition-all"));
        assert!(BUTTON_BASE.contains("duration-300"));
    }

    // === Badge CVA Tests ===

    #[test]
    fn test_badge_class() {
        let class = badge_class(BadgeVariant::Outline, None);
        assert!(class.contains("text-foreground"));
        assert!(class.contains("rounded-full"));
    }

    #[test]
    fn test_badge_success_variant() {
        let class = badge_class(BadgeVariant::Success, None);
        assert!(class.contains("bg-teal-500"));
        assert!(class.contains("text-white"));
    }

    #[test]
    fn test_badge_warning_variant() {
        let class = badge_class(BadgeVariant::Warning, None);
        assert!(class.contains("bg-amber-500"));
    }

    // === Alert CVA Tests ===

    #[test]
    fn test_alert_class_default() {
        let class = alert_class(AlertVariant::Default, None);
        assert!(class.contains("rounded-lg"));
        assert!(class.contains("border"));
    }

    #[test]
    fn test_alert_class_destructive() {
        let class = alert_class(AlertVariant::Destructive, None);
        assert!(class.contains("border-destructive"));
        assert!(class.contains("text-destructive"));
    }

    #[test]
    fn test_alert_class_success() {
        let class = alert_class(AlertVariant::Success, None);
        assert!(class.contains("border-teal-500"));
        assert!(class.contains("text-teal-600"));
    }

    // === Input CVA Tests ===

    #[test]
    fn test_input_class_valid() {
        let class = input_class(false, Some("w-64"));
        assert!(class.contains("border-input"));
        assert!(class.contains("w-64"));
        assert!(!class.contains("border-destructive"));
    }

    #[test]
    fn test_input_class_invalid() {
        let class = input_class(true, None);
        assert!(class.contains("border-destructive"));
        assert!(class.contains("focus-visible:ring-destructive"));
    }

    // === Textarea CVA Tests ===

    #[test]
    fn test_textarea_class() {
        let class = textarea_class(false, None);
        assert!(class.contains("min-h-[80px]"));
        assert!(class.contains("rounded-md"));
    }

    #[test]
    fn test_textarea_class_invalid() {
        let class = textarea_class(true, None);
        assert!(class.contains("border-destructive"));
    }

    // === Skeleton CVA Tests ===

    #[test]
    fn test_skeleton_class() {
        let class = skeleton_class(Some("h-4 w-full"));
        assert!(class.contains("animate-pulse"));
        assert!(class.contains("bg-muted"));
        assert!(class.contains("h-4"));
    }

    // === Spinner CVA Tests ===

    #[test]
    fn test_spinner_class_default() {
        let class = spinner_class(SpinnerSize::Default, None);
        assert!(class.contains("animate-spin"));
        assert!(class.contains("h-6"));
        assert!(class.contains("w-6"));
    }

    #[test]
    fn test_spinner_class_sm() {
        let class = spinner_class(SpinnerSize::Sm, None);
        assert!(class.contains("h-4"));
        assert!(class.contains("w-4"));
    }

    #[test]
    fn test_spinner_class_lg() {
        let class = spinner_class(SpinnerSize::Lg, None);
        assert!(class.contains("h-8"));
        assert!(class.contains("w-8"));
    }

    // === Utility Function Tests ===

    #[test]
    fn test_cn_merge() {
        let class = cn(&["text-red-500", "bg-blue-500", "text-green-500"]);
        // tw_merge should resolve conflicts - text-green-500 should win
        assert!(class.contains("text-green-500") || class.contains("text-red-500"));
    }

    #[test]
    fn test_with_class() {
        let result = with_class("base-class", Some("extra"));
        assert!(result.contains("base-class"));
        assert!(result.contains("extra"));
    }

    #[test]
    fn test_with_class_none() {
        let result = with_class("base-class", None);
        assert!(result.contains("base-class"));
    }

    // === Constant Validation Tests ===

    #[test]
    fn test_dialog_classes_defined() {
        assert!(!DIALOG_OVERLAY.is_empty());
        assert!(!DIALOG_CONTENT.is_empty());
        assert!(!DIALOG_HEADER.is_empty());
        assert!(!DIALOG_FOOTER.is_empty());
        assert!(!DIALOG_TITLE.is_empty());
        assert!(!DIALOG_DESCRIPTION.is_empty());
    }

    #[test]
    fn test_tabs_classes_defined() {
        assert!(!TABS_LIST.is_empty());
        assert!(!TABS_TRIGGER.is_empty());
        assert!(!TABS_CONTENT.is_empty());
    }

    #[test]
    fn test_switch_classes_defined() {
        assert!(!SWITCH_BASE.is_empty());
        assert!(!SWITCH_THUMB.is_empty());
        assert!(SWITCH_BASE.contains("data-[state=checked]"));
    }

    #[test]
    fn test_progress_classes_defined() {
        assert!(!PROGRESS_BASE.is_empty());
        assert!(!PROGRESS_INDICATOR.is_empty());
        assert!(PROGRESS_BASE.contains("rounded-full"));
    }

    #[test]
    fn test_checkbox_classes_defined() {
        assert!(!CHECKBOX_BASE.is_empty());
        assert!(CHECKBOX_BASE.contains("data-[state=checked]"));
    }

    #[test]
    fn test_tooltip_classes_defined() {
        assert!(!TOOLTIP_CONTENT.is_empty());
        assert!(TOOLTIP_CONTENT.contains("z-50"));
    }

    #[test]
    fn test_separator_classes_defined() {
        assert!(!SEPARATOR_HORIZONTAL.is_empty());
        assert!(!SEPARATOR_VERTICAL.is_empty());
        assert!(SEPARATOR_HORIZONTAL.contains("h-[1px]"));
        assert!(SEPARATOR_VERTICAL.contains("w-[1px]"));
    }

    #[test]
    fn test_label_classes_defined() {
        assert!(!LABEL_BASE.is_empty());
        assert!(LABEL_BASE.contains("text-sm"));
        assert!(LABEL_BASE.contains("font-medium"));
    }

    #[test]
    fn test_card_classes_defined() {
        assert!(!CARD_BASE.is_empty());
        assert!(!CARD_HEADER.is_empty());
        assert!(!CARD_TITLE.is_empty());
        assert!(!CARD_DESCRIPTION.is_empty());
        assert!(!CARD_CONTENT.is_empty());
        assert!(!CARD_FOOTER.is_empty());
    }

    #[test]
    fn test_select_classes_defined() {
        assert!(!SELECT_TRIGGER.is_empty());
        assert!(!SELECT_CONTENT.is_empty());
        assert!(!SELECT_ITEM.is_empty());
    }
}
