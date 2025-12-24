<script lang="ts">
	import { browser } from "$app/environment";
	import {
		getProjects,
		ensureProject,
		deleteProject,
		type Project,
	} from "$lib/api/client";
	import { slugify } from "$lib/utils/slugify";
	import { toast } from "svelte-sonner";
	import FolderKanban from "lucide-svelte/icons/folder-kanban";
	import ArrowRight from "lucide-svelte/icons/arrow-right";
	import Plus from "lucide-svelte/icons/plus";
	import Calendar from "lucide-svelte/icons/calendar";
	import MoreVertical from "lucide-svelte/icons/more-vertical";
	import Trash2 from "lucide-svelte/icons/trash-2";
	import Pencil from "lucide-svelte/icons/pencil";
	import { ProjectCardSkeleton } from "$lib/components/skeletons";
	import {
		BlurFade,
		ShimmerButton,
		NumberTicker,
	} from "$lib/components/magic";
	import * as Dialog from "$lib/components/ui/dialog";
	import * as Card from "$lib/components/ui/card";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
	import * as AlertDialog from "$lib/components/ui/alert-dialog";
	import { Button } from "$lib/components/ui/button";
	import { Badge } from "$lib/components/ui/badge";
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import { Textarea } from "$lib/components/ui/textarea";
	import { SearchInput } from "$lib/components/ui/search-input";
	import { SortButton, type SortDirection } from "$lib/components/ui/sort-button";
	import { BulkActionBar } from "$lib/components/ui/bulk-action-bar";
	import { Checkbox } from "$lib/components/ui/checkbox";
	import { EmptyState } from "$lib/components/ui/empty-state";

	let projects = $state<Project[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Search filter
	let searchQuery = $state("");

	// Sort state
	type SortField = 'name' | 'date';
	let sortField = $state<SortField>('date');
	let sortDirection = $state<SortDirection>('desc');

	function handleSort(field: string, direction: SortDirection) {
		sortField = field as SortField;
		sortDirection = direction;
	}

	// Selection state
	let selectedIds = $state<Set<number>>(new Set());

	let selectedCount = $derived(selectedIds.size);
	let allSelected = $derived(() => {
		const filtered = filteredProjects();
		return filtered.length > 0 && filtered.every(p => selectedIds.has(p.id));
	});
	let someSelected = $derived(() => {
		const filtered = filteredProjects();
		return filtered.some(p => selectedIds.has(p.id)) && !allSelected();
	});

	function toggleSelection(projectId: number, e: Event) {
		e.preventDefault();
		e.stopPropagation();
		const newSet = new Set(selectedIds);
		if (newSet.has(projectId)) {
			newSet.delete(projectId);
		} else {
			newSet.add(projectId);
		}
		selectedIds = newSet;
	}

	function toggleSelectAll() {
		const filtered = filteredProjects();
		if (allSelected()) {
			// Deselect all filtered
			const newSet = new Set(selectedIds);
			filtered.forEach(p => newSet.delete(p.id));
			selectedIds = newSet;
		} else {
			// Select all filtered
			const newSet = new Set(selectedIds);
			filtered.forEach(p => newSet.add(p.id));
			selectedIds = newSet;
		}
	}

	function clearSelection() {
		selectedIds = new Set();
	}

	function isSelected(projectId: number): boolean {
		return selectedIds.has(projectId);
	}

	// Filter and sort projects
	let filteredProjects = $derived(() => {
		let result = projects;

		// Apply search filter
		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase();
			result = result.filter(
				(p) =>
					p.human_key.toLowerCase().includes(query) ||
					p.slug.toLowerCase().includes(query)
			);
		}

		// Apply sorting
		return [...result].sort((a, b) => {
			let comparison = 0;
			if (sortField === 'name') {
				comparison = a.human_key.localeCompare(b.human_key);
			} else if (sortField === 'date') {
				comparison = new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
			}
			return sortDirection === 'asc' ? comparison : -comparison;
		});
	});

	// New project form
	let showNewForm = $state(false);
	let projectName = $state("");
	let projectDescription = $state("");
	let projectIdentifier = $state("");
	let identifierEdited = $state(false);
	let creating = $state(false);
	let nameError = $state<string | null>(null);

	// Auto-generate identifier from name (unless manually edited)
	$effect(() => {
		if (!identifierEdited && projectName) {
			projectIdentifier = slugify(projectName);
		}
	});

	function handleIdentifierInput() {
		identifierEdited = true;
	}

	function resetForm() {
		projectName = "";
		projectDescription = "";
		projectIdentifier = "";
		identifierEdited = false;
		nameError = null;
	}

	function validateName(): boolean {
		if (!projectName.trim()) {
			nameError = "Project name is required";
			return false;
		}
		if (projectName.trim().length < 2) {
			nameError = "Project name must be at least 2 characters";
			return false;
		}
		nameError = null;
		return true;
	}

	// Use $effect for client-side data loading in Svelte 5
	$effect(() => {
		if (browser) {
			loadProjects();
		}
	});

	async function loadProjects() {
		loading = true;
		error = null;
		try {
			projects = await getProjects();
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to load projects";
		} finally {
			loading = false;
		}
	}

	async function createProject() {
		if (!validateName()) return;
		if (!projectIdentifier.trim()) return;

		creating = true;
		error = null;
		try {
			// Use identifier as the human_key for the API
			await ensureProject(projectIdentifier.trim());
			await loadProjects();
			toast.success(`Project "${projectName}" created successfully`);
			resetForm();
			showNewForm = false;
		} catch (e) {
			error = e instanceof Error ? e.message : "Failed to create project";
			toast.error(error);
		} finally {
			creating = false;
		}
	}

	// Delete project state
	let showDeleteDialog = $state(false);
	let projectToDelete = $state<Project | null>(null);
	let deleting = $state(false);
	let openDropdownId = $state<number | null>(null);

	function handleDeleteClick(e: Event, project: Project) {
		e.preventDefault();
		e.stopPropagation();
		// Close the dropdown first
		openDropdownId = null;
		// Small delay to let dropdown close animation complete
		setTimeout(() => {
			projectToDelete = project;
			showDeleteDialog = true;
		}, 50);
	}

	async function confirmDelete() {
		if (!projectToDelete) return;
		deleting = true;
		try {
			await deleteProject(projectToDelete.slug);
			toast.success(`Project "${projectToDelete.human_key}" deleted`);
			await loadProjects();
		} catch (e) {
			toast.error(
				e instanceof Error ? e.message : "Failed to delete project",
			);
		} finally {
			deleting = false;
			showDeleteDialog = false;
			projectToDelete = null;
		}
	}

	// Bulk delete state
	let showBulkDeleteDialog = $state(false);
	let bulkDeleting = $state(false);

	function handleBulkDelete() {
		showBulkDeleteDialog = true;
	}

	async function confirmBulkDelete() {
		bulkDeleting = true;
		const projectsToDelete = projects.filter(p => selectedIds.has(p.id));
		let successCount = 0;
		let failCount = 0;

		for (const project of projectsToDelete) {
			try {
				await deleteProject(project.slug);
				successCount++;
			} catch {
				failCount++;
			}
		}

		if (successCount > 0) {
			toast.success(`${successCount} project${successCount > 1 ? 's' : ''} deleted`);
		}
		if (failCount > 0) {
			toast.error(`Failed to delete ${failCount} project${failCount > 1 ? 's' : ''}`);
		}

		clearSelection();
		await loadProjects();
		bulkDeleting = false;
		showBulkDeleteDialog = false;
	}

	function handleBulkExport() {
		const projectsToExport = projects.filter(p => selectedIds.has(p.id));
		const exportData = {
			exported_at: new Date().toISOString(),
			projects: projectsToExport.map(p => ({
				human_key: p.human_key,
				slug: p.slug,
				created_at: p.created_at
			}))
		};

		const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `projects-export-${Date.now()}.json`;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);

		toast.success(`Exported ${projectsToExport.length} project${projectsToExport.length > 1 ? 's' : ''}`);
		clearSelection();
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString("en-US", {
			year: "numeric",
			month: "short",
			day: "numeric",
		});
	}
