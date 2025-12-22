<script lang="ts">
    import { onMount } from "svelte";
    import {
        listAttachments,
        getProjects,
        type Attachment,
        type Project,
    } from "$lib/api/client";
    import {
        Paperclip,
        Download,
        FileImage,
        FileText,
        File,
        FileCode,
        FileArchive,
        Loader2,
        AlertCircle,
    } from "lucide-svelte";

    // ============================================================================
    // State
    // ============================================================================

    let attachments = $state<Attachment[]>([]);
    let projects = $state<Project[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let selectedProject = $state<string>("");

    // Sort options
    type SortOption =
        | "date_desc"
        | "date_asc"
        | "name_asc"
        | "name_desc"
        | "size_desc"
        | "size_asc";
    let sortBy = $state<SortOption>("date_desc");

    // ============================================================================
    // Derived Values
    // ============================================================================

    const sortedAttachments = $derived.by(() => {
        const sorted = [...attachments];
        switch (sortBy) {
            case "date_desc":
                return sorted.sort((a, b) =>
                    b.created_at.localeCompare(a.created_at),
                );
            case "date_asc":
                return sorted.sort((a, b) =>
                    a.created_at.localeCompare(b.created_at),
                );
            case "name_asc":
                return sorted.sort((a, b) =>
                    a.filename.localeCompare(b.filename),
                );
            case "name_desc":
                return sorted.sort((a, b) =>
                    b.filename.localeCompare(a.filename),
                );
            case "size_desc":
                return sorted.sort((a, b) => b.size_bytes - a.size_bytes);
            case "size_asc":
                return sorted.sort((a, b) => a.size_bytes - b.size_bytes);
            default:
                return sorted;
        }
    });

    // ============================================================================
    // Data Fetching
    // ============================================================================

    async function loadData() {
        loading = true;
        error = null;

        try {
            const [attachmentsData, projectsData] = await Promise.all([
                listAttachments(selectedProject || undefined),
                getProjects(),
            ]);
            attachments = attachmentsData;
            projects = projectsData;
        } catch (e) {
            error =
                e instanceof Error ? e.message : "Failed to load attachments";
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        loadData();
    });

    function handleProjectChange(event: Event) {
        const target = event.target as HTMLSelectElement;
        selectedProject = target.value;
        loadData();
    }

    // ============================================================================
    // Helpers
    // ============================================================================

    function formatFileSize(bytes: number): string {
        if (bytes < 1024) return `${bytes} B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }

    function getFileExtension(filename: string): string {
        return filename.split(".").pop()?.toLowerCase() ?? "";
    }

    function isImageFile(mimeType: string): boolean {
        return mimeType.startsWith("image/");
    }

    function isPdfFile(mimeType: string): boolean {
        return mimeType === "application/pdf";
    }

    function getDownloadUrl(id: number): string {
        return `/api/attachments/${id}`;
    }
</script>

<svelte:head>
    <title>Attachments | MCP Agent Mail</title>
</svelte:head>

<div class="space-y-6">
    <!-- Page Header -->
    <div class="flex items-center gap-3">
        <div class="p-3 bg-indigo-100 dark:bg-indigo-900/30 rounded-xl">
            <Paperclip class="w-6 h-6 text-indigo-600" />
        </div>
        <div>
            <h1
                class="font-display text-2xl font-bold text-charcoal-800 dark:text-cream-100"
            >
                Attachments
            </h1>
            <p class="text-charcoal-500 dark:text-charcoal-400 text-sm">
                Browse and download file attachments from messages
            </p>
        </div>
    </div>

    <!-- Filters -->
    <div class="flex flex-wrap items-center gap-4">
        <div class="flex items-center gap-2">
            <label
                for="project-filter"
                class="text-sm text-charcoal-500 dark:text-charcoal-400"
            >
                Project:
            </label>
            <select
                id="project-filter"
                class="px-3 py-1.5 text-sm border border-charcoal-200 dark:border-charcoal-700 rounded-lg bg-white dark:bg-charcoal-900 text-charcoal-700 dark:text-charcoal-200"
                value={selectedProject}
                onchange={handleProjectChange}
            >
                <option value="">All Projects</option>
                {#each projects as project}
                    <option value={project.slug}
                        >{project.human_key || project.slug}</option
                    >
                {/each}
            </select>
        </div>

        <div class="flex items-center gap-2">
            <label
                for="sort-by"
                class="text-sm text-charcoal-500 dark:text-charcoal-400"
            >
                Sort:
            </label>
            <select
                id="sort-by"
                class="px-3 py-1.5 text-sm border border-charcoal-200 dark:border-charcoal-700 rounded-lg bg-white dark:bg-charcoal-900 text-charcoal-700 dark:text-charcoal-200"
                bind:value={sortBy}
            >
                <option value="date_desc">Newest First</option>
                <option value="date_asc">Oldest First</option>
                <option value="name_asc">Name A-Z</option>
                <option value="name_desc">Name Z-A</option>
                <option value="size_desc">Largest First</option>
                <option value="size_asc">Smallest First</option>
            </select>
        </div>
    </div>

    <!-- Loading State -->
    {#if loading}
        <div class="flex items-center justify-center py-12">
            <Loader2 class="w-8 h-8 text-indigo-500 animate-spin" />
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
                {sortedAttachments.length} attachment{sortedAttachments.length !==
                1
                    ? "s"
                    : ""}
            </p>

            {#if sortedAttachments.length === 0}
                <div
                    class="rounded-xl border border-charcoal-200 dark:border-charcoal-700 bg-white dark:bg-charcoal-900 p-8 text-center text-charcoal-400"
                >
                    <Paperclip class="w-12 h-12 mx-auto mb-3 opacity-50" />
                    <p>No attachments found</p>
                    <p class="text-sm mt-1">
                        {selectedProject
                            ? "Try selecting a different project"
                            : "Attachments will appear when added to messages"}
                    </p>
                </div>
            {:else}
                <div
                    class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4"
                >
                    {#each sortedAttachments as attachment}
                        {@const ext = getFileExtension(attachment.filename)}
                        {@const isImage = isImageFile(attachment.mime_type)}
                        {@const isPdf = isPdfFile(attachment.mime_type)}

                        <div
                            class="rounded-xl border border-charcoal-200 dark:border-charcoal-700 bg-white dark:bg-charcoal-900 overflow-hidden hover:border-indigo-300 dark:hover:border-indigo-700 transition-colors group"
                        >
                            <!-- Preview Area -->
                            <div
                                class="h-32 flex items-center justify-center bg-charcoal-50 dark:bg-charcoal-800"
                            >
                                {#if isImage}
                                    <FileImage
                                        class="w-12 h-12 text-green-500"
                                    />
                                {:else if isPdf}
                                    <FileText class="w-12 h-12 text-red-500" />
                                {:else if ["js", "ts", "py", "rs", "go", "java"].includes(ext)}
                                    <FileCode class="w-12 h-12 text-blue-500" />
                                {:else if ["zip", "tar", "gz", "7z", "rar"].includes(ext)}
                                    <FileArchive
                                        class="w-12 h-12 text-amber-500"
                                    />
                                {:else}
                                    <File class="w-12 h-12 text-charcoal-400" />
                                {/if}
                            </div>

                            <!-- File Info -->
                            <div class="p-4">
                                <h3
                                    class="font-medium text-charcoal-800 dark:text-cream-100 truncate text-sm"
                                    title={attachment.filename}
                                >
                                    {attachment.filename}
                                </h3>
                                <div
                                    class="flex items-center justify-between mt-2 text-xs text-charcoal-500 dark:text-charcoal-400"
                                >
                                    <span
                                        >{formatFileSize(
                                            attachment.size_bytes,
                                        )}</span
                                    >
                                    <span
                                        >{attachment.created_at.split(
                                            "T",
                                        )[0]}</span
                                    >
                                </div>

                                <!-- Download Button -->
                                <a
                                    href={getDownloadUrl(attachment.id)}
                                    download={attachment.filename}
                                    class="mt-3 w-full inline-flex items-center justify-center gap-2 px-3 py-1.5 text-sm bg-indigo-50 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400 rounded-lg hover:bg-indigo-100 dark:hover:bg-indigo-900/50 transition-colors"
                                >
                                    <Download class="w-4 h-4" />
                                    Download
                                </a>
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    {/if}
</div>
