//! Breadcrumb navigation component.

use leptos::prelude::*;

/// A single breadcrumb item
#[derive(Debug, Clone)]
pub struct BreadcrumbItem {
    /// Display label
    pub label: String,
    /// Href (empty string for current/last item)
    pub href: String,
}

impl BreadcrumbItem {
    pub fn new(label: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: href.into(),
        }
    }
}

/// Breadcrumb navigation component.
///
/// # Example
/// ```rust,ignore
/// view! {
///     <Breadcrumb items=vec![
///         BreadcrumbItem::new("Projects", "/projects"),
///         BreadcrumbItem::new("my-project", "/projects/my-project"),
///         BreadcrumbItem::new("File Reservations", ""),
///     ] />
/// }
/// ```
#[component]
pub fn Breadcrumb(
    /// List of breadcrumb items
    items: Vec<BreadcrumbItem>,
) -> impl IntoView {
    let len = items.len();

    view! {
        <nav aria-label="Breadcrumb" class="text-sm">
            <ol class="flex items-center gap-2 text-charcoal-500 dark:text-charcoal-400">
                {items.into_iter().enumerate().map(|(idx, item)| {
                    let is_last = idx == len - 1;
                    let label = item.label.clone();
                    let href = item.href.clone();

                    view! {
                        <li class="flex items-center gap-2">
                            {if is_last {
                                view! {
                                    <span class="font-medium text-charcoal-700 dark:text-cream-200">
                                        {label}
                                    </span>
                                }.into_any()
                            } else {
                                view! {
                                    <>
                                        <a
                                            href={href}
                                            class="hover:text-amber-600 dark:hover:text-amber-400 transition-colors"
                                        >
                                            {label}
                                        </a>
                                        <i data-lucide="chevron-right" class="icon-xs opacity-50"></i>
                                    </>
                                }.into_any()
                            }}
                        </li>
                    }
                }).collect::<Vec<_>>()}
            </ol>
        </nav>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breadcrumb_item_new() {
        let item = BreadcrumbItem::new("Test", "/test");
        assert_eq!(item.label, "Test");
        assert_eq!(item.href, "/test");
    }

    #[test]
    fn test_breadcrumb_item_empty_href() {
        let item = BreadcrumbItem::new("Current", "");
        assert!(item.href.is_empty());
    }
}
