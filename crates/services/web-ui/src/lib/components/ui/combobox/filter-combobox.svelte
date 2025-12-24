<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import * as Command from '$lib/components/ui/command/index.js';
	import Check from 'lucide-svelte/icons/check';
	import ChevronsUpDown from 'lucide-svelte/icons/chevrons-up-down';

	interface Props {
		value: string;
		onValueChange: (value: string) => void;
		options: string[];
		placeholder?: string;
		searchPlaceholder?: string;
		emptyMessage?: string;
		class?: string;
	}

	let {
		value,
		onValueChange,
		options,
		placeholder = 'Select...',
		searchPlaceholder = 'Search...',
		emptyMessage = 'No results found.',
		class: className
	}: Props = $props();

	let open = $state(false);

	function handleSelect(selectedValue: string) {
		onValueChange(selectedValue === value ? '' : selectedValue);
		open = false;
	}

	const displayValue = $derived(value || placeholder);
</script>

<Popover.Root bind:open>
	<Popover.Trigger>
		{#snippet child({ props })}
			<Button
				variant="outline"
				role="combobox"
				aria-expanded={open}
				class={cn('w-full justify-between', className)}
				{...props}
			>
				<span class="truncate {!value ? 'text-muted-foreground' : ''}">{displayValue}</span>
				<ChevronsUpDown class="ml-2 h-4 w-4 shrink-0 opacity-50" />
			</Button>
		{/snippet}
	</Popover.Trigger>
	<Popover.Content class="w-[--bits-popover-anchor-width] p-0" align="start">
		<Command.Root>
			<Command.Input placeholder={searchPlaceholder} class="h-9" />
			<Command.List class="max-h-[200px]">
				<Command.Empty>{emptyMessage}</Command.Empty>
				<Command.Group>
					<Command.Item value="" onSelect={() => handleSelect('')}>
						<Check class={cn('mr-2 h-4 w-4', value === '' ? 'opacity-100' : 'opacity-0')} />
						<span class="text-muted-foreground">All</span>
					</Command.Item>
					{#each options as option, i (i)}
						<Command.Item value={option} onSelect={() => handleSelect(option)}>
							<Check class={cn('mr-2 h-4 w-4', value === option ? 'opacity-100' : 'opacity-0')} />
							{option}
						</Command.Item>
					{/each}
				</Command.Group>
			</Command.List>
		</Command.Root>
	</Popover.Content>
</Popover.Root>
