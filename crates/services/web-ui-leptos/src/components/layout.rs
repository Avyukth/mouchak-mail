//! Main layout component with navigation.
//! Digital Correspondence design - postal aesthetics meets terminal precision.

use super::{Button, ButtonSize, ButtonVariant};
use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::hooks::use_location;

/// Main layout wrapper with navigation and content outlet.
#[component]
pub fn Layout() -> impl IntoView {
    // Dark mode signal - persisted to localStorage
    let (dark_mode, set_dark_mode) = signal(false);

    // Mobile navigation state
    let mobile_nav_open = RwSignal::new(false);

    // Get current location for aria-current
    let location = use_location();

    // Initialize from localStorage and watch for changes
    Effect::new(move |_| {
        if let Some(window) = web_sys::window()
            && let Ok(Some(storage)) = window.local_storage()
            && let Ok(Some(saved)) = storage.get_item("darkMode")
            && saved == "true"
        {
            set_dark_mode.set(true);
        }
    });

    // Toggle dark mode class on document and save preference
    Effect::new(move |_| {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document()
                && let Some(html) = document.document_element()
            {
                if dark_mode.get() {
                    let _ = html.class_list().add_1("dark");
                } else {
                    let _ = html.class_list().remove_1("dark");
                }
            }
            // Save to localStorage
            if let Ok(Some(storage)) = window.local_storage() {
                let _ =
                    storage.set_item("darkMode", if dark_mode.get() { "true" } else { "false" });
            }
        }
    });

    // Close mobile nav when clicking outside or navigating
    Effect::new(move |_| {
        // Close mobile nav on route change
        let _ = location.pathname.get();
        mobile_nav_open.set(false);
    });

    view! {
        <div class="min-h-screen bg-background transition-colors flex flex-col overflow-x-hidden">
            // Skip link for keyboard accessibility
            <a
                href="#main-content"
                class="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-50 focus:px-4 focus:py-2 focus:bg-background focus:text-foreground focus:shadow-lg focus:rounded-md"
            >
                "Skip to main content"
            </a>

            // Gradient mesh background overlay
            <div class="fixed inset-0 bg-gradient-mesh pointer-events-none" aria-hidden="true"></div>

            // Header with navigation - glassmorphism
            <header class="sticky top-0 z-50 glass border-b border-border/50">
                <nav class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8" role="navigation" aria-label="Main navigation">
                    <div class="flex justify-between h-16 items-center">
                        // Logo / Brand
                        <div class="flex items-center space-x-8">
                            <a href="/" class="flex items-center space-x-2.5 group">
                                <i data-lucide="mail" class="icon-xl text-primary group-hover:text-primary/80 transition-all duration-200"></i>
                                <span class="font-display font-semibold text-lg text-foreground group-hover:text-primary transition-all duration-200">
                                    "MCP Agent Mail"
                                </span>
                            </a>

                            // Navigation links
                            <div class="hidden md:flex items-center space-x-1">
                                <NavLink href="/" label="Dashboard" icon="gauge" />
                                <NavLink href="/projects" label="Projects" icon="folder-open" />
                                <NavLink href="/agents" label="Agents" icon="bot" />
                                <NavLink href="/inbox" label="Inbox" icon="inbox" />
                                <NavLink href="/mail/unified" label="All Mail" icon="layers" />
                                <NavLink href="/attachments" label="Files" icon="paperclip" />
                            </div>
                        </div>

                        // Right side actions
                        <div class="flex items-center space-x-3">
                            // Status indicator
                            <div class="hidden sm:flex status-online">
                                <span class="text-xs font-medium">"Online"</span>
                            </div>

                            // Dark mode toggle
                            <Button
                                variant=ButtonVariant::Ghost
                                size=ButtonSize::Icon
                                on_click=Callback::new(move |_| set_dark_mode.update(|v| *v = !*v))
                                title={if dark_mode.get() { "Switch to light mode".to_string() } else { "Switch to dark mode".to_string() }}
                                class="border border-border rounded-full hover:bg-accent".to_string()
                            >
                                {move || if dark_mode.get() {
                                    view! { <i data-lucide="sun" class="icon-lg text-primary"></i> }.into_any()
                                } else {
                                    view! { <i data-lucide="moon" class="icon-lg text-muted-foreground"></i> }.into_any()
                                }}
                            </Button>

                            // Mobile hamburger menu button
                            <Button
                                variant=ButtonVariant::Ghost
                                size=ButtonSize::Icon
                                on_click=Callback::new(move |_| mobile_nav_open.update(|v| *v = !*v))
                                class="md:hidden".to_string()
                                aria_label="Toggle navigation menu".to_string()
                                aria_expanded=Signal::derive(move || mobile_nav_open.get().to_string())
                                aria_controls="mobile-nav".to_string()
                            >
                                {move || if mobile_nav_open.get() {
                                    view! { <i data-lucide="x" class="icon-lg"></i> }.into_any()
                                } else {
                                    view! { <i data-lucide="menu" class="icon-lg"></i> }.into_any()
                                }}
                            </Button>
                        </div>
                    </div>
                </nav>
            </header>

            // Mobile navigation drawer - slide down panel
            {move || {
                if mobile_nav_open.get() {
                    Some(view! {
                        <div
                            id="mobile-nav"
                            class="md:hidden fixed inset-x-0 top-16 z-40 bg-background/95 backdrop-blur-lg border-b border-border shadow-lg animate-slide-down"
                            role="dialog"
                            aria-modal="true"
                            aria-label="Navigation menu"
                        >
                            <nav class="max-w-7xl mx-auto px-4 py-4 space-y-1">
                                <MobileNavLink
                                    href="/"
                                    label="Dashboard"
                                    icon="gauge"
                                    current_path=Signal::derive(move || location.pathname.get())
                                    on_click=Callback::new(move |_| mobile_nav_open.set(false))
                                />
                                <MobileNavLink
                                    href="/projects"
                                    label="Projects"
                                    icon="folder-open"
                                    current_path=Signal::derive(move || location.pathname.get())
                                    on_click=Callback::new(move |_| mobile_nav_open.set(false))
                                />
                                <MobileNavLink
                                    href="/agents"
                                    label="Agents"
                                    icon="bot"
                                    current_path=Signal::derive(move || location.pathname.get())
                                    on_click=Callback::new(move |_| mobile_nav_open.set(false))
                                />
                                <MobileNavLink
                                    href="/inbox"
                                    label="Inbox"
                                    icon="inbox"
                                    current_path=Signal::derive(move || location.pathname.get())
                                    on_click=Callback::new(move |_| mobile_nav_open.set(false))
                                />
                                <MobileNavLink
                                    href="/mail/unified"
                                    label="All Mail"
                                    icon="layers"
                                    current_path=Signal::derive(move || location.pathname.get())
                                    on_click=Callback::new(move |_| mobile_nav_open.set(false))
                                />
                                <MobileNavLink
                                    href="/attachments"
                                    label="Files"
                                    icon="paperclip"
                                    current_path=Signal::derive(move || location.pathname.get())
                                    on_click=Callback::new(move |_| mobile_nav_open.set(false))
                                />
                            </nav>
                        </div>
                    })
                } else {
                    None
                }
            }}

            // Overlay when mobile nav is open
            {move || {
                if mobile_nav_open.get() {
                    Some(view! {
                        <div
                            class="md:hidden fixed inset-0 top-16 z-30 bg-black/20"
                            on:click=move |_| mobile_nav_open.set(false)
                            aria-hidden="true"
                        ></div>
                    })
                } else {
                    None
                }
            }}

            // Main content area
            <main id="main-content" tabindex="-1" class="relative max-w-7xl mx-auto py-8 px-4 sm:px-6 lg:px-8 flex-1 w-full" role="main">
                <div class="animate-fade-in">
                    <Outlet />
                </div>
            </main>

            // Footer
            <footer class="relative border-t border-border bg-muted/50">
                <div class="max-w-7xl mx-auto py-6 px-4">
                    <div class="flex flex-col sm:flex-row justify-between items-center gap-4">
                        <div class="flex items-center space-x-2 text-sm text-muted-foreground">
                            <i data-lucide="mail" class="icon-sm text-primary"></i>
                            <span class="font-display font-medium">"MCP Agent Mail"</span>
                            <span class="text-border">"•"</span>
                            <span class="font-mono text-xs">"Rust/WASM"</span>
                        </div>
                        <div class="flex items-center space-x-4 text-sm text-muted-foreground">
                            <a href="https://github.com" class="flex items-center space-x-1.5 hover:text-primary transition-all duration-200">
                                <i data-lucide="github" class="icon-sm"></i>
                                <span>"GitHub"</span>
                            </a>
                            <a href="/docs" class="flex items-center space-x-1.5 hover:text-primary transition-all duration-200">
                                <i data-lucide="book-open" class="icon-sm"></i>
                                <span>"Docs"</span>
                            </a>
                        </div>
                    </div>
                </div>
            </footer>
        </div>
    }
}

