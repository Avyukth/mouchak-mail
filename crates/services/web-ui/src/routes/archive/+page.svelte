<script lang="ts">
    import { onMount } from "svelte";
    import { listArchiveCommits, type ArchiveCommit } from "$lib/api/client";
    import {
        GitBranch,
        GitCommit,
        FileDiff,
        ChevronRight,
        Loader2,
        AlertCircle,
    } from "lucide-svelte";

    // ============================================================================
    // State
    // ============================================================================

    let commits = $state<ArchiveCommit[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);

    // ============================================================================
    // Data Fetching
    // ============================================================================

    async function loadCommits() {
        loading = true;
        error = null;

        try {
            commits = await listArchiveCommits(50);
        } catch (e) {
            error = e instanceof Error ? e.message : "Failed to load commits";
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        loadCommits();
    });

    // ============================================================================
    // Helpers
    // ============================================================================

    function formatDate(dateStr: string): string {
        if (!dateStr) return "—";
        return dateStr.split("T")[0] ?? dateStr;
    }

    function getShortSha(sha: string): string {
        return sha.slice(0, 7);
    }
</script>

<svelte:head>
    <title>Archive Browser | MCP Agent Mail</title>
</svelte:head>

<div class="space-y-6">
    <!-- Breadcrumb -->
    <nav class="text-sm text-charcoal-500 dark:text-charcoal-400">
        <span class="font-medium text-charcoal-700 dark:text-charcoal-200"
            >Archive</span
        >
    </nav>

    <!-- Page Header -->
    <div class="flex items-center gap-3">
        <div class="p-3 bg-violet-100 dark:bg-violet-900/30 rounded-xl">
            <GitBranch class="w-6 h-6 text-violet-600" />
        </div>
        <div>
            <h1
                class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100"
            >
                Archive Browser
            </h1>
            <p class="text-charcoal-500 dark:text-charcoal-400 text-sm">
                Explore git history and browse files at any commit
            </p>
        </div>
    </div>

    <!-- Loading State -->
    {#if loading}
        <div class="flex items-center justify-center py-12">
            <Loader2 class="w-8 h-8 text-violet-500 animate-spin" />
        </div>
    {/if}

    <!-- Error State -->
    {#if error}
        <div
            class="rounded-xl border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/20 p-6 text-center"
        >
            <AlertCircle class="w-8 h-8 mx-auto mb-2 text-red-500" />
            <p class="text-red-700 dark:text-red-400">{error}</p>
        </div>
    {/if}

    <!-- Content -->
    {#if !loading && !error}
        <div class="space-y-4">
            <p class="text-sm text-charcoal-500 dark:text-charcoal-400">
                {commits.length} commit{commits.length !== 1 ? "s" : ""}
            </p>

            {#if commits.length === 0}
                <div
                    class="rounded-xl border border-charcoal-200 dark:border-charcoal-700 bg-white dark:bg-charcoal-900 p-8 text-center text-charcoal-400"
                >
                    <GitCommit class="w-12 h-12 mx-auto mb-3 opacity-50" />
                    <p>No commits yet</p>
                    <p class="text-sm mt-1">
                        The archive will populate as changes are made
                    </p>
                </div>
            {:else}
                <div
                    class="rounded-xl border border-charcoal-200 dark:border-charcoal-700 bg-white dark:bg-charcoal-900 overflow-hidden"
                >
                    <ul
                        class="divide-y divide-charcoal-200 dark:divide-charcoal-700"
                    >
                        {#each commits as commit}
                            <li class="group">
                                <a
                                    href="/archive/commit/{commit.sha}"
                                    class="flex items-start gap-4 px-6 py-4 hover:bg-charcoal-50 dark:hover:bg-charcoal-800/50 transition-colors"
                                >
                                    <div
                                        class="flex-shrink-0 p-2 bg-violet-100 dark:bg-violet-900/30 rounded-lg"
                                    >
                                        <GitCommit
                                            class="w-4 h-4 text-violet-600 dark:text-violet-400"
                                        />
                                    </div>
                                    <div class="flex-1 min-w-0">
                                        <div
                                            class="flex items-baseline justify-between gap-4 mb-1"
                                        >
                                            <h4
                                                class="font-medium text-charcoal-800 dark:text-cream-100 truncate group-hover:text-violet-600 transition-colors"
                                            >
                                                {commit.message}
                                            </h4>
                                            <span
                                                class="flex-shrink-0 text-xs font-mono text-charcoal-400"
                                            >
                                                {formatDate(commit.timestamp)}
                                            </span>
                                        </div>
                                        <div
                                            class="flex items-center gap-4 text-sm text-charcoal-500 dark:text-charcoal-400"
                                        >
                                            <span
                                                class="font-mono text-xs bg-charcoal-100 dark:bg-charcoal-700 px-2 py-0.5 rounded"
                                            >
                                                {getShortSha(commit.sha)}
                                            </span>
                                            <span>{commit.author}</span>
                                        </div>
                                    </div>
                                    <ChevronRight
                                        class="w-4 h-4 text-charcoal-300 group-hover:text-violet-500 transition-colors"
                                    />
                                </a>
                            </li>
                        {/each}
                    </ul>
                </div>
            {/if}
        </div>
    {/if}
</div>
