<script lang="ts">
    import { onMount } from "svelte";
    import { page } from "$app/stores";
    import { goto } from "$app/navigation";
    import { getThread, type Thread, type Message } from "$lib/api/client";
    import {
        ArrowLeft,
        ChevronDown,
        ChevronRight,
        User,
        Reply,
        Loader2,
        MessageSquareOff,
        AlertTriangle,
    } from "lucide-svelte";

    // ============================================================================
    // State
    // ============================================================================

    let thread: Thread | null = $state(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let focusedIndex = $state(0);
    let expandedStates = $state<Map<number, boolean>>(new Map());

    // Maximum indentation depth
    const MAX_DEPTH = 5;

    // ============================================================================
    // Derived Values
    // ============================================================================

    const threadId = $derived($page.params.id);
    const projectSlug = $derived($page.url.searchParams.get("project") ?? "");

    // Thread nodes with depth info
    interface ThreadNode {
        message: Message;
        depth: number;
        index: number;
    }

    const threadNodes = $derived.by(() => {
        if (!thread?.messages) return [];
        return thread.messages.map((msg, idx) => ({
            message: msg,
            depth: idx === 0 ? 0 : 1, // First message is root, others are replies
            index: idx,
        }));
    });

    // ============================================================================
    // Data Fetching
    // ============================================================================

    async function loadThread() {
        if (!threadId || !projectSlug) {
            loading = false;
            error = "Missing thread ID or project";
            return;
        }

        loading = true;
        error = null;

        try {
            thread = await getThread(projectSlug, threadId);
            // Expand first message by default
            if (thread?.messages?.[0]) {
                expandedStates.set(thread.messages[0].id, true);
            }
        } catch (e) {
            error = e instanceof Error ? e.message : "Failed to load thread";
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        loadThread();
    });

    // ============================================================================
    // Actions
    // ============================================================================

    function toggleExpanded(messageId: number) {
        const current = expandedStates.get(messageId) ?? false;
        expandedStates = new Map(expandedStates.set(messageId, !current));
    }

    function isExpanded(messageId: number): boolean {
        return expandedStates.get(messageId) ?? false;
    }

    function goBack() {
        history.back();
    }

    function getReplyUrl(messageId: number): string {
        return `/inbox/${messageId}?project=${encodeURIComponent(projectSlug)}&reply=true`;
    }

    // ============================================================================
    // Keyboard Navigation
    // ============================================================================

    function handleKeydown(event: KeyboardEvent) {
        const nodes = threadNodes;
        const maxIdx = nodes.length - 1;

        switch (event.key) {
            case "ArrowUp":
            case "k":
                event.preventDefault();
                if (focusedIndex > 0) {
                    focusedIndex--;
                }
                break;

            case "ArrowDown":
            case "j":
                event.preventDefault();
                if (focusedIndex < maxIdx) {
                    focusedIndex++;
                }
                break;

            case "Enter":
            case " ":
                event.preventDefault();
                const node = nodes[focusedIndex];
                if (node) {
                    toggleExpanded(node.message.id);
                }
                break;

            case "Escape":
                event.preventDefault();
                goBack();
                break;
        }
    }

    // ============================================================================
    // Indentation Classes
    // ============================================================================

    function getIndentClass(depth: number): string {
        const capped = Math.min(depth, MAX_DEPTH);
        switch (capped) {
            case 0:
                return "";
            case 1:
                return "ml-4 sm:ml-6";
            case 2:
                return "ml-8 sm:ml-12";
            case 3:
                return "ml-12 sm:ml-18";
            case 4:
                return "ml-16 sm:ml-24";
            default:
                return "ml-20 sm:ml-30";
        }
    }
</script>

<svelte:head>
    <title>{thread?.subject ?? "Thread"} | MCP Agent Mail</title>
</svelte:head>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
    class="thread-view space-y-4"
    tabindex="0"
    onkeydown={handleKeydown}
    role="application"
    aria-label="Thread view"
>
    <!-- Header with back button and keyboard hints -->
    <div class="flex items-center justify-between">
        <nav
            class="flex items-center gap-2 text-sm text-charcoal-500 dark:text-charcoal-400"
        >
            <button
                onclick={goBack}
                class="flex items-center gap-1.5 hover:text-amber-600 dark:hover:text-amber-400 transition-colors min-h-[44px] px-2 rounded-lg hover:bg-charcoal-100 dark:hover:bg-charcoal-800"
            >
                <ArrowLeft class="w-4 h-4" />
                <span>Back to Inbox</span>
            </button>
        </nav>

        <div
            class="hidden sm:flex items-center gap-2 text-sm text-charcoal-400 dark:text-charcoal-500"
        >
            <kbd
                class="px-1.5 py-0.5 rounded bg-charcoal-100 dark:bg-charcoal-800 text-xs"
                >↑↓</kbd
            >
            <span>navigate</span>
            <kbd
                class="px-1.5 py-0.5 rounded bg-charcoal-100 dark:bg-charcoal-800 text-xs"
                >Enter</kbd
            >
            <span>expand</span>
            <kbd
                class="px-1.5 py-0.5 rounded bg-charcoal-100 dark:bg-charcoal-800 text-xs"
                >Esc</kbd
            >
            <span>back</span>
        </div>
    </div>

    <!-- Error State -->
    {#if error}
        <div
            class="rounded-xl border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/20 p-4"
        >
            <div class="flex items-start gap-3">
                <AlertTriangle class="w-5 h-5 text-red-500 flex-shrink-0" />
                <p class="text-red-700 dark:text-red-400">{error}</p>
            </div>
        </div>
    {/if}

    <!-- Loading State -->
    {#if loading}
        <div class="flex items-center justify-center py-12">
            <Loader2 class="w-6 h-6 text-amber-500 animate-spin" />
            <span class="ml-2 text-charcoal-500">Loading thread...</span>
        </div>
    {/if}

    <!-- Thread Content -->
    {#if !loading && !error && thread}
        <!-- Thread Header -->
        <div class="border-b border-charcoal-200 dark:border-charcoal-700 pb-4">
            <h1
                class="text-xl font-semibold text-charcoal-900 dark:text-charcoal-100"
            >
                {thread.subject || "(No subject)"}
            </h1>
            <div
                class="flex items-center gap-2 mt-2 text-sm text-charcoal-500 dark:text-charcoal-400"
            >
                <span
                    >{thread.message_count} message{thread.message_count !== 1
                        ? "s"
                        : ""}</span
                >
                <span>·</span>
                <span
                    >{thread.participants.length} participant{thread
                        .participants.length !== 1
                        ? "s"
                        : ""}: {thread.participants.join(", ")}</span
                >
            </div>
        </div>

        <!-- Message Tree -->
        {#if threadNodes.length === 0}
            <div
                class="text-center py-12 text-charcoal-500 dark:text-charcoal-400"
            >
                <MessageSquareOff class="w-12 h-12 mx-auto mb-4 opacity-50" />
                <p>No messages in this thread</p>
            </div>
        {:else}
            <div class="space-y-2" role="tree" aria-label="Message thread">
                {#each threadNodes as node, idx}
                    {@const isFocused = idx === focusedIndex}
                    {@const expanded = isExpanded(node.message.id)}
                    {@const indentClass = getIndentClass(node.depth)}

                    <div
                        class="{indentClass} {isFocused
                            ? 'ring-2 ring-amber-500 ring-offset-2 rounded-xl'
                            : ''}"
                        role="treeitem"
                        aria-selected={isFocused}
                        aria-expanded={expanded}
                        tabindex={isFocused ? 0 : -1}
                    >
                        <div
                            class="rounded-xl border border-charcoal-200 dark:border-charcoal-700 bg-white dark:bg-charcoal-900 shadow-sm"
                        >
                            <div class="p-4">
                                <!-- Header row with collapse toggle -->
                                <div class="flex items-start gap-3">
                                    <!-- Collapse/expand button -->
                                    <button
                                        type="button"
                                        class="mt-1 p-1 rounded hover:bg-charcoal-100 dark:hover:bg-charcoal-800 transition-colors"
                                        onclick={() =>
                                            toggleExpanded(node.message.id)}
                                        aria-label={expanded
                                            ? "Collapse"
                                            : "Expand"}
                                    >
                                        {#if expanded}
                                            <ChevronDown
                                                class="w-4 h-4 text-charcoal-400"
                                            />
                                        {:else}
                                            <ChevronRight
                                                class="w-4 h-4 text-charcoal-400"
                                            />
                                        {/if}
                                    </button>

                                    <!-- Message content -->
                                    <div class="flex-1 min-w-0">
                                        <!-- Subject and metadata -->
                                        <div
                                            class="flex items-center gap-2 flex-wrap"
                                        >
                                            <h3
                                                class="font-medium text-charcoal-900 dark:text-charcoal-100 truncate"
                                            >
                                                {node.message.subject ||
                                                    "(No subject)"}
                                            </h3>
                                            {#if node.depth > 0}
                                                <span
                                                    class="sm:hidden badge badge-charcoal text-xs"
                                                >
                                                    ↳ {node.depth}
                                                </span>
                                            {/if}
                                        </div>

                                        <!-- Sender and timestamp -->
                                        <div
                                            class="flex items-center gap-2 text-sm text-charcoal-500 dark:text-charcoal-400 mt-1"
                                        >
                                            <span
                                                class="flex items-center gap-1"
                                            >
                                                <User class="w-3 h-3" />
                                                {node.message.sender_name ??
                                                    "Unknown"}
                                            </span>
                                            <span>·</span>
                                            <span
                                                >{node.message
                                                    .created_relative ??
                                                    node.message
                                                        .created_ts}</span
                                            >
                                        </div>

                                        <!-- Body preview or full body -->
                                        <div
                                            class="mt-2 text-charcoal-600 dark:text-charcoal-300"
                                        >
                                            {#if expanded}
                                                <div
                                                    class="whitespace-pre-wrap"
                                                >
                                                    {node.message.body_md}
                                                </div>
                                            {:else}
                                                <p class="line-clamp-2">
                                                    {node.message.body_md.slice(
                                                        0,
                                                        200,
                                                    )}{node.message.body_md
                                                        .length > 200
                                                        ? "..."
                                                        : ""}
                                                </p>
                                            {/if}
                                        </div>

                                        <!-- Reply button (when expanded) -->
                                        {#if expanded}
                                            <div
                                                class="mt-4 pt-3 border-t border-charcoal-200 dark:border-charcoal-700"
                                            >
                                                <a
                                                    href={getReplyUrl(
                                                        node.message.id,
                                                    )}
                                                    class="inline-flex items-center gap-2 px-3 py-1.5 text-sm border border-charcoal-300 dark:border-charcoal-600 rounded-lg hover:bg-charcoal-100 dark:hover:bg-charcoal-800 transition-colors"
                                                >
                                                    <Reply class="w-4 h-4" />
                                                    Reply
                                                </a>
                                            </div>
                                        {/if}
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    {/if}
</div>

<style>
    .thread-view:focus {
        outline: none;
    }

    .badge {
        display: inline-flex;
        align-items: center;
        padding: 0.125rem 0.5rem;
        border-radius: 9999px;
        font-size: 0.75rem;
        font-weight: 500;
    }

    .badge-charcoal {
        background-color: var(--charcoal-100, #f3f4f6);
        color: var(--charcoal-600, #4b5563);
    }

    :global(.dark) .badge-charcoal {
        background-color: var(--charcoal-800, #1f2937);
        color: var(--charcoal-400, #9ca3af);
    }

    kbd {
        font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas,
            "Liberation Mono", monospace;
    }
</style>
