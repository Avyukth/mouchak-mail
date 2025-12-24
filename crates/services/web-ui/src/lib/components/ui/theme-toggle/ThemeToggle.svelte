<script lang="ts">
	import { setMode, mode } from 'mode-watcher';
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

	// Map mode-watcher's mode to our options
	let currentMode = $derived<ThemeMode>($mode === undefined ? 'system' : $mode);

	// Calculate indicator position and width
	let indicatorStyle = $derived.by(() => {
		const idx = options.findIndex((o) => o.value === currentMode);
		// Base width for icon-only: 36px, expanded with label: ~70px
		const iconOnlyWidth = 36;
		const expandedWidth = 70;

		// Calculate left position (3px padding + sum of previous option widths + gaps)
		let left = 3;
		for (let i = 0; i < idx; i++) {
			left += iconOnlyWidth + 2; // 2px gap
		}

		const width = currentMode === options[idx]?.value ? expandedWidth : iconOnlyWidth;
		return `left: ${left}px; width: ${width}px;`;
	});

	function handleSelect(value: ThemeMode) {
		setMode(value);
	}
</script>

<div class="theme-toggle" role="radiogroup" aria-label="Theme selection">
	<div class="theme-indicator" style={indicatorStyle}></div>

	{#each options as option}
		{@const isActive = currentMode === option.value}
		<button
			class="theme-option"
			data-active={isActive}
			role="radio"
			aria-checked={isActive}
			aria-label="{option.label} theme"
			onclick={() => handleSelect(option.value)}
		>
			<option.icon class="h-4 w-4" />
			{#if isActive}
				<span class="theme-label">{option.label}</span>
			{/if}
		</button>
	{/each}
</div>
