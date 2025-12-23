<script lang="ts">
	import { goto } from '$app/navigation';
	import { toggleMode } from 'mode-watcher';
	import * as Command from '$lib/components/ui/command/index.js';
	import Home from 'lucide-svelte/icons/home';
	import FolderKanban from 'lucide-svelte/icons/folder-kanban';
	import Bot from 'lucide-svelte/icons/bot';
	import Mail from 'lucide-svelte/icons/mail';
	import Inbox from 'lucide-svelte/icons/inbox';
	import SunMoon from 'lucide-svelte/icons/sun-moon';
	import Settings from 'lucide-svelte/icons/settings';
	import Search from 'lucide-svelte/icons/search';
	import FileText from 'lucide-svelte/icons/file-text';
	import Fuse from 'fuse.js';

	let open = $state(false);
	let searchValue = $state('');

	// Navigation items
	const navigationItems = [
		{ id: 'home', label: 'Dashboard', href: '/', icon: Home, shortcut: '⌘D' },
		{ id: 'projects', label: 'Projects', href: '/projects', icon: FolderKanban, shortcut: '⌘P' },
		{ id: 'agents', label: 'Agents', href: '/agents', icon: Bot, shortcut: '⌘A' },
		{ id: 'mail', label: 'Mail', href: '/mail', icon: Mail, shortcut: '⌘M' },
		{ id: 'inbox', label: 'Inbox', href: '/inbox', icon: Inbox, shortcut: '⌘I' }
	];

	// Action items
	const actionItems = [
		{ id: 'theme', label: 'Toggle Theme', action: () => toggleMode(), icon: SunMoon, shortcut: '⌘T' },
		{ id: 'settings', label: 'Settings', action: () => goto('/settings'), icon: Settings, shortcut: '⌘,' }
	];

	// All searchable items
	const allItems = [
		...navigationItems.map((item) => ({ ...item, type: 'navigation' as const })),
		...actionItems.map((item) => ({ ...item, type: 'action' as const }))
	];

	// Fuse.js for fuzzy search
	const fuse = new Fuse(allItems, {
		keys: ['label', 'id'],
		threshold: 0.4,
		includeScore: true
	});

	// Filtered items based on search
	let filteredItems = $derived.by(() => {
		if (!searchValue.trim()) {
			return allItems;
		}
		return fuse.search(searchValue).map((result) => result.item);
	});

	let filteredNavigation = $derived(filteredItems.filter((item) => item.type === 'navigation'));
	let filteredActions = $derived(filteredItems.filter((item) => item.type === 'action'));

	// Handle item selection
	function handleSelect(item: (typeof allItems)[0]) {
		if (item.type === 'navigation' && 'href' in item) {
			goto(item.href);
		} else if (item.type === 'action' && 'action' in item) {
			item.action();
		}
		open = false;
		searchValue = '';
	}

	// Keyboard shortcut listener
	$effect(() => {
		function handleKeydown(e: KeyboardEvent) {
			// Cmd+K or Ctrl+K to open
			if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
				e.preventDefault();
				open = !open;
			}

			// Forward slash to open (when not in input)
			if (e.key === '/' && !isInputFocused()) {
				e.preventDefault();
				open = true;
			}

			// Escape to close
			if (e.key === 'Escape' && open) {
				open = false;
				searchValue = '';
			}
		}

		function handleCustomOpen() {
			open = true;
		}

		window.addEventListener('keydown', handleKeydown);
		window.addEventListener('open-command-palette', handleCustomOpen);

		return () => {
			window.removeEventListener('keydown', handleKeydown);
			window.removeEventListener('open-command-palette', handleCustomOpen);
		};
	});

	function isInputFocused(): boolean {
		const activeElement = document.activeElement;
		return (
			activeElement instanceof HTMLInputElement ||
			activeElement instanceof HTMLTextAreaElement ||
			activeElement?.getAttribute('contenteditable') === 'true'
		);
	}
</script>

<Command.Dialog bind:open>
	<Command.Input placeholder="Type a command or search..." bind:value={searchValue} />
	<Command.List>
		<Command.Empty>
			<div class="flex flex-col items-center gap-2 py-6 text-muted-foreground">
				<Search class="h-8 w-8 opacity-50" />
				<p>No results found for "{searchValue}"</p>
			</div>
		</Command.Empty>

		{#if filteredNavigation.length > 0}
			<Command.Group heading="Navigation">
				{#each filteredNavigation as item (item.id)}
					<Command.Item value={item.id} onSelect={() => handleSelect(item)}>
						<item.icon class="mr-2 h-4 w-4" />
						<span>{item.label}</span>
						{#if item.shortcut}
							<Command.Shortcut>{item.shortcut}</Command.Shortcut>
						{/if}
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}

		{#if filteredActions.length > 0}
			<Command.Separator />
			<Command.Group heading="Actions">
				{#each filteredActions as item (item.id)}
					<Command.Item value={item.id} onSelect={() => handleSelect(item)}>
						<item.icon class="mr-2 h-4 w-4" />
						<span>{item.label}</span>
						{#if item.shortcut}
							<Command.Shortcut>{item.shortcut}</Command.Shortcut>
						{/if}
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
	</Command.List>
</Command.Dialog>