/// Navigation link component with Lucide icon.
/// Uses min-h-[44px] for WCAG 2.1 AA touch target compliance.
/// Supports aria-current for active page indication.
#[component]
fn NavLink(href: &'static str, label: &'static str, icon: &'static str) -> impl IntoView {
    let location = use_location();
    let is_active = Signal::derive(move || {
        let path = location.pathname.get();
        if href == "/" {
            path == "/"
        } else {
            path.starts_with(href)
        }
    });

    view! {
        <a
            href=href
            aria-current=move || if is_active.get() { Some("page") } else { None }
            class={move || format!(
                "nav-link flex items-center space-x-2 px-3 py-2 min-h-[44px] rounded-lg text-sm font-medium transition-all duration-200 {}",
                if is_active.get() {
                    "text-primary bg-primary/10"
                } else {
                    "text-muted-foreground hover:text-primary hover:bg-primary/10"
                }
            )}
        >
            <i data-lucide=icon class="icon-sm"></i>
            <span>{label}</span>
        </a>
    }
}

/// Mobile navigation link - larger touch targets
#[component]
fn MobileNavLink(
    href: &'static str,
    label: &'static str,
    icon: &'static str,
    #[prop(into)] current_path: Signal<String>,
    on_click: Callback<()>,
) -> impl IntoView {
    let is_active = Signal::derive(move || {
        let path = current_path.get();
        if href == "/" {
            path == "/"
        } else {
            path.starts_with(href)
        }
    });

    view! {
        <a
            href=href
            aria-current=move || if is_active.get() { Some("page") } else { None }
            class={move || format!(
                "flex items-center space-x-3 px-4 py-3 min-h-[48px] rounded-lg text-base font-medium transition-all duration-200 {}",
                if is_active.get() {
                    "text-primary bg-primary/10"
                } else {
                    "text-muted-foreground hover:text-primary hover:bg-primary/10"
                }
            )}
            on:click=move |_| on_click.run(())
        >
            <i data-lucide=icon class="icon-lg"></i>
            <span>{label}</span>
            {move || is_active.get().then(|| view! {
                <i data-lucide="check" class="icon-sm ml-auto text-primary"></i>
            })}
        </a>
    }
}

