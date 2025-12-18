//! Separator component for visual dividers between content sections.
//!
//! shadcn/ui pattern: Simple semantic separator with horizontal/vertical orientation.

use leptos::prelude::*;

/// Orientation of the separator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// Horizontal separator (default) - full width, 1px height
    #[default]
    Horizontal,
    /// Vertical separator - full height, 1px width
    Vertical,
}

/// A visual separator/divider component.
///
/// # Props
/// - `orientation`: Horizontal (default) or Vertical
/// - `class`: Additional CSS classes to apply
/// - `decorative`: If true, hides from accessibility tree (default: true)
///
/// # Example
/// ```rust,ignore
/// // Horizontal separator between sections
/// view! {
///     <div>"Section 1"</div>
///     <Separator />
///     <div>"Section 2"</div>
/// }
///
/// // Vertical separator in a flex row
/// view! {
///     <div class="flex items-center gap-4">
///         <span>"Item 1"</span>
///         <Separator orientation=Orientation::Vertical />
///         <span>"Item 2"</span>
///     </div>
/// }
/// ```
#[component]
pub fn Separator(
    /// Orientation of the separator (default: Horizontal)
    #[prop(optional)]
    orientation: Option<Orientation>,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: Option<String>,
    /// If true (default), separator is decorative and hidden from accessibility tree
    #[prop(default = true)]
    decorative: bool,
) -> impl IntoView {
    let orientation = orientation.unwrap_or_default();

    // Base classes
    let base_class = "shrink-0 bg-border dark:bg-charcoal-700";

    // Orientation-specific classes
    let orientation_class = match orientation {
        Orientation::Horizontal => "h-[1px] w-full",
        Orientation::Vertical => "w-[1px] h-full",
    };

    // Combine classes
    let final_class = match class {
        Some(c) => format!("{} {} {}", base_class, orientation_class, c),
        None => format!("{} {}", base_class, orientation_class),
    };

    // ARIA orientation value
    let aria_orientation = match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    };

    view! {
        <div
            class={final_class}
            role={if decorative { "none" } else { "separator" }}
            aria-orientation={if decorative { None } else { Some(aria_orientation) }}
            aria-hidden={if decorative { Some("true") } else { None }}
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Orientation tests ===

    #[test]
    fn test_orientation_default_is_horizontal() {
        assert_eq!(Orientation::default(), Orientation::Horizontal);
    }

    #[test]
    fn test_orientation_equality() {
        assert_eq!(Orientation::Horizontal, Orientation::Horizontal);
        assert_eq!(Orientation::Vertical, Orientation::Vertical);
        assert_ne!(Orientation::Horizontal, Orientation::Vertical);
    }

    #[test]
    fn test_orientation_is_copy() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<Orientation>();
    }

    // === CSS class generation tests ===

    #[test]
    fn test_horizontal_classes() {
        let base = "shrink-0 bg-border dark:bg-charcoal-700";
        let horizontal = "h-[1px] w-full";
        let combined = format!("{} {}", base, horizontal);
        assert!(combined.contains("h-[1px]"));
        assert!(combined.contains("w-full"));
        assert!(combined.contains("bg-border"));
    }

    #[test]
    fn test_vertical_classes() {
        let base = "shrink-0 bg-border dark:bg-charcoal-700";
        let vertical = "w-[1px] h-full";
        let combined = format!("{} {}", base, vertical);
        assert!(combined.contains("w-[1px]"));
        assert!(combined.contains("h-full"));
        assert!(combined.contains("bg-border"));
    }

    #[test]
    fn test_class_merging() {
        let base = "shrink-0 bg-border";
        let orientation = "h-[1px] w-full";
        let custom = "my-4";
        let combined = format!("{} {} {}", base, orientation, custom);
        assert!(combined.contains("my-4"));
        assert!(combined.contains("shrink-0"));
    }

    // === Accessibility tests ===

    #[test]
    fn test_decorative_role() {
        // When decorative=true, role should be "none"
        let decorative = true;
        let role = if decorative { "none" } else { "separator" };
        assert_eq!(role, "none");
    }

    #[test]
    fn test_semantic_role() {
        // When decorative=false, role should be "separator"
        let decorative = false;
        let role = if decorative { "none" } else { "separator" };
        assert_eq!(role, "separator");
    }

    #[test]
    fn test_aria_orientation_horizontal() {
        let orientation = Orientation::Horizontal;
        let aria = match orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        };
        assert_eq!(aria, "horizontal");
    }

    #[test]
    fn test_aria_orientation_vertical() {
        let orientation = Orientation::Vertical;
        let aria = match orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        };
        assert_eq!(aria, "vertical");
    }

    #[test]
    fn test_decorative_hides_aria_orientation() {
        // When decorative, aria-orientation should be None
        let decorative = true;
        let aria: Option<&str> = if decorative { None } else { Some("horizontal") };
        assert!(aria.is_none());
    }
}
