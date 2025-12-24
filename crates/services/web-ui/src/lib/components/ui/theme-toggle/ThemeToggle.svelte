<script lang="ts">
	import { setMode, userPrefersMode } from 'mode-watcher';
	import Sun from 'lucide-svelte/icons/sun';
	import Moon from 'lucide-svelte/icons/moon';
	import Monitor from 'lucide-svelte/icons/monitor';
	import type { ComponentType } from 'svelte';

	type ThemeMode = 'light' | 'dark' | 'system';

	interface ThemeOption {
		value: ThemeMode;
		icon: ComponentType;
		label: string;
	}

	const options: ThemeOption[] = [
		{ value: 'dark', icon: Moon, label: 'Dark' },
		{ value: 'light', icon: Sun, label: 'Light' },
		{ value: 'system', icon: Monitor, label: 'System' }
	];

	// Icon-only width and expanded (with label) width
	const ICON_WIDTH = 32; // px
	const EXPANDED_WIDTH = 80; // px
	const GAP = 2; // px between buttons
	const PADDING = 4; // px container padding

	// Use userPrefersMode to track the user's preference (includes 'system')
	let currentMode = $derived<ThemeMode>(userPrefersMode.current ?? 'system');

	// Calculate indicator position and width dynamically
	let indicatorStyle = $derived.by(() => {
		const idx = options.findIndex((o) => o.value === currentMode);
		if (idx === -1) return 'opacity: 0;';

		// Calculate offset from first button position (left-1 handles base padding)
		let offset = 0;
		for (let i = 0; i < idx; i++) {
			offset += ICON_WIDTH + GAP;
		}

		return `transform: translateX(${offset}px); width: ${EXPANDED_WIDTH}px;`;
	});

	function handleSelect(value: ThemeMode) {
		setMode(value);
	}
</script>

<div class="flex items-center">
	<div
		class="relative flex gap-0.5 overflow-hidden rounded-lg border border-border bg-muted/50 p-1"
		role="radiogroup"
		aria-label="Theme selection"
	>
		<div
			class="absolute left-1 top-1 h-6 rounded-md bg-zinc-900 dark:bg-zinc-100 shadow-sm transition-all duration-300 ease-out"
			style={indicatorStyle}
		></div>

		{#each options as option}
			{@const isActive = currentMode === option.value}
			<button
				class="relative z-10 flex cursor-pointer items-center justify-center rounded-md h-6 gap-1 px-2 transition-all duration-200
					{isActive
					? 'text-zinc-100 dark:text-zinc-900'
					: 'text-zinc-500 dark:text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-300'}"
				style="width: {isActive ? EXPANDED_WIDTH : ICON_WIDTH}px;"
				title={option.label}
				role="radio"
				aria-checked={isActive}
				aria-label="Switch to {option.label} theme"
				onclick={() => handleSelect(option.value)}
			>
				<span class="flex-shrink-0">
					<option.icon class="h-4 w-4" />
				</span>
				{#if isActive}
					<span class="font-mono text-2xs uppercase tracking-tight whitespace-nowrap">
						{option.label}
					</span>
				{/if}
			</button>
		{/each}
	</div>
</div>
