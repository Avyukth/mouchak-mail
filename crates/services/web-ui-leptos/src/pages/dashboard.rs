//! Dashboard page - main landing page with health status and quick stats.
//! Digital Correspondence design - warm, inviting, purposeful.

use leptos::prelude::*;
use crate::api::client::{self, Project};

/// Dashboard page component with health check and project stats.
#[component]
pub fn Dashboard() -> impl IntoView {
    // Use RwSignals for simple state management
    let health_status = RwSignal::new(String::from("checking..."));
    let health_error = RwSignal::new(Option::<String>::None);
    let projects = RwSignal::new(Vec::<Project>::new());
    let projects_loaded = RwSignal::new(false);

    // Load data on mount using spawn_local
    Effect::new(move |_| {
        // Spawn async task to load health
        leptos::task::spawn_local(async move {
            match client::check_health().await {
                Ok(h) => {
                    health_status.set(h.status);
                    health_error.set(None);
                }
                Err(e) => {
                    health_status.set("offline".to_string());
                    health_error.set(Some(e.message));
                }
            }
        });

        // Spawn async task to load projects
        leptos::task::spawn_local(async move {
            match client::get_projects().await {
                Ok(p) => {
                    projects.set(p);
                    projects_loaded.set(true);
                }
                Err(_) => {
                    projects_loaded.set(true);
                }
            }
        });
    });

    view! {
        <div class="space-y-8">
            // Welcome Header
            <div class="relative overflow-hidden rounded-2xl bg-gradient-to-br from-amber-50 via-cream-100 to-teal-50 dark:from-charcoal-800 dark:via-charcoal-800 dark:to-charcoal-700 p-8 border border-cream-200 dark:border-charcoal-600">
                // Decorative elements
                <div class="absolute top-0 right-0 w-64 h-64 bg-gradient-to-br from-amber-200/20 to-transparent dark:from-amber-500/10 rounded-full -translate-y-1/2 translate-x-1/2"></div>
                <div class="absolute bottom-0 left-0 w-48 h-48 bg-gradient-to-tr from-teal-200/20 to-transparent dark:from-teal-500/10 rounded-full translate-y-1/2 -translate-x-1/2"></div>
                
                <div class="relative">
                    <h1 class="font-display text-3xl font-bold text-charcoal-800 dark:text-cream-100 mb-2">
                        "Welcome to MCP Agent Mail"
                    </h1>
                    <p class="text-charcoal-600 dark:text-charcoal-300 max-w-xl">
                        "Your central hub for agent-to-agent communication. Monitor system health, manage projects, and track messages across your agent network."
                    </p>
                </div>
            </div>

            // Status Cards Grid
            <div class="grid grid-cols-1 md:grid-cols-3 gap-6 stagger">
                // Backend Status Card
                <div class="group card-elevated p-6 hover:border-teal-300 dark:hover:border-teal-700 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="p-3 rounded-xl bg-teal-100 dark:bg-teal-900/50 group-hover:scale-105 transition-transform">
                            <i data-lucide="server" class="icon-xl text-teal-600 dark:text-teal-400"></i>
                        </div>
                        <div class={move || {
                            let status = health_status.get();
                            match status.as_str() {
                                "ok" => "flex items-center gap-2 px-3 py-1 rounded-full bg-teal-100 dark:bg-teal-900/50 text-teal-700 dark:text-teal-300 text-sm font-medium",
                                "checking..." => "flex items-center gap-2 px-3 py-1 rounded-full bg-amber-100 dark:bg-amber-900/50 text-amber-700 dark:text-amber-300 text-sm font-medium",
                                _ => "flex items-center gap-2 px-3 py-1 rounded-full bg-red-100 dark:bg-red-900/50 text-red-700 dark:text-red-300 text-sm font-medium",
                            }
                        }}>
                            {move || {
                                let status = health_status.get();
                                match status.as_str() {
                                    "ok" => view! { <i data-lucide="circle-check" class="icon-sm"></i> }.into_any(),
                                    "checking..." => view! { <i data-lucide="loader-2" class="icon-sm animate-spin"></i> }.into_any(),
                                    _ => view! { <i data-lucide="circle-x" class="icon-sm"></i> }.into_any(),
                                }
                            }}
                            <span class="capitalize">{move || health_status.get()}</span>
                        </div>
                    </div>
                    <h3 class="font-display font-semibold text-charcoal-700 dark:text-cream-200 mb-1">"Backend Status"</h3>
                    <p class="text-sm text-charcoal-500 dark:text-charcoal-400">"API server health check"</p>
                </div>

                // Projects Count Card
                <div class="group card-elevated p-6 hover:border-amber-300 dark:hover:border-amber-700 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="p-3 rounded-xl bg-amber-100 dark:bg-amber-900/50 group-hover:scale-105 transition-transform">
                            <i data-lucide="folder-open" class="icon-xl text-amber-600 dark:text-amber-400"></i>
                        </div>
                        <span class="font-display text-3xl font-bold text-amber-600 dark:text-amber-400">
                            {move || {
                                if projects_loaded.get() {
                                    projects.get().len().to_string()
                                } else {
                                    "—".to_string()
                                }
                            }}
                        </span>
                    </div>
                    <h3 class="font-display font-semibold text-charcoal-700 dark:text-cream-200 mb-1">"Active Projects"</h3>
                    <p class="text-sm text-charcoal-500 dark:text-charcoal-400">"Registered agent workspaces"</p>
                </div>

                // Quick Actions Card
                <div class="card-elevated p-6">
                    <div class="flex items-center gap-3 mb-4">
                        <div class="p-3 rounded-xl bg-violet-100 dark:bg-violet-900/50">
                            <i data-lucide="zap" class="icon-xl text-violet-600 dark:text-violet-400"></i>
                        </div>
                        <h3 class="font-display font-semibold text-charcoal-700 dark:text-cream-200">"Quick Actions"</h3>
                    </div>
                    <div class="space-y-3">
                        <a
                            href="/projects"
                            class="group flex items-center justify-between px-4 py-3 rounded-xl bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 hover:bg-amber-100 dark:hover:bg-amber-900/40 hover:border-amber-300 dark:hover:border-amber-700 transition-all"
                        >
                            <div class="flex items-center space-x-2">
                                <i data-lucide="folder" class="icon-sm text-amber-500"></i>
                                <span class="font-medium text-amber-700 dark:text-amber-300">"Browse Projects"</span>
                            </div>
                            <i data-lucide="arrow-right" class="icon-sm text-amber-400 group-hover:translate-x-1 transition-transform"></i>
                        </a>
                        <a
                            href="/inbox"
                            class="group flex items-center justify-between px-4 py-3 rounded-xl bg-cream-100 dark:bg-charcoal-700 border border-cream-200 dark:border-charcoal-600 hover:bg-cream-200 dark:hover:bg-charcoal-600 hover:border-cream-300 dark:hover:border-charcoal-500 transition-all"
                        >
                            <div class="flex items-center space-x-2">
                                <i data-lucide="inbox" class="icon-sm text-charcoal-500 dark:text-charcoal-400"></i>
                                <span class="font-medium text-charcoal-700 dark:text-cream-200">"Check Inbox"</span>
                            </div>
                            <i data-lucide="arrow-right" class="icon-sm text-charcoal-400 group-hover:translate-x-1 transition-transform"></i>
                        </a>
                    </div>
                </div>
            </div>

            // Error display
            {move || {
                health_error.get().map(|e| view! {
                    <div class="rounded-xl border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/20 p-4 animate-slide-up">
                        <div class="flex items-start gap-3">
                            <i data-lucide="triangle-alert" class="icon-lg text-red-500"></i>
                            <div>
                                <p class="font-medium text-red-700 dark:text-red-400">
                                    "Connection Error"
                                </p>
                                <p class="text-sm text-red-600 dark:text-red-500 mt-1">{e}</p>
                                <p class="text-xs text-red-500 dark:text-red-600 mt-2 font-mono">
                                    "Ensure backend is running on port 8765"
                                </p>
                            </div>
                        </div>
                    </div>
                })
            }}

            // Recent Projects List
            {move || {
                let project_list = projects.get();
                if project_list.is_empty() {
                    None
                } else {
                    Some(view! {
                        <div class="card-elevated overflow-hidden">
                            <div class="px-6 py-4 border-b border-cream-200 dark:border-charcoal-700 bg-cream-50/50 dark:bg-charcoal-800/50">
                                <div class="flex items-center justify-between">
                                    <div class="flex items-center space-x-2">
                                        <i data-lucide="history" class="icon-sm text-charcoal-400"></i>
                                        <h2 class="font-display text-lg font-semibold text-charcoal-800 dark:text-cream-100">"Recent Projects"</h2>
                                    </div>
                                    <a href="/projects" class="text-sm font-medium text-amber-600 dark:text-amber-400 hover:text-amber-700 dark:hover:text-amber-300 transition-colors flex items-center space-x-1">
                                        <span>"View all"</span>
                                        <i data-lucide="arrow-right" class="icon-xs"></i>
                                    </a>
                                </div>
                            </div>
                            <ul class="divide-y divide-cream-200 dark:divide-charcoal-700">
                                {project_list.into_iter().take(5).map(|project| {
                                    let href = format!("/projects/{}", project.slug);
                                    let slug = project.slug.clone();
                                    let human_key = project.human_key.clone().unwrap_or_else(|| project.slug.clone());
                                    view! {
                                        <li class="group">
                                            <a href=href class="flex items-center justify-between px-6 py-4 hover:bg-cream-50 dark:hover:bg-charcoal-800/50 transition-colors">
                                                <div class="flex items-center gap-4">
                                                    <div class="w-10 h-10 rounded-lg bg-amber-100 dark:bg-amber-900/50 flex items-center justify-center group-hover:scale-105 transition-transform">
                                                        <i data-lucide="folder" class="icon-lg text-amber-600 dark:text-amber-400"></i>
                                                    </div>
                                                    <div>
                                                        <p class="font-medium text-charcoal-800 dark:text-cream-100">{slug}</p>
                                                        <p class="text-sm text-charcoal-500 dark:text-charcoal-400 font-mono truncate max-w-xs">{human_key}</p>
                                                    </div>
                                                </div>
                                                <i data-lucide="chevron-right" class="icon-sm text-charcoal-300 dark:text-charcoal-600 group-hover:text-amber-500 group-hover:translate-x-1 transition-all"></i>
                                            </a>
                                        </li>
                                    }
                                }).collect::<Vec<_>>()}
                            </ul>
                        </div>
                    })
                }
            }}
        </div>
    }
}
