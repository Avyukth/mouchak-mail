<script lang="ts">
	import { fly } from 'svelte/transition';
	import X from 'lucide-svelte/icons/x';
	import Trash2 from 'lucide-svelte/icons/trash-2';
	import Download from 'lucide-svelte/icons/download';
	import { Button } from '$lib/components/ui/button';
	import { cn } from '$lib/utils';

	interface Props {
		selectedCount: number;
		onClear: () => void;
		onDelete: () => void;
		onExport?: () => void;
		class?: string;
	}

	let { selectedCount, onClear, onDelete, onExport, class: className = '' }: Props = $props();

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onClear();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if selectedCount > 0}
	<div
		data-testid="bulk-action-bar"
		role="toolbar"
		aria-label="Bulk actions"
		transition:fly={{ y: 20, duration: 200 }}
		class={cn(
			'fixed bottom-4 left-1/2 -translate-x-1/2 z-50',
			'bg-background border border-border rounded-xl shadow-lg',
			'px-4 py-3 flex items-center gap-4',
			className
		)}
	>
		<span class="text-sm font-medium text-foreground">
			{selectedCount} selected
		</span>

		<div class="h-4 w-px bg-border"></div>

		<div class="flex items-center gap-2">
			{#if onExport}
				<Button
					data-testid="bulk-export-button"
					variant="ghost"
					size="sm"
					onclick={onExport}
				>
					<Download class="h-4 w-4 mr-1.5" />
					Export
				</Button>
			{/if}
			<Button
				data-testid="bulk-delete-button"
				variant="ghost"
				size="sm"
				class="text-destructive hover:text-destructive hover:bg-destructive/10"
				onclick={onDelete}
			>
				<Trash2 class="h-4 w-4 mr-1.5" />
				Delete
			</Button>
		</div>

		<Button
			data-testid="clear-selection-button"
			variant="ghost"
			size="icon"
			onclick={onClear}
			aria-label="Clear selection"
			class="h-8 w-8"
		>
			<X class="h-4 w-4" />
		</Button>
	</div>
{/if}
