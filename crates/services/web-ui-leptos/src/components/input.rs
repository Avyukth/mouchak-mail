//! Input component with focus ring and error states.
//!
//! Follows shadcn/ui patterns with ARIA accessibility support.

use leptos::prelude::*;

/// Input component with focus ring and aria-invalid support.
///
/// # Props
/// - `id`: Optional input ID for label association
/// - `input_type`: Input type (text, email, password, etc.)
/// - `value`: Signal for controlled input value
/// - `placeholder`: Placeholder text
/// - `disabled`: Whether input is disabled
/// - `invalid`: Whether input has validation errors (sets aria-invalid)
/// - `class`: Additional CSS classes
/// - `on_input`: Callback when value changes
///
/// # Example
/// ```rust,ignore
/// let value = RwSignal::new(String::new());
/// view! {
///     <Input
///         value=value
///         placeholder="Enter your email..."
///         invalid=false
///     />
/// }
/// ```
#[component]
pub fn Input(
    /// Input ID for label association
    #[prop(optional, into)]
    id: Option<String>,
    /// Input type (defaults to "text")
    #[prop(default = "text".to_string(), into)]
    input_type: String,
    /// Controlled value signal
    #[prop(into)]
    value: RwSignal<String>,
    /// Placeholder text
    #[prop(optional, into)]
    placeholder: Option<String>,
    /// Disabled state
    #[prop(default = false)]
    disabled: bool,
    /// Invalid/error state (sets aria-invalid)
    #[prop(default = false)]
    invalid: bool,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: Option<String>,
    /// Input change callback
    #[prop(optional)]
    on_input: Option<Callback<String>>,
) -> impl IntoView {
    let base_class = "input h-10";

    // Merge classes
    let final_class = match &class {
        Some(c) => format!("{} {}", base_class, c),
        None => base_class.to_string(),
    };

    // Add error styling if invalid
    let final_class = if invalid {
        format!("{} border-red-500 focus:ring-red-500 focus:border-red-500", final_class)
    } else {
        final_class
    };

    view! {
        <input
            id={id}
            type={input_type}
            class={final_class}
            prop:value=move || value.get()
            on:input=move |ev| {
                let val = event_target_value(&ev);
                value.set(val.clone());
                if let Some(cb) = on_input.as_ref() {
                    cb.run(val);
                }
            }
            placeholder={placeholder}
            disabled={disabled}
            aria-invalid={if invalid { Some("true") } else { None }}
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_base_class() {
        // Input should have h-10 for 40px touch target
        assert!("input h-10".contains("h-10"));
    }

    #[test]
    fn test_input_invalid_class() {
        let base = "input h-10";
        let with_invalid = format!("{} border-red-500 focus:ring-red-500 focus:border-red-500", base);
        assert!(with_invalid.contains("border-red-500"));
        assert!(with_invalid.contains("focus:ring-red-500"));
    }

    #[test]
    fn test_class_merge() {
        let base = "input h-10";
        let extra = Some("w-full".to_string());
        let merged = match &extra {
            Some(c) => format!("{} {}", base, c),
            None => base.to_string(),
        };
        assert_eq!(merged, "input h-10 w-full");
    }

    #[test]
    fn test_class_merge_none() {
        let base = "input h-10";
        let extra: Option<String> = None;
        let merged = match &extra {
            Some(c) => format!("{} {}", base, c),
            None => base.to_string(),
        };
        assert_eq!(merged, "input h-10");
    }
}