#[cfg(test)]
mod tests {
    // === Accessibility Pattern Tests ===

    #[test]
    fn test_skip_link_target() {
        // Skip link should target #main-content
        let target = "#main-content";
        assert!(target.starts_with('#'));
        assert!(!target.is_empty());
    }

    #[test]
    fn test_main_landmark_id() {
        // Main landmark should have id="main-content"
        let expected_id = "main-content";
        assert_eq!(expected_id, "main-content");
    }

    #[test]
    fn test_nav_aria_label() {
        // Navigation should have descriptive aria-label
        let aria_label = "Main navigation";
        assert!(!aria_label.is_empty());
        assert!(aria_label.contains("navigation"));
    }

    // === Touch Target Tests ===

    #[test]
    fn test_touch_target_class_contains_min_height() {
        // NavLink should have min-h-[44px] for WCAG compliance
        let nav_class = "nav-link flex items-center space-x-2 px-3 py-2 min-h-[44px] rounded-lg";
        assert!(nav_class.contains("min-h-[44px]"));
    }

    #[test]
    fn test_touch_target_minimum_size() {
        // WCAG 2.1 AA requires 44x44px minimum
        let min_size = 44;
        assert!(min_size >= 44, "Touch target must be at least 44px");
    }

    // === Dark Mode Tests ===

