<script lang="ts">
    import { onMount } from "svelte";
    import { page } from "$app/stores";
    import {
        listFileReservations,
        type FileReservation,
    } from "$lib/api/client";
    import {
        Shield,
        Lock,
        Users,
        FileCheck,
        Loader2,
        AlertCircle,
        ChevronRight,
        Home,
    } from "lucide-svelte";

    // ============================================================================
    // State
    // ============================================================================

    let reservations = $state<FileReservation[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);

    // ============================================================================
    // Derived Values
    // ============================================================================

    const projectSlug = $derived($page.params.slug);

    // ============================================================================
    // Data Fetching
    // ============================================================================

    async function loadReservations() {
        if (!projectSlug) {
            loading = false;
            error = "Missing project slug";
            return;
        }

        loading = true;
        error = null;

        try {
            reservations = await listFileReservations(projectSlug);
        } catch (e) {
            error =
                e instanceof Error ? e.message : "Failed to load reservations";
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        loadReservations();
    });

    // ============================================================================
    // Helpers
    // ============================================================================

    function formatDate(dateStr: string): string {
        if (!dateStr) return "—";
        return dateStr.split("T")[0] ?? dateStr;
    }

    function isExpired(expiresTs: string): boolean {
        if (!expiresTs) return false;
        return new Date(expiresTs) < new Date();
    }
</script>

<svelte:head>
    <title>File Reservations - {projectSlug} | MCP Agent Mail</title>
</svelte:head>

<div class="space-y-6">
    <!-- Breadcrumb -->
    <nav
        class="flex items-center gap-2 text-sm text-muted-foreground"
    >
        <a
            href="/projects"
            class="hover:text-amber-600 dark:hover:text-amber-400 transition-colors flex items-center gap-1"
        >
            <Home class="w-3 h-3" />
            Projects
        </a>
        <ChevronRight class="w-3 h-3" />
        <a
            href="/projects/{projectSlug}"
            class="hover:text-amber-600 dark:hover:text-amber-400 transition-colors"
        >
            {projectSlug}
        </a>
        <ChevronRight class="w-3 h-3" />
        <span class="font-medium text-foreground"
            >File Reservations</span
        >
    </nav>

    <!-- Page Header -->
    <div class="flex items-center gap-3">
        <div class="p-3 bg-amber-100 dark:bg-amber-900/30 rounded-xl">
            <Shield class="w-6 h-6 text-amber-600" />
        </div>
        <div>
            <h1
                class="font-display text-2xl font-bold text-foreground"
            >
                File Reservations
            </h1>
            <p class="text-muted-foreground text-sm">
                When agents want to edit files, they can "reserve" them to
                signal their intent.
            </p>
        </div>
    </div>

    <!-- Info Banner -->
    <div
        class="rounded-xl border border-sky-200 dark:border-sky-800 bg-sky-50 dark:bg-sky-900/20 p-4"
    >
        <h3 class="font-medium text-sky-800 dark:text-sky-200 mb-1">
            Advisory system
        </h3>
        <p class="text-sm text-sky-700 dark:text-sky-300">
            Reservations are <em>signals</em>, not hard locks. Agents can still
            edit files, but they'll see warnings if conflicts exist.
        </p>
        <p class="text-sm text-sky-700 dark:text-sky-300 mt-2">
            Install a <a
                href="/projects/{projectSlug}"
                class="underline hover:no-underline">pre-commit hook</a
            > to enforce reservations at commit time.
        </p>
    </div>

    <!-- Loading State -->
    {#if loading}
        <div class="flex items-center justify-center py-12">
            <Loader2 class="w-8 h-8 text-amber-500 animate-spin" />
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
            <p class="text-sm text-muted-foreground">
                {reservations.length} active reservation{reservations.length !==
                1
                    ? "s"
                    : ""}
            </p>

            {#if reservations.length === 0}
                <div
                    class="rounded-xl border border-border bg-card p-8 text-center text-muted-foreground"
                >
                    <FileCheck class="w-12 h-12 mx-auto mb-3 opacity-50" />
                    <p>No active file reservations</p>
                    <p class="text-sm mt-1">
                        Agents can reserve files using the reserve_file tool
                    </p>
                </div>
            {:else}
                <div
                    class="rounded-xl border border-border bg-card overflow-hidden"
                >
                    <div class="overflow-x-auto">
                        <table class="w-full">
                            <thead
                                class="bg-muted border-b border-border"
                            >
                                <tr>
                                    <th
                                        class="px-4 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                    >
                                        ID
                                    </th>
                                    <th
                                        class="px-4 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                    >
                                        Agent
                                    </th>
                                    <th
                                        class="px-4 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                    >
                                        Path Pattern
                                    </th>
                                    <th
                                        class="px-4 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                    >
                                        Type
                                    </th>
                                    <th
                                        class="px-4 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                    >
                                        Created
                                    </th>
                                    <th
                                        class="px-4 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                    >
                                        Agent
                                    </th>
                                    <th
                                        class="px-4 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                    >
                                        Path Pattern
                                    </th>
                                    <th
                                        class="px-4 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                    >
                                        Type
                                    </th>
                                    <th
                                        class="px-4 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                    >
                                        Created
                                    </th>
                                </tr>
                            </thead>
                                <tbody
                                class="divide-y divide-border"
                            >
                                {#each reservations as reservation}
                                    {@const expired = isExpired(
                                        reservation.expires_ts,
                                    )}
                                    <tr class={expired ? "opacity-50" : ""}>
                                        <td
                                            class="px-4 py-3 text-sm font-mono text-muted-foreground"
                                        >
                                            #{reservation.id}
                                        </td>
                                        <td class="px-4 py-3">
                                            <div
                                                class="flex items-center gap-2"
                                            >
                                                <div
                                                    class="w-6 h-6 rounded-full bg-gradient-to-br from-amber-400 to-orange-500 flex items-center justify-center text-white text-xs font-bold"
                                                >
                                                    {reservation.agent_name
                                                        ?.charAt(0)
                                                        .toUpperCase() ?? "?"}
                                                </div>
                                                <span
                                                    class="text-sm font-medium text-foreground"
                                                >
                                                    {reservation.agent_name ??
                                                        "Unknown"}
                                                </span>
                                            </div>
                                        </td>
                                        <td class="px-4 py-3">
                                            <code
                                                class="text-sm bg-muted px-2 py-1 rounded font-mono"
                                            >
                                                {reservation.path_pattern}
                                            </code>
                                        </td>
                                        <td class="px-4 py-3">
                                            {#if reservation.exclusive}
                                                <span
                                                    class="inline-flex items-center gap-1 text-xs px-2 py-1 bg-rose-100 dark:bg-rose-900/30 text-rose-700 dark:text-rose-300 rounded-full"
                                                >
                                                    <Lock class="w-3 h-3" />
                                                    Exclusive
                                                </span>
                                            {:else}
                                                <span
                                                    class="inline-flex items-center gap-1 text-xs px-2 py-1 bg-sky-100 dark:bg-sky-900/30 text-sky-700 dark:text-sky-300 rounded-full"
                                                >
                                                    <Users class="w-3 h-3" />
                                                    Shared
                                                </span>
                                            {/if}
                                        </td>
                                        <td
                                            class="px-4 py-3 text-sm text-muted-foreground whitespace-nowrap"
                                        >
                                            {formatDate(reservation.created_at)}
                                        </td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    </div>
                </div>
            {/if}
        </div>
    {/if}
</div>
