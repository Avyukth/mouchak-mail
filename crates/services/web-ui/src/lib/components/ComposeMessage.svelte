<script lang="ts">
	import { sendMessage, type Agent } from '$lib/api/client';

	interface Props {
		projectSlug: string;
		senderName: string;
		agents: Agent[];
		replyTo?: {
			threadId?: string;
			subject: string;
			recipientName?: string;
		};
		onClose: () => void;
		onSent: () => void;
	}

	let {
		projectSlug,
		senderName,
		agents,
		replyTo,
		onClose,
		onSent
	}: Props = $props();

	// Form state
	let recipients = $state<string[]>(replyTo?.recipientName ? [replyTo.recipientName] : []);
	let subject = $state(replyTo ? `Re: ${replyTo.subject.replace(/^Re: /, '')}` : '');
	let body = $state('');
	let importance = $state<'low' | 'normal' | 'high'>('normal');
	let ackRequired = $state(false);
	let threadId = $state(replyTo?.threadId || '');

	let sending = $state(false);
	let error = $state<string | null>(null);

	// Available recipients (all agents except sender)
	let availableRecipients = $derived(
		agents.filter(a => a.name !== senderName)
	);

	function toggleRecipient(agentName: string) {
		if (recipients.includes(agentName)) {
			recipients = recipients.filter(r => r !== agentName);
		} else {
			recipients = [...recipients, agentName];
		}
	}

	async function handleSubmit() {
		if (recipients.length === 0) {
			error = 'Please select at least one recipient';
			return;
		}
		if (!subject.trim()) {
			error = 'Please enter a subject';
			return;
		}
		if (!body.trim()) {
			error = 'Please enter a message body';
			return;
		}

		sending = true;
		error = null;

		try {
			await sendMessage(
				projectSlug,
				senderName,
				recipients,
				subject.trim(),
				body.trim(),
				threadId || undefined,
				importance,
				ackRequired
			);
			onSent();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to send message';
		} finally {
			sending = false;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			onClose();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex flex-col h-full max-h-[90vh]">
	<!-- Header -->
	<div class="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
		<h2 class="text-lg font-semibold text-gray-900 dark:text-white">
			{replyTo ? 'Reply' : 'New Message'}
		</h2>
		<button
			onclick={onClose}
			class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
		>
			<span class="text-xl">×</span>
		</button>
	</div>

	<!-- Form -->
	<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="flex-1 overflow-y-auto p-4 space-y-4">
		<!-- From (readonly) -->
		<div>
			<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
				From
			</label>
			<div class="px-4 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg text-gray-700 dark:text-gray-300">
				{senderName}
				<span class="text-gray-500 dark:text-gray-400 text-sm ml-2">({projectSlug})</span>
			</div>
		</div>

		<!-- Recipients -->
		<div>
			<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
				To *
			</label>
			{#if availableRecipients.length === 0}
				<p class="text-sm text-gray-500 dark:text-gray-400 italic">
					No other agents in this project. Register more agents to send messages.
				</p>
			{:else}
				<div class="flex flex-wrap gap-2">
					{#each availableRecipients as agent}
						<button
							type="button"
							onclick={() => toggleRecipient(agent.name)}
							class="px-3 py-1.5 rounded-full text-sm transition-colors {recipients.includes(agent.name)
								? 'bg-primary-600 text-white'
								: 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'}"
						>
							{agent.name}
						</button>
					{/each}
				</div>
				{#if recipients.length > 0}
					<p class="mt-2 text-sm text-gray-500 dark:text-gray-400">
						Selected: {recipients.join(', ')}
					</p>
				{/if}
			{/if}
		</div>

		<!-- Subject -->
		<div>
			<label for="subject" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
				Subject *
			</label>
			<input
				id="subject"
				type="text"
				bind:value={subject}
				placeholder="Enter subject..."
				class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
			/>
		</div>

		<!-- Thread ID (optional) -->
		{#if !replyTo}
			<div>
				<label for="threadId" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
					Thread ID
					<span class="text-gray-400 font-normal">(optional)</span>
				</label>
				<input
					id="threadId"
					type="text"
					bind:value={threadId}
					placeholder="Leave empty for new thread"
					class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
				/>
			</div>
		{/if}

		<!-- Body -->
		<div>
			<label for="body" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
				Message *
				<span class="text-gray-400 font-normal">(Markdown supported)</span>
			</label>
			<textarea
				id="body"
				bind:value={body}
				rows="8"
				placeholder="Write your message..."
				class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent resize-none font-mono text-sm"
			></textarea>
		</div>

		<!-- Options -->
		<div class="flex flex-wrap gap-4">
			<!-- Importance -->
			<div>
				<label for="importance" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
					Importance
				</label>
				<select
					id="importance"
					bind:value={importance}
					class="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
				>
					<option value="low">Low</option>
					<option value="normal">Normal</option>
					<option value="high">High</option>
				</select>
			</div>

			<!-- Ack Required -->
			<div class="flex items-center pt-6">
				<label class="flex items-center gap-2 cursor-pointer">
					<input
						type="checkbox"
						bind:checked={ackRequired}
						class="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
					/>
					<span class="text-sm text-gray-700 dark:text-gray-300">
						Require acknowledgment
					</span>
				</label>
			</div>
		</div>

		<!-- Error -->
		{#if error}
			<div class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
				<p class="text-red-700 dark:text-red-400 text-sm">{error}</p>
			</div>
		{/if}
	</form>

	<!-- Footer -->
	<div class="p-4 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
		<button
			type="button"
			onclick={onClose}
			class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
		>
			Cancel
		</button>
		<button
			onclick={handleSubmit}
			disabled={sending || recipients.length === 0}
			class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
		>
			{#if sending}
				<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
				<span>Sending...</span>
			{:else}
				<span>Send Message</span>
			{/if}
		</button>
	</div>
</div>
