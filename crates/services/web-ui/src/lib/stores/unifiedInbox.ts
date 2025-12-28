import { derived, get, writable } from 'svelte/store';

export type SortBy = 'newest' | 'oldest' | 'sender' | 'longest';
export type ViewMode = 'split' | 'list';

export interface Filter {
	project: string;
	sender: string;
	recipient: string;
	importance: string;
	hasThread: string;
}

export interface Message {
	id: number;
	project_slug: string;
	sender_name: string;
	subject: string;
	body_md?: string | null;
	excerpt?: string | null;
	importance?: string;
	thread_id?: string | null;
	recipients?: string[];
	recipient_names?: string[];
	created_ts?: string;
	is_read?: boolean;
}

const defaultFilters: Filter = {
	project: '',
	sender: '',
	recipient: '',
	importance: '',
	hasThread: ''
};

export const allMessages = writable<Message[]>([]);
export const searchQuery = writable('');
export const showFilters = writable(false);
export const sortBy = writable<SortBy>('newest');
export const viewMode = writable<ViewMode>('split');
export const isFullscreen = writable(false);
export const selectedMessage = writable<Message | null>(null);
export const selectedMessages = writable<number[]>([]);
export const filters = writable<Filter>({ ...defaultFilters });
export const autoRefreshEnabled = writable(true);
export const isRefreshing = writable(false);

const getRecipients = (message: Message): string[] =>
	message.recipients ?? message.recipient_names ?? [];

const getSearchText = (message: Message): string => {
	const parts = [
		message.subject,
		message.body_md ?? '',
		message.sender_name,
		...getRecipients(message)
	];
	return parts.join(' ').toLowerCase();
};

const messageDateKey = (message: Message): number => {
	if (message.created_ts) {
		const parsed = Date.parse(message.created_ts);
		if (!Number.isNaN(parsed)) {
			return parsed;
		}
	}
	return message.id;
};

export const filteredMessages = derived(
	[allMessages, searchQuery, filters, sortBy],
	([$messages, $query, $filters, $sort]) => {
		const query = $query.trim().toLowerCase();
		const recipientFilter = $filters.recipient.trim().toLowerCase();

		let result = $messages.filter((message) => {
			if (query && !getSearchText(message).includes(query)) {
				return false;
			}
			if ($filters.project && message.project_slug !== $filters.project) {
				return false;
			}
			if ($filters.sender && message.sender_name !== $filters.sender) {
				return false;
			}
			if (recipientFilter) {
				const recipients = getRecipients(message).map((name) => name.toLowerCase());
				if (!recipients.includes(recipientFilter)) {
					return false;
				}
			}
			if ($filters.importance && message.importance !== $filters.importance) {
				return false;
			}
			if ($filters.hasThread === 'yes' && !message.thread_id) {
				return false;
			}
			if ($filters.hasThread === 'no' && message.thread_id) {
				return false;
			}
			return true;
		});

		switch ($sort) {
			case 'oldest':
				result = [...result].sort((a, b) => messageDateKey(a) - messageDateKey(b));
				break;
			case 'sender':
				result = [...result].sort((a, b) =>
					a.sender_name.localeCompare(b.sender_name)
				);
				break;
			case 'longest':
				result = [...result].sort((a, b) => {
					const aLen = (a.subject?.length ?? 0) + (a.body_md?.length ?? 0);
					const bLen = (b.subject?.length ?? 0) + (b.body_md?.length ?? 0);
					return bLen - aLen;
				});
				break;
			default:
				result = [...result].sort((a, b) => messageDateKey(b) - messageDateKey(a));
				break;
		}

		return result;
	}
);

export const uniqueProjects = derived(allMessages, ($messages) =>
	Array.from(new Set($messages.map((message) => message.project_slug)))
);

export const uniqueSenders = derived(allMessages, ($messages) =>
	Array.from(new Set($messages.map((message) => message.sender_name)))
);

export const uniqueRecipients = derived(allMessages, ($messages) => {
	const recipients = $messages.flatMap((message) => getRecipients(message));
	return Array.from(new Set(recipients));
});

export const filtersActive = derived(filters, ($filters) =>
	Object.values($filters).some((value) => value !== '')
);

export function clearFilters(): void {
	filters.set({ ...defaultFilters });
}

export function toggleSelectAll(): void {
	const current = get(selectedMessages);
	const ids = get(filteredMessages).map((message) => message.id);
	const allSelected = ids.length > 0 && ids.every((id) => current.includes(id));
	selectedMessages.set(allSelected ? [] : ids);
}

export function selectNextMessage(): void {
	const messages = get(filteredMessages);
	if (messages.length === 0) {
		return;
	}

	const current = get(selectedMessage);
	const currentIndex = current
		? messages.findIndex((message) => message.id === current.id)
		: -1;
	const nextIndex = Math.min(currentIndex + 1, messages.length - 1);
	selectedMessage.set(messages[nextIndex]);
}

export function selectPreviousMessage(): void {
	const messages = get(filteredMessages);
	if (messages.length === 0) {
		return;
	}

	const current = get(selectedMessage);
	const currentIndex = current
		? messages.findIndex((message) => message.id === current.id)
		: messages.length;
	const prevIndex = Math.max(currentIndex - 1, 0);
	selectedMessage.set(messages[prevIndex]);
}

// Derived unread count for sidebar badge
export const unreadCount = derived(allMessages, ($messages) =>
	$messages.filter((m) => !m.is_read).length
);

// Mark messages as read - updates local state and syncs with backend
export async function markMessagesAsRead(ids: number[]): Promise<void> {
	if (ids.length === 0) return;

	// Get messages to mark
	const messages = get(allMessages);
	const messagesToMark = messages.filter((m) => ids.includes(m.id));

	// Update local state immediately for responsive UI
	allMessages.update((msgs) =>
		msgs.map((message) =>
			ids.includes(message.id) ? { ...message, is_read: true } : message
		)
	);

	// Sync with backend - dynamically import to avoid circular dependency
	try {
		const { dataProvider } = await import('$lib/data');
		await Promise.allSettled(
			messagesToMark.map((msg) => {
				// Use first recipient as the agent reading the message
				const recipients = msg.recipients ?? msg.recipient_names ?? [];
				const agentName = recipients[0];
				if (!agentName || !msg.project_slug) return Promise.resolve();
				return dataProvider.markMessageRead(msg.project_slug, agentName, msg.id);
			})
		);
	} catch {
		// Backend sync failed silently - local state is already updated
	}
}
