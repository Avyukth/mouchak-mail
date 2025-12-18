//! Main layout component with navigation.
//! Digital Correspondence design - postal aesthetics meets terminal precision.

use super::{Button, ButtonSize, ButtonVariant};
use leptos::prelude::*;
use leptos_router::components::Outlet;

/// Main layout wrapper with navigation and content outlet.
#[component]
pub fn Layout() -> impl IntoView {
    // Dark mode signal - persisted to localStorage
    let (dark_mode, set_dark_mode) = signal(false);

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

    view! {
        <div class="min-h-screen bg-cream-100 dark:bg-charcoal-900 transition-colors flex flex-col">
            // Skip link for keyboard accessibility
            <a
                href="#main-content"
                class="skip-link"
            >
                "Skip to main content"
            </a>

            // Gradient mesh background overlay
            <div class="fixed inset-0 bg-gradient-mesh pointer-events-none" aria-hidden="true"></div>

            // Navigation header with glassmorphism
            <nav class="sticky top-0 z-50 glass border-b border-cream-300/50 dark:border-charcoal-700/50" role="navigation" aria-label="Main navigation">
                <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="flex justify-between h-16 items-center">
                        // Logo / Brand
                        <div class="flex items-center space-x-8">
                            <a href="/" class="flex items-center space-x-2.5 group">
                                <i data-lucide="mail" class="icon-xl text-amber-500 group-hover:text-amber-600 transition-colors"></i>
                                <span class="font-display font-semibold text-lg text-charcoal-800 dark:text-cream-100 group-hover:text-amber-600 dark:group-hover:text-amber-400 transition-colors">
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
                            </div>
                        </div>

                        // Right side actions
                        <div class="flex items-center space-x-3">
                            // Status indicator
                            <div class="hidden sm:flex items-center space-x-2 px-3 py-1.5 rounded-full bg-teal-100/50 dark:bg-teal-900/30 border border-teal-200 dark:border-teal-800">
                                <span class="w-2 h-2 rounded-full bg-teal-500 animate-pulse-gentle"></span>
                                <span class="text-xs font-medium text-teal-700 dark:text-teal-300">"Online"</span>
                            </div>

                            // Dark mode toggle
                            <Button
                                variant=ButtonVariant::Ghost
                                size=ButtonSize::Icon
                                on_click=Callback::new(move |_| set_dark_mode.update(|v| *v = !*v))
                                title={if dark_mode.get() { "Switch to light mode".to_string() } else { "Switch to dark mode".to_string() }}
                                class="border border-cream-300 dark:border-charcoal-600".to_string()
                            >
                                {move || if dark_mode.get() {
                                    view! { <i data-lucide="sun" class="icon-lg text-amber-500"></i> }.into_any()
                                } else {
                                    view! { <i data-lucide="moon" class="icon-lg text-charcoal-500"></i> }.into_any()
                                }}
                            </Button>
                        </div>
                    </div>
                </div>
            </nav>

            // Mobile navigation drawer (future enhancement)

            // Main content area
            <main id="main-content" tabindex="-1" class="relative max-w-7xl mx-auto py-8 px-4 sm:px-6 lg:px-8 flex-1 w-full" role="main">
                <div class="animate-fade-in">
                    <Outlet />
                </div>
            </main>

            // Footer
            <footer class="relative border-t border-cream-300 dark:border-charcoal-700 bg-cream-50/50 dark:bg-charcoal-800/50">
                <div class="max-w-7xl mx-auto py-6 px-4">
                    <div class="flex flex-col sm:flex-row justify-between items-center gap-4">
                        <div class="flex items-center space-x-2 text-sm text-charcoal-500 dark:text-charcoal-400">
                            <i data-lucide="mail" class="icon-sm text-amber-500"></i>
                            <span class="font-display font-medium">"MCP Agent Mail"</span>
                            <span class="text-cream-400 dark:text-charcoal-600">"•"</span>
                            <span class="font-mono text-xs">"Rust/WASM"</span>
                        </div>
                        <div class="flex items-center space-x-4 text-sm text-charcoal-400 dark:text-charcoal-500">
                            <a href="https://github.com" class="flex items-center space-x-1.5 hover:text-amber-600 dark:hover:text-amber-400 transition-colors">
                                <i data-lucide="github" class="icon-sm"></i>
                                <span>"GitHub"</span>
                            </a>
                            <a href="/docs" class="flex items-center space-x-1.5 hover:text-amber-600 dark:hover:text-amber-400 transition-colors">
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
#[component]
fn NavLink(href: &'static str, label: &'static str, icon: &'static str) -> impl IntoView {
    view! {
        <a
            href=href
            class="nav-link flex items-center space-x-2 px-3 py-2 rounded-lg text-sm font-medium text-charcoal-600 dark:text-charcoal-300 hover:text-amber-600 dark:hover:text-amber-400 hover:bg-amber-50 dark:hover:bg-amber-900/20 transition-all"
        >
            <i data-lucide=icon class="icon-sm"></i>
            <span>{label}</span>
        </a>
    }
}
