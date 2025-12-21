//! Archive Browser page - view git history and browse files at commits.

use crate::api::client::{self, CommitSummary};
use crate::components::{Breadcrumb, BreadcrumbItem};
use leptos::prelude::*;

/// Archive Browser page component.
#[component]
pub fn ArchiveBrowser() -> impl IntoView {
    // State
    let commits = RwSignal::new(Vec::<CommitSummary>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);

    // Load commits on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            loading.set(true);
            match client::get_archive_commits(Some(50)).await {
                Ok(c) => {
                    commits.set(c);
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
                BreadcrumbItem::new("Archive", ""),
            ]} />

            // Page Header
            <div class="flex items-center gap-3">
                <div class="p-3 bg-violet-100 dark:bg-violet-900/30 rounded-xl">
                    <i data-lucide="git-branch" class="icon-lg text-violet-600"></i>
                </div>
                <div>
                    <h1 class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100">
                        "Archive Browser"
                    </h1>
                    <p class="text-charcoal-500 dark:text-charcoal-400 text-sm">
                        "Explore git history and browse files at any commit"
                    </p>
                </div>
            </div>

            // Loading/Error/Content
            {move || {
                if loading.get() {
                    view! {
                        <div class="flex items-center justify-center py-12">
                            <i data-lucide="loader-2" class="icon-xl text-violet-500 animate-spin"></i>
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
                    let commit_list = commits.get();
                    let count = commit_list.len();

                    view! {
                        <div class="space-y-4">
                            <p class="text-sm text-charcoal-500 dark:text-charcoal-400">
                                {format!("{} commit{}", count, if count == 1 { "" } else { "s" })}
                            </p>

                            {if commit_list.is_empty() {
                                view! {
                                    <div class="card-elevated p-8 text-center text-charcoal-400">
                                        <i data-lucide="git-commit" class="icon-xl mx-auto mb-3 opacity-50"></i>
                                        <p>"No commits yet"</p>
                                        <p class="text-sm mt-1">
                                            "The archive will populate as changes are made"
                                        </p>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="card-elevated overflow-hidden">
                                        <ul class="divide-y divide-cream-200 dark:divide-charcoal-700">
                                            {commit_list.iter().map(|commit| {
                                                let sha = commit.sha.clone();
                                                let short_sha = commit.short_sha.clone();
                                                let message = commit.message.clone();
                                                let author = commit.author.clone();
                                                let timestamp = commit.timestamp.clone();
                                                let files_changed = commit.files_changed;

                                                view! {
                                                    <li class="group">
                                                        <a
                                                            href={format!("/archive/commit/{}", sha)}
                                                            class="flex items-start gap-4 px-6 py-4 hover:bg-cream-50 dark:hover:bg-charcoal-800/50 transition-colors"
                                                        >
                                                            <div class="flex-shrink-0 p-2 bg-violet-100 dark:bg-violet-900/30 rounded-lg">
                                                                <i data-lucide="git-commit" class="icon-sm text-violet-600 dark:text-violet-400"></i>
                                                            </div>
                                                            <div class="flex-1 min-w-0">
                                                                <div class="flex items-baseline justify-between gap-4 mb-1">
                                                                    <h4 class="font-medium text-charcoal-800 dark:text-cream-100 truncate group-hover:text-violet-600 transition-colors">
                                                                        {message}
                                                                    </h4>
                                                                    <span class="flex-shrink-0 text-xs font-mono text-charcoal-400">
                                                                        {format_date(&timestamp)}
                                                                    </span>
                                                                </div>
                                                                <div class="flex items-center gap-4 text-sm text-charcoal-500 dark:text-charcoal-400">
                                                                    <span class="font-mono text-xs bg-cream-100 dark:bg-charcoal-700 px-2 py-0.5 rounded">
                                                                        {short_sha}
                                                                    </span>
                                                                    <span>{author}</span>
                                                                    <span class="flex items-center gap-1">
                                                                        <i data-lucide="file-diff" class="icon-xs"></i>
                                                                        {files_changed} " file" {if files_changed == 1 { "" } else { "s" }}
                                                                    </span>
                                                                </div>
                                                            </div>
                                                            <i data-lucide="chevron-right" class="icon-sm text-charcoal-300 group-hover:text-violet-500 transition-colors"></i>
                                                        </a>
                                                    </li>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </ul>
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

fn format_date(date_str: &str) -> String {
    if date_str.is_empty() {
        return "—".to_string();
    }
    date_str.split('T').next().unwrap_or(date_str).to_string()
}