</script>

<div class="space-y-4 md:space-y-6">
	<!-- Header (scrolls away) -->
	<BlurFade delay={0}>
		<div
			class="flex flex-col sm:flex-row sm:items-center justify-between gap-4"
		>
			<div>
				<h1 class="text-xl md:text-2xl font-bold text-foreground">
					Projects
				</h1>
				<p class="text-sm md:text-base text-muted-foreground">
					Manage your agent mail projects
				</p>
			</div>
			<ShimmerButton on:click={() => (showNewForm = true)}>
				<Plus class="h-4 w-4 mr-2" />
				New Project
			</ShimmerButton>
		</div>
	</BlurFade>

	<!-- Sticky Toolbar: Select All + Search + Sort + Count -->
	<div class="sticky top-0 z-10 -mx-4 md:-mx-6 px-4 md:px-6 py-3 bg-background/95 backdrop-blur-sm border-b border-border">
		<div class="flex flex-col sm:flex-row gap-3 sm:items-center">
			<!-- Select All Checkbox -->
			{#if filteredProjects().length > 0}
				<div class="flex items-center gap-2">
					<Checkbox
						data-testid="select-all-checkbox"
						checked={allSelected()}
						indeterminate={someSelected()}
						onCheckedChange={toggleSelectAll}
						aria-label="Select all projects"
					/>
				</div>
			{/if}
			<div class="flex-1 max-w-md">
				<SearchInput
					bind:value={searchQuery}
					placeholder="Search projects..."
				/>
			</div>
			<div class="flex items-center gap-1">
				<span class="text-xs text-muted-foreground mr-2">Sort by:</span>
				<SortButton
					field="name"
					label="Name"
					currentField={sortField}
					currentDirection={sortDirection}
					onSort={handleSort}
				/>
				<SortButton
					field="date"
					label="Date"
					currentField={sortField}
					currentDirection={sortDirection}
					onSort={handleSort}
				/>
			</div>
		</div>

		<!-- Stats (always visible when not loading) -->
		{#if !loading}
			<div class="flex items-center gap-4 text-sm text-muted-foreground mt-3">
				<span>Showing {filteredProjects().length} of {projects.length} projects</span>
			</div>
		{/if}
	</div>

	<!-- Error Message -->
	{#if error}
		<BlurFade delay={100}>
			<div
				class="bg-destructive/10 border border-destructive/30 rounded-xl p-4"
			>
				<p class="text-destructive">{error}</p>
			</div>
		</BlurFade>
	{/if}

	<!-- Loading State -->
	{#if loading}
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
			{#each Array(6) as _}
				<ProjectCardSkeleton />
			{/each}
		</div>
	{:else if projects.length === 0}
		<!-- Empty State - No Projects -->
		<BlurFade delay={100}>
			<Card.Root class="p-8 md:p-12">
				<EmptyState
					title="No projects yet"
					description="Create your first project to start sending messages between agents."
					actionLabel="Create Project"
					onAction={() => (showNewForm = true)}
				>
					{#snippet icon()}
						<FolderKanban class="h-12 w-12" />
					{/snippet}
				</EmptyState>
			</Card.Root>
		</BlurFade>
	{:else if filteredProjects().length === 0}
		<!-- Empty State - No Matching Results -->
		<BlurFade delay={100}>
			<Card.Root class="p-8 md:p-12">
				<EmptyState
					title="No projects found"
					description="No projects match your search. Try a different search term."
					actionLabel="Clear search"
					onAction={() => (searchQuery = "")}
				>
					{#snippet icon()}
						<FolderKanban class="h-12 w-12" />
					{/snippet}
				</EmptyState>
			</Card.Root>
		</BlurFade>
	{:else}
		<!-- Projects Grid - Cards with hover effects -->
		<BlurFade delay={100}>
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
				{#each filteredProjects() as project, index}
					<div
						class="group relative animate-in fade-in slide-in-from-bottom-2"
						style="animation-delay: calc({index} * var(--delay-stagger)); animation-fill-mode: both;"
					>
						<!-- Selection Checkbox -->
						<div
							class="absolute top-3 left-3 z-10"
							onclick={(e: Event) => toggleSelection(project.id, e)}
						>
							<Checkbox
								data-testid="project-select-checkbox"
								checked={isSelected(project.id)}
								aria-label="Select project {project.human_key}"
								class="bg-background/80 backdrop-blur-sm"
							/>
						</div>

						<a
							href="/projects/{project.slug}"
							class="block"
						>
							<Card.Root
								class="h-full hover:shadow-lg hover:border-primary/50 transition-all duration-200 hover:-translate-y-1 {isSelected(project.id) ? 'ring-2 ring-primary border-primary' : ''}"
							>
								<Card.Content class="p-5 md:p-6 pl-12">
									<div
										class="flex items-start justify-between mb-4"
									>
										<div class="flex items-center gap-3">
											<div
												class="w-10 h-10 bg-primary/10 rounded-lg flex items-center justify-center group-hover:bg-primary/20 transition-colors"
											>
												<FolderKanban
													class="h-5 w-5 text-primary"
												/>
											</div>
											<div class="min-w-0">
												<h3
													class="font-semibold text-foreground truncate group-hover:text-primary transition-colors"
												>
													{project.human_key}
												</h3>
											</div>
										</div>
										<DropdownMenu.Root
										open={openDropdownId === project.id}
										onOpenChange={(open) => {
											openDropdownId = open ? project.id : null;
										}}
									>
											<DropdownMenu.Trigger>
												{#snippet child({ props })}
													<Button
														{...props}
														variant="ghost"
														size="icon"
														class="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity"
														onclick={(e: Event) => {
															e.preventDefault();
															e.stopPropagation();
														}}
													>
														<MoreVertical class="h-4 w-4" />
														<span class="sr-only">More options</span>
													</Button>
												{/snippet}
											</DropdownMenu.Trigger>
											<DropdownMenu.Content align="end">
												<DropdownMenu.Item
													class="text-destructive focus:text-destructive"
													onclick={(e: Event) =>
														handleDeleteClick(
															e,
															project,
														)}
												>
													<Trash2 class="h-4 w-4 mr-2" />
													Delete
												</DropdownMenu.Item>
											</DropdownMenu.Content>
										</DropdownMenu.Root>
									</div>

									<div
										class="flex items-center gap-2 text-sm text-muted-foreground"
									>
										<Calendar class="h-4 w-4" />
										<span
											>Created {formatDate(
												project.created_at,
											)}</span
										>
									</div>
								</Card.Content>
							</Card.Root>
						</a>
					</div>
				{/each}
			</div>
		</BlurFade>
	{/if}
</div>

<!-- New Project Dialog -->
<Dialog.Root bind:open={showNewForm}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>Create New Project</Dialog.Title>
			<Dialog.Description>
				Create a new project to organize agents and messages.
			</Dialog.Description>
		</Dialog.Header>
		<form
			onsubmit={(e) => {
				e.preventDefault();
				createProject();
			}}
			class="space-y-4"
		>
			<!-- Project Name (required) -->
			<div class="space-y-2">
				<Label for="projectName">
					Project Name <span class="text-destructive">*</span>
				</Label>
				<Input
					id="projectName"
					type="text"
					bind:value={projectName}
					placeholder="My Awesome Project"
					class={nameError ? "border-destructive" : ""}
					aria-invalid={!!nameError}
					aria-describedby={nameError ? "name-error" : undefined}
				/>
				{#if nameError}
					<p id="name-error" class="text-sm text-destructive">{nameError}</p>
				{/if}
			</div>

			<!-- Description (optional) -->
			<div class="space-y-2">
				<Label for="projectDescription">Description</Label>
				<Textarea
					id="projectDescription"
					bind:value={projectDescription}
					placeholder="A brief description of this project..."
					rows={2}
				/>
			</div>

			<!-- Identifier (auto-generated from name) -->
			<div class="space-y-2">
				<Label for="projectIdentifier" class="flex items-center gap-2">
					Identifier
					<Badge variant="secondary" class="text-xs">Auto-generated</Badge>
				</Label>
				<div class="flex gap-2">
					<Input
						id="projectIdentifier"
						type="text"
						bind:value={projectIdentifier}
						oninput={handleIdentifierInput}
						placeholder="my-awesome-project"
						class="font-mono text-sm"
					/>
					{#if identifierEdited}
						<Button
							type="button"
							variant="ghost"
							size="icon"
							title="Reset to auto-generated"
							onclick={() => {
								identifierEdited = false;
								projectIdentifier = slugify(projectName);
							}}
						>
							<Pencil class="h-4 w-4" />
						</Button>
					{/if}
				</div>
				<p class="text-xs text-muted-foreground">
					Used in URLs and API calls. Can be edited manually.
				</p>
			</div>

			<Dialog.Footer class="flex-col sm:flex-row gap-2">
				<Button
					type="button"
					variant="outline"
					onclick={() => {
						showNewForm = false;
						resetForm();
					}}
					class="w-full sm:w-auto"
				>
					Cancel
				</Button>
				<Button
					type="submit"
					disabled={creating || !projectName.trim() || !projectIdentifier.trim()}
					class="w-full sm:w-auto"
				>
					{creating ? "Creating..." : "Create Project"}
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>

<!-- Delete Project Confirmation Dialog -->
<AlertDialog.Root bind:open={showDeleteDialog}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Delete Project</AlertDialog.Title>
			<AlertDialog.Description>
				Are you sure you want to delete <strong
					>{projectToDelete?.human_key}</strong
				>? This action cannot be undone. All agents, messages, and data
				associated with this project will be permanently deleted.
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel
				onclick={() => {
					showDeleteDialog = false;
					projectToDelete = null;
				}}
			>
				Cancel
			</AlertDialog.Cancel>
			<AlertDialog.Action
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
				disabled={deleting}
				onclick={confirmDelete}
			>
				{deleting ? "Deleting..." : "Delete Project"}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

<!-- Bulk Delete Confirmation Dialog -->
<AlertDialog.Root bind:open={showBulkDeleteDialog}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Delete {selectedCount} Projects</AlertDialog.Title>
			<AlertDialog.Description>
				Are you sure you want to delete {selectedCount} project{selectedCount > 1 ? 's' : ''}?
				This action cannot be undone. All agents, messages, and data
				associated with these projects will be permanently deleted.
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel
				onclick={() => {
					showBulkDeleteDialog = false;
				}}
			>
				Cancel
			</AlertDialog.Cancel>
			<AlertDialog.Action
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
				disabled={bulkDeleting}
				onclick={confirmBulkDelete}
			>
				{bulkDeleting ? "Deleting..." : `Delete ${selectedCount} Project${selectedCount > 1 ? 's' : ''}`}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

<!-- Bulk Action Bar -->
<BulkActionBar
	{selectedCount}
	onClear={clearSelection}
	onDelete={handleBulkDelete}
	onExport={handleBulkExport}
/>

<style>
	/* Staggered animation keyframes */
	@keyframes fade-in {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes slide-in-from-bottom-2 {
		from {
			transform: translateY(8px);
		}
		to {
			transform: translateY(0);
		}
	}

	.animate-in {
		animation:
			fade-in 300ms ease-out,
			slide-in-from-bottom-2 300ms ease-out;
	}

	/* Respect reduced motion */
	@media (prefers-reduced-motion: reduce) {
		.animate-in {
			animation: none;
		}
	}
</style>