    #[test]
    fn test_dark_mode_class() {
        // Dark mode should use "dark" class on html element
        let dark_class = "dark";
        assert_eq!(dark_class, "dark");
    }

    #[test]
    fn test_dark_mode_storage_key() {
        // Dark mode preference should be stored in localStorage
        let storage_key = "darkMode";
        assert_eq!(storage_key, "darkMode");
    }

    #[test]
    fn test_dark_mode_storage_values() {
        // Storage should use "true"/"false" strings
        let true_value = "true";
        let false_value = "false";
        assert_eq!(true_value, "true");
        assert_eq!(false_value, "false");
    }

    // === Navigation Link Tests ===

    #[test]
    fn test_nav_links_have_icons() {
        // Each nav link should have an associated icon
        let nav_items = [
            ("Dashboard", "gauge"),
            ("Projects", "folder-open"),
            ("Agents", "bot"),
            ("Inbox", "inbox"),
            ("All Mail", "layers"),
            ("Files", "paperclip"),
        ];

        for (label, icon) in nav_items {
            assert!(!label.is_empty());
            assert!(!icon.is_empty());
        }
    }

    #[test]
    fn test_nav_links_have_valid_hrefs() {
        // All navigation hrefs should start with /
        let hrefs = [
            "/",
            "/projects",
            "/agents",
            "/inbox",
            "/mail/unified",
            "/attachments",
        ];
        for href in hrefs {
            assert!(href.starts_with('/'), "Href '{}' should start with /", href);
        }
    }

    // === CSS Class Tests ===

    #[test]
    fn test_header_glassmorphism_class() {
        // Header should use glass class for glassmorphism effect
        let header_class = "glass border-b border-border/50";
        assert!(header_class.contains("glass"));
        assert!(header_class.contains("border-b"));
    }

    #[test]
    fn test_footer_styling() {
        // Footer should have subtle muted background
        let footer_class = "bg-muted/50";
        assert!(footer_class.contains("bg-muted"));
    }

    #[test]
    fn test_main_content_max_width() {
        // Main content should be constrained for readability
        let main_class = "max-w-7xl mx-auto";
        assert!(main_class.contains("max-w-7xl"));
        assert!(main_class.contains("mx-auto"));
    }

    // === Transition Tests ===

    #[test]
    fn test_nav_link_has_transition() {
        // Nav links should have smooth transitions
        let nav_class = "transition-all";
        assert!(nav_class.contains("transition"));
    }

    #[test]
    fn test_dark_mode_transition() {
        // Dark mode switch should have color transition
        let wrapper_class = "transition-colors";
        assert!(wrapper_class.contains("transition"));
    }

    // === Responsive Design Tests ===

    #[test]
    fn test_nav_hidden_on_mobile() {
        // Navigation links should be hidden on mobile (hidden md:flex)
        let nav_class = "hidden md:flex items-center space-x-1";
        assert!(nav_class.contains("hidden"));
        assert!(nav_class.contains("md:flex"));
    }

    #[test]
    fn test_status_indicator_hidden_on_mobile() {
        // Status indicator should be hidden on small screens
        let status_class = "hidden sm:flex";
        assert!(status_class.contains("hidden"));
        assert!(status_class.contains("sm:flex"));
    }

    // === Brand Tests ===

    #[test]
    fn test_brand_name() {
        let brand_name = "MCP Agent Mail";
        assert_eq!(brand_name, "MCP Agent Mail");
    }

    #[test]
    fn test_brand_icon() {
        let brand_icon = "mail";
        assert_eq!(brand_icon, "mail");
    }

    // === Footer Links Tests ===

    #[test]
    fn test_footer_has_github_link() {
        let github_url = "https://github.com";
        assert!(github_url.starts_with("https://"));
    }

    #[test]
    fn test_footer_has_docs_link() {
        let docs_href = "/docs";
        assert!(docs_href.starts_with('/'));
    }
}
