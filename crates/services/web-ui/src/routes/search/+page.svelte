<script lang="ts">
    import { onMount } from "svelte";
    import { page } from "$app/stores";
    import { goto } from "$app/navigation";
    import {
        searchMessages,
        getProjects,
        type Message,
        type Project,
    } from "$lib/api/client";
    import {
        Search,
        X,
        User,
        Folder,
        AlertTriangle,
        SearchX,
    } from "lucide-svelte";
    import * as Card from "$lib/components/ui/card";
    import { Input } from "$lib/components/ui/input";
    import { Badge } from "$lib/components/ui/badge";
    import { Skeleton } from "$lib/components/ui/skeleton";
    import * as Select from "$lib/components/ui/select";

    // State
    let searchInput = $state("");
    let searchQuery = $state("");
    let selectedProject = $state("");
    let projects = $state<Project[]>([]);
    let results = $state<Message[]>([]);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let hasSearched = $state(false);

    // Debounce timer
    let debounceTimer: ReturnType<typeof setTimeout> | null = null;
    const DEBOUNCE_MS = 300;

    // Initialize from URL params
    onMount(() => {
        const urlQuery = $page.url.searchParams.get("q") || "";
        const urlProject = $page.url.searchParams.get("project") || "";
        searchInput = urlQuery;
        searchQuery = urlQuery;
        selectedProject = urlProject;

        // Load projects for filter
        loadProjects();

        // If we have an initial query, search
        if (urlQuery) {
            executeSearch();
        }
    });

    async function loadProjects() {
        try {
            projects = await getProjects();
        } catch (e) {
            console.error("Failed to load projects:", e);
        }
    }

    function handleInputChange(value: string) {
        searchInput = value;

        // Debounce
        if (debounceTimer) {
            clearTimeout(debounceTimer);
        }
        debounceTimer = setTimeout(() => {
            searchQuery = value;
            executeSearch();
        }, DEBOUNCE_MS);
    }

    async function executeSearch() {
        if (!searchQuery.trim()) {
            results = [];
            hasSearched = false;
            return;
        }

        loading = true;
        hasSearched = true;
        error = null;

        try {
            // Use selected project or first available
            const projectSlug = selectedProject || (projects[0]?.slug ?? "");

            if (!projectSlug) {
                error = "No projects available to search";
                loading = false;
                return;
            }

            results = await searchMessages({
                project_slug: projectSlug,
                query: searchQuery,
                limit: 50,
            });

            // Update URL
            const url = new URL(window.location.href);
            url.searchParams.set("q", searchQuery);
            if (selectedProject) {
                url.searchParams.set("project", selectedProject);
            } else {
                url.searchParams.delete("project");
            }
            goto(url.pathname + url.search, {
                replaceState: true,
                keepFocus: true,
            });
        } catch (e) {
            error = e instanceof Error ? e.message : "Search failed";
        } finally {
            loading = false;
        }
    }

    function clearSearch() {
        searchInput = "";
        searchQuery = "";
        results = [];
        hasSearched = false;
    }

    function clearProjectFilter() {
        selectedProject = "";
        executeSearch();
    }

    function handleProjectChange(value: string) {
        selectedProject = value;
        if (searchQuery) {
            executeSearch();
        }
    }

    // Highlight search terms in text
    function highlightText(
        text: string,
        query: string,
    ): { before: string; match: string; after: string }[] {
        if (!query) return [{ before: text, match: "", after: "" }];

        const parts: { before: string; match: string; after: string }[] = [];
        const lowerText = text.toLowerCase();
        const lowerQuery = query.toLowerCase();
        let lastIndex = 0;

        let index = lowerText.indexOf(lowerQuery);
        while (index !== -1) {
            parts.push({
                before: text.slice(lastIndex, index),
                match: text.slice(index, index + query.length),
                after: "",
            });
            lastIndex = index + query.length;
            index = lowerText.indexOf(lowerQuery, lastIndex);
        }

        if (lastIndex < text.length) {
            if (parts.length > 0) {
                parts[parts.length - 1].after = text.slice(lastIndex);
            } else {
                parts.push({
                    before: text.slice(lastIndex),
                    match: "",
                    after: "",
                });
            }
        }

        return parts.length > 0
            ? parts
            : [{ before: text, match: "", after: "" }];
    }

    // Create snippet centered around first match
    function createSnippet(body: string, query: string, maxLen = 200): string {
        if (!query || !body) return body?.slice(0, maxLen) || "";

        const lowerBody = body.toLowerCase();
        const lowerQuery = query.toLowerCase();
        const pos = lowerBody.indexOf(lowerQuery);

        if (pos === -1) {
            return body.length > maxLen ? body.slice(0, maxLen) + "..." : body;
        }

        const start = Math.max(0, pos - maxLen / 3);
        const end = Math.min(body.length, start + maxLen);
        let snippet = body.slice(start, end);

        if (start > 0) snippet = "..." + snippet;
        if (end < body.length) snippet = snippet + "...";

        return snippet;
    }
</script>

<svelte:head>
    <title>Search Messages | MCP Agent Mail</title>
</svelte:head>

