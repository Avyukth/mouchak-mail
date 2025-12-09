<script lang="ts">
	import { onMount } from 'svelte';
	import { checkHealth, getProjects, type Project } from '$lib/api/client';

	let healthStatus = $state<string>('checking...');
	let projects = $state<Project[]>([]);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			const health = await checkHealth();
			healthStatus = health.status;

			const projectList = await getProjects();
			projects = projectList;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to connect to backend';
			healthStatus = 'offline';
		}
	});
</script>

<div class="space-y-6">
	<div>
		<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Dashboard</h1>
		<p class="text-gray-600 dark:text-gray-400">Welcome to MCP Agent Mail</p>
	</div>

	<!-- Status Cards -->
	<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
		<!-- Backend Status -->
		<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
			<div class="flex items-center gap-3">
				<div
					class="w-3 h-3 rounded-full"
					class:bg-green-500={healthStatus === 'ok'}
					class:bg-yellow-500={healthStatus === 'checking...'}
					class:bg-red-500={healthStatus === 'offline'}
				></div>
				<h3 class="font-semibold text-gray-900 dark:text-white">Backend Status</h3>
			</div>
			<p class="mt-2 text-2xl font-bold text-gray-700 dark:text-gray-300 capitalize">
				{healthStatus}
			</p>
		</div>

		<!-- Projects Count -->
		<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
			<div class="flex items-center gap-3">
				<span class="text-2xl">📁</span>
				<h3 class="font-semibold text-gray-900 dark:text-white">Projects</h3>
			</div>
			<p class="mt-2 text-2xl font-bold text-primary-600 dark:text-primary-400">
				{projects.length}
			</p>
		</div>

		<!-- Quick Actions -->
		<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
			<h3 class="font-semibold text-gray-900 dark:text-white mb-3">Quick Actions</h3>
			<div class="space-y-2">
				<a
					href="/projects"
					class="block px-4 py-2 bg-primary-100 dark:bg-primary-900 text-primary-700 dark:text-primary-300 rounded-lg hover:bg-primary-200 dark:hover:bg-primary-800 transition-colors"
				>
					View Projects →
				</a>
				<a
					href="/inbox"
					class="block px-4 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
				>
					Check Inbox →
				</a>
			</div>
		</div>
	</div>

	{#if error}
		<div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl p-4">
			<p class="text-red-700 dark:text-red-400">
				<strong>Error:</strong> {error}
			</p>
			<p class="text-sm text-red-600 dark:text-red-500 mt-1">
				Make sure the backend is running on port 8000
			</p>
		</div>
	{/if}

	<!-- Recent Projects -->
	{#if projects.length > 0}
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700">
			<div class="p-4 border-b border-gray-200 dark:border-gray-700">
				<h2 class="text-lg font-semibold text-gray-900 dark:text-white">Recent Projects</h2>
			</div>
			<ul class="divide-y divide-gray-200 dark:divide-gray-700">
				{#each projects.slice(0, 5) as project}
					<li class="p-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
						<a href="/projects/{project.slug}" class="block">
							<p class="font-medium text-gray-900 dark:text-white">{project.slug}</p>
							<p class="text-sm text-gray-500 dark:text-gray-400 truncate">{project.human_key}</p>
						</a>
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>
