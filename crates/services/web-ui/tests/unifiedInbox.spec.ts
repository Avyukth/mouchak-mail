import { test, expect } from '@playwright/test';
import { get } from 'svelte/store';
import {
	allMessages,
	clearFilters,
	filters,
	filteredMessages,
	searchQuery,
	selectNextMessage,
	selectPreviousMessage,
	selectedMessage,
	selectedMessages,
	sortBy,
	toggleSelectAll,
	uniqueProjects,
	uniqueRecipients,
	uniqueSenders,
	type Message
} from '../src/lib/stores/unifiedInbox';

const defaultFilters = {
	project: '',
	sender: '',
	recipient: '',
	importance: '',
	hasThread: ''
};

const sampleMessages: Message[] = [
	{
		id: 1,
		project_slug: 'alpha',
		sender_name: 'Alice',
		subject: 'Hello',
		body_md: 'first message',
		importance: 'normal',
		thread_id: null,
		recipients: ['Bob'],
		created_ts: '2025-01-01T10:00:00Z'
	},
	{
		id: 2,
		project_slug: 'alpha',
		sender_name: 'Bob',
		subject: 'Urgent rocket',
		body_md: 'Launch sequence',
		importance: 'high',
		thread_id: 'thread-1',
		recipients: ['Cara', 'Alice'],
		created_ts: '2025-01-02T10:00:00Z'
	},
	{
		id: 3,
		project_slug: 'beta',
		sender_name: 'Cara',
		subject: 'Weekly update',
		body_md: 'status report',
		importance: 'low',
		thread_id: null,
		recipients: ['Bob'],
		created_ts: '2024-12-31T10:00:00Z'
	}
];

test.describe('Unified inbox stores', () => {
	test.beforeEach(() => {
		allMessages.set([]);
		searchQuery.set('');
		sortBy.set('newest');
		selectedMessages.set([]);
		selectedMessage.set(null);
		filters.set({ ...defaultFilters });
	});

	test('derived stores provide unique filter values', () => {
		allMessages.set(sampleMessages);

		expect(get(uniqueProjects).sort()).toEqual(['alpha', 'beta']);
		expect(get(uniqueSenders).sort()).toEqual(['Alice', 'Bob', 'Cara']);
		expect(get(uniqueRecipients).sort()).toEqual(['Alice', 'Bob', 'Cara']);
	});

	test('filteredMessages respects search and filters', () => {
		allMessages.set(sampleMessages);
		searchQuery.set('rocket');
		filters.set({
			...defaultFilters,
			project: 'alpha',
			recipient: 'Alice',
			importance: 'high',
			hasThread: 'yes'
		});

		const result = get(filteredMessages);
		expect(result).toHaveLength(1);
		expect(result[0].id).toBe(2);
	});

	test('filteredMessages sorts by requested mode', () => {
		allMessages.set(sampleMessages);

		sortBy.set('oldest');
		expect(get(filteredMessages).map((m) => m.id)).toEqual([3, 1, 2]);

		sortBy.set('sender');
		expect(get(filteredMessages).map((m) => m.id)).toEqual([1, 2, 3]);

		sortBy.set('longest');
		expect(get(filteredMessages)[0].id).toBe(2);
	});

	test('actions update selection state', () => {
		allMessages.set(sampleMessages);
		searchQuery.set('rocket');

		toggleSelectAll();
		expect(get(selectedMessages)).toEqual([2]);

		toggleSelectAll();
		expect(get(selectedMessages)).toEqual([]);
	});

	test('navigation actions move selected message', () => {
		allMessages.set(sampleMessages);
		sortBy.set('newest');

		selectNextMessage();
		expect(get(selectedMessage)?.id).toBe(2);

		selectNextMessage();
		expect(get(selectedMessage)?.id).toBe(1);

		selectPreviousMessage();
		expect(get(selectedMessage)?.id).toBe(2);
	});

	test('clearFilters resets to defaults', () => {
		filters.set({
			project: 'alpha',
			sender: 'Alice',
			recipient: 'Bob',
			importance: 'high',
			hasThread: 'yes'
		});

		clearFilters();
		expect(get(filters)).toEqual(defaultFilters);
	});
});