<div class="pt-4 md:pt-6 pb-4 md:pb-6 space-y-6">
    <!-- Header -->
    <div class="space-y-4">
        <div class="flex items-center gap-3">
            <Search class="h-6 w-6 text-amber-500" />
            <h1 class="text-2xl font-bold text-foreground">Search Messages</h1>
        </div>

        <!-- Search input and filters -->
        <div class="flex flex-col sm:flex-row gap-3">
            <div class="flex-1 relative">
                <Search
                    class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none"
                />
                <Input
                    id="search-input"
                    type="text"
                    value={searchInput}
                    oninput={(e) => handleInputChange(e.currentTarget.value)}
                    placeholder="Search messages..."
                    class="pl-10"
                />
            </div>

            <!-- Project filter -->
            {#if projects.length > 0}
                <Select.Root
                    type="single"
                    value={selectedProject}
                    onValueChange={(v) => handleProjectChange(v ?? "")}
                >
                    <Select.Trigger class="w-full sm:w-48">
                        {selectedProject || "All Projects"}
                    </Select.Trigger>
                    <Select.Content>
                        <Select.Item value="">All Projects</Select.Item>
                        {#each projects as project}
                            <Select.Item value={project.slug}
                                >{project.slug}</Select.Item
                            >
                        {/each}
                    </Select.Content>
                </Select.Root>
            {/if}
        </div>

        <!-- Active filters as chips -->
        {#if searchQuery || selectedProject}
            <div
                class="flex flex-wrap gap-2"
                role="list"
                aria-label="Active filters"
            >
                {#if searchQuery}
                    <Badge variant="default" class="flex items-center gap-1">
                        <Search class="h-3 w-3" />
                        "{searchQuery}"
                        <button
                            type="button"
                            onclick={clearSearch}
                            class="ml-1 hover:text-destructive"
                            aria-label="Clear search"
                        >
                            <X class="h-3 w-3" />
                        </button>
                    </Badge>
                {/if}
                {#if selectedProject}
                    <Badge variant="secondary" class="flex items-center gap-1">
                        <Folder class="h-3 w-3" />
                        {selectedProject}
                        <button
                            type="button"
                            onclick={clearProjectFilter}
                            class="ml-1 hover:text-destructive"
                            aria-label="Clear project filter"
                        >
                            <X class="h-3 w-3" />
                        </button>
                    </Badge>
                {/if}
            </div>
        {/if}
    </div>

    <!-- Screen reader announcement -->
    <div class="sr-only" aria-live="polite" aria-atomic="true">
        {#if hasSearched && !loading}
            {results.length} results found for {searchQuery}
        {/if}
    </div>

    <!-- Error display -->
    {#if error}
        <div
            class="rounded-xl border border-destructive/50 bg-destructive/10 p-4"
        >
            <div class="flex items-start gap-3">
                <AlertTriangle class="h-5 w-5 text-destructive" />
                <p class="text-destructive">{error}</p>
            </div>
        </div>
    {/if}

    <!-- Loading state -->
    {#if loading}
        <div class="space-y-4">
            {#each Array(3) as _}
                <Skeleton class="h-24 w-full" />
            {/each}
        </div>
    {:else if hasSearched && results.length === 0}
        <!-- No results -->
        <div class="text-center py-12">
            <SearchX
                class="h-12 w-12 mx-auto mb-4 text-muted-foreground opacity-50"
            />
            <h2 class="text-lg font-medium text-foreground">
                No results found
            </h2>
            <p class="text-muted-foreground mt-2">
                Try different keywords or remove some filters
            </p>
            <div class="mt-4 text-sm text-muted-foreground">
                <p class="font-medium">Suggestions:</p>
                <ul class="mt-2 space-y-1">
                    <li>• Check your spelling</li>
                    <li>• Try more general terms</li>
                    <li>• Search within a different project</li>
                </ul>
            </div>
        </div>
    {:else if results.length > 0}
        <!-- Results -->
        <div class="space-y-4">
            <p class="text-sm text-muted-foreground">
                {results.length} result{results.length === 1 ? "" : "s"}
            </p>

            <div class="space-y-3" role="list" aria-label="Search results">
                {#each results as message}
                    <a
                        href="/inbox/{message.id}?project={message.project_id}"
                        class="block"
                    >
                        <Card.Root
                            class="hover:bg-accent/50 transition-colors cursor-pointer"
                        >
                            <Card.Content class="pt-4">
                                <div class="space-y-2">
                                    <!-- Subject with highlight -->
                                    <h3 class="font-medium text-foreground">
                                        {#each highlightText(message.subject, searchQuery) as part}
                                            {part.before}<mark
                                                class="bg-yellow-200 dark:bg-yellow-800 px-0.5 rounded"
                                                >{part.match}</mark
                                            >{part.after}
                                        {/each}
                                    </h3>

                                    <!-- Metadata -->
                                    <div
                                        class="flex items-center gap-2 text-sm text-muted-foreground"
                                    >
                                        <span class="flex items-center gap-1">
                                            <User class="h-3 w-3" />
                                            {message.sender_name || "Unknown"}
                                        </span>
                                        <span>·</span>
                                        <span>{message.created_ts}</span>
                                    </div>

                                    <!-- Body snippet with highlights -->
                                    <p
                                        class="text-sm text-muted-foreground line-clamp-2"
                                    >
                                        {#each highlightText(createSnippet(message.body_md, searchQuery), searchQuery) as part}
                                            {part.before}<mark
                                                class="bg-yellow-200 dark:bg-yellow-800 px-0.5 rounded"
                                                >{part.match}</mark
                                            >{part.after}
                                        {/each}
                                    </p>
                                </div>
                            </Card.Content>
                        </Card.Root>
                    </a>
                {/each}
            </div>
        </div>
    {:else}
        <!-- Initial state -->
        <div class="text-center py-12 text-muted-foreground">
            <Search class="h-12 w-12 mx-auto mb-4 opacity-30" />
            <p>Enter a search term to find messages</p>
        </div>
    {/if}
</div>
