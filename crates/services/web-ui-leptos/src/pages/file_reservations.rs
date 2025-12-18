//! File Reservations page - view and manage file reservations in a project.

use crate::api::client;
use crate::components::{AgentAvatar, Breadcrumb, BreadcrumbItem, BannerVariant, InfoBanner};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

/// File reservation data
#[derive(Debug, Clone)]
pub struct FileReservation {
    pub id: i64,
    pub agent_name: String,
    pub path_pattern: String,
    pub exclusive: bool,
    pub reason: Option<String>,
    pub created_ts: String,
    pub expires_ts: Option<String>,
    pub expired: bool,
}

/// File Reservations page component.
#[component]
pub fn FileReservations() -> impl IntoView {
    let params = use_params_map();
    let project_slug = params.with_untracked(|p| p.get("slug").unwrap_or_default());
    let project_slug_for_fetch = project_slug.clone();
    let project_slug_for_breadcrumb = project_slug.clone();
    let project_slug_for_display = project_slug.clone();

    // State
    let reservations = RwSignal::new(Vec::<FileReservation>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);

    // Fetch reservations on mount
    Effect::new(move |_| {
        let slug = project_slug_for_fetch.clone();
        leptos::task::spawn_local(async move {
            loading.set(true);
            match client::get_file_reservations(&slug).await {
                Ok(res) => {
                    // Map API response to our struct
                    let mapped: Vec<FileReservation> = res
                        .into_iter()
                        .map(|r| FileReservation {
                            id: r.id,
                            agent_name: r.agent_name,
                            path_pattern: r.path_pattern,
                            exclusive: r.exclusive,
                            reason: r.reason,
                            created_ts: r.created_ts,
                            expires_ts: r.expires_ts,
                            expired: r.expired,
                        })
                        .collect();
                    reservations.set(mapped);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.message));
                    loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="space-y-6">
            // Breadcrumb
            <Breadcrumb items={vec![
                BreadcrumbItem::new("Projects", "/projects"),
                BreadcrumbItem::new(project_slug_for_breadcrumb.clone(), format!("/projects/{}", project_slug_for_breadcrumb)),
                BreadcrumbItem::new("File Reservations", ""),
            ]} />

            // Page Header
            <div class="flex items-center gap-3">
                <div class="p-3 bg-amber-100 dark:bg-amber-900/30 rounded-xl">
                    <i data-lucide="shield" class="icon-lg text-amber-600"></i>
                </div>
                <div>
                    <h1 class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100">
                        "File Reservations"
                    </h1>
                    <p class="text-charcoal-500 dark:text-charcoal-400 text-sm">
                        "When agents want to edit files, they can \"reserve\" them to signal their intent."
                    </p>
                </div>
            </div>

            // Info Banner
            <InfoBanner variant=BannerVariant::Info>
                <p>
                    <strong>"Advisory system: "</strong>
                    "Reservations are "
                    <em>"signals"</em>
                    ", not hard locks. Agents can still edit files, but they'll see warnings if conflicts exist."
                </p>
                <p class="mt-2">
                    "Install a "
                    <a href={format!("/projects/{}", project_slug_for_display)} class="text-sky-600 dark:text-sky-400 underline hover:no-underline">
                        "pre-commit hook"
                    </a>
                    " to enforce reservations at commit time."
                </p>
            </InfoBanner>

            // Loading/Error/Content
            {move || {
                if loading.get() {
                    view! {
                        <div class="flex items-center justify-center py-12">
                            <i data-lucide="loader-2" class="icon-xl text-amber-500 animate-spin"></i>
                        </div>
                    }.into_any()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="card-elevated p-6 text-center text-rose-600 dark:text-rose-400">
                            <i data-lucide="alert-circle" class="icon-lg mx-auto mb-2"></i>
                            <p>{err}</p>
                        </div>
                    }.into_any()
                } else {
                    let res = reservations.get();
                    let count = res.len();
                    view! {
                        <div class="space-y-4">
                            <p class="text-sm text-charcoal-500 dark:text-charcoal-400">
                                {format!("{} active reservation{}", count, if count == 1 { "" } else { "s" })}
                            </p>

                            {if res.is_empty() {
                                view! {
                                    <div class="card-elevated p-8 text-center text-charcoal-400">
                                        <i data-lucide="file-check" class="icon-xl mx-auto mb-3 opacity-50"></i>
                                        <p>"No active file reservations"</p>
                                        <p class="text-sm mt-1">
                                            "Agents can reserve files using the reserve_file tool"
                                        </p>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="card-elevated overflow-hidden">
                                        <div class="overflow-x-auto">
                                            <table class="w-full">
                                                <thead class="bg-cream-50 dark:bg-charcoal-800 border-b border-cream-200 dark:border-charcoal-700">
                                                    <tr>
                                                        <th class="px-4 py-3 text-left text-xs font-semibold text-charcoal-500 dark:text-charcoal-400 uppercase tracking-wider">
                                                            "ID"
                                                        </th>
                                                        <th class="px-4 py-3 text-left text-xs font-semibold text-charcoal-500 dark:text-charcoal-400 uppercase tracking-wider">
                                                            "Agent"
                                                        </th>
                                                        <th class="px-4 py-3 text-left text-xs font-semibold text-charcoal-500 dark:text-charcoal-400 uppercase tracking-wider">
                                                            "Path Pattern"
                                                        </th>
                                                        <th class="px-4 py-3 text-left text-xs font-semibold text-charcoal-500 dark:text-charcoal-400 uppercase tracking-wider">
                                                            "Type"
                                                        </th>
                                                        <th class="px-4 py-3 text-left text-xs font-semibold text-charcoal-500 dark:text-charcoal-400 uppercase tracking-wider">
                                                            "Created"
                                                        </th>
                                                    </tr>
                                                </thead>
                                                <tbody class="divide-y divide-cream-200 dark:divide-charcoal-700">
                                                    {res.iter().map(|r| {
                                                        let id = r.id;
                                                        let agent = r.agent_name.clone();
                                                        let path = r.path_pattern.clone();
                                                        let exclusive = r.exclusive;
                                                        let created = r.created_ts.clone();
                                                        let expired = r.expired;

                                                        view! {
                                                            <tr class={if expired { "opacity-50 line-through" } else { "" }}>
                                                                <td class="px-4 py-3 text-sm font-mono text-charcoal-500">
                                                                    {format!("#{}", id)}
                                                                </td>
                                                                <td class="px-4 py-3">
                                                                    <div class="flex items-center gap-2">
                                                                        <AgentAvatar name={agent.clone()} size="sm" />
                                                                        <span class="text-sm font-medium text-charcoal-700 dark:text-cream-200">
                                                                            {agent}
                                                                        </span>
                                                                    </div>
                                                                </td>
                                                                <td class="px-4 py-3">
                                                                    <code class="text-sm bg-cream-100 dark:bg-charcoal-800 px-2 py-1 rounded font-mono">
                                                                        {path}
                                                                    </code>
                                                                </td>
                                                                <td class="px-4 py-3">
                                                                    {if exclusive {
                                                                        view! {
                                                                            <span class="inline-flex items-center gap-1 text-xs px-2 py-1 bg-rose-100 dark:bg-rose-900/30 text-rose-700 dark:text-rose-300 rounded-full">
                                                                                <i data-lucide="lock" class="icon-xs"></i>
                                                                                "Exclusive"
                                                                            </span>
                                                                        }
                                                                    } else {
                                                                        view! {
                                                                            <span class="inline-flex items-center gap-1 text-xs px-2 py-1 bg-sky-100 dark:bg-sky-900/30 text-sky-700 dark:text-sky-300 rounded-full">
                                                                                <i data-lucide="users" class="icon-xs"></i>
                                                                                "Shared"
                                                                            </span>
                                                                        }
                                                                    }}
                                                                </td>
                                                                <td class="px-4 py-3 text-sm text-charcoal-500 whitespace-nowrap">
                                                                    {created.split('T').next().unwrap_or(&created).to_string()}
                                                                </td>
                                                            </tr>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                }.into_any()
                            }}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
