<!--
  @component
  ProjectList - Enhanced recent projects list with metadata.

  @example
  ```svelte
  <ProjectList projects={recentProjects} />
  ```
-->
<script lang="ts">
	import * as Card from '$lib/components/ui/card/index.js';
	import FolderKanban from 'lucide-svelte/icons/folder-kanban';
	import ChevronRight from 'lucide-svelte/icons/chevron-right';
	import { formatRelativeTime } from '$lib/utils/date';

	interface ProjectItem {
		id?: number;
		slug: string;
		human_key: string;
		agentCount?: number;
		created_at?: string;
	}

	interface Props {
		projects: ProjectItem[];
		showViewAll?: boolean;
	}

	let { projects, showViewAll = true }: Props = $props();
</script>

<Card.Root data-testid="project-list">
	<Card.Header class="pb-2">
		<div class="flex items-center justify-between">
			<Card.Title class="text-base">Recent Projects</Card.Title>
			{#if showViewAll}
				<a
					href="/projects"
					class="text-sm text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 rounded"
				>
					View all →
				</a>
			{/if}
		</div>
	</Card.Header>
	<Card.Content class="space-y-1 pt-0">
		{#if projects.length === 0}
			<div class="py-8 text-center text-muted-foreground">
				<FolderKanban class="h-8 w-8 mx-auto mb-2 opacity-50" />
				<p class="text-sm">No projects yet</p>
				<a href="/projects" class="text-sm text-primary hover:underline mt-1 inline-block">
					Create your first project
				</a>
			</div>
		{:else}
			{#each projects as project, index}
				<a
					href="/projects/{project.slug}"
					class="flex items-center gap-3 p-3 -mx-3 rounded-lg hover:bg-muted/50 transition-colors group focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
					style="animation-delay: {index * 50}ms"
					data-testid="project-item"
				>
					<div
						class="h-9 w-9 rounded-md bg-primary/10 flex items-center justify-center shrink-0"
					>
						<FolderKanban class="h-4 w-4 text-primary" />
					</div>
					<div class="flex-1 min-w-0">
						<p
							class="font-medium truncate group-hover:text-primary transition-colors text-foreground"
						>
							{project.human_key}
						</p>
						<p class="text-xs text-muted-foreground">
							{#if project.agentCount !== undefined}
								{project.agentCount}
								{project.agentCount === 1 ? 'agent' : 'agents'}
							{/if}
							{#if project.created_at}
								{#if project.agentCount !== undefined}
									·
								{/if}
								{formatRelativeTime(project.created_at)}
							{/if}
						</p>
					</div>
					<ChevronRight
						class="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
					/>
				</a>
			{/each}
		{/if}
	</Card.Content>
</Card.Root>
