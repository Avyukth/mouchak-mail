/**
 * Date formatting utilities for the dashboard.
 *
 * @module utils/date
 */

/**
 * Format a date/timestamp into a human-readable relative time string.
 *
 * @param date - ISO date string, Date object, or timestamp
 * @returns Relative time string (e.g., "just now", "5m ago", "2h ago", "3d ago")
 *
 * @example
 * ```ts
 * formatRelativeTime(new Date(Date.now() - 5 * 60 * 1000)) // "5m ago"
 * formatRelativeTime("2024-12-24T10:00:00Z") // "2h ago" (depending on current time)
 * ```
 */
export function formatRelativeTime(date: Date | string | number): string {
	const now = new Date();
	const then = date instanceof Date ? date : new Date(date);
	const diffMs = now.getTime() - then.getTime();

	// Handle invalid dates
	if (isNaN(diffMs)) {
		return '';
	}

	// Handle future dates
	if (diffMs < 0) {
		return 'soon';
	}

	const diffSecs = Math.floor(diffMs / 1000);
	const diffMins = Math.floor(diffMs / 60000);
	const diffHours = Math.floor(diffMs / 3600000);
	const diffDays = Math.floor(diffMs / 86400000);
	const diffWeeks = Math.floor(diffMs / 604800000);

	if (diffSecs < 60) return 'just now';
	if (diffMins < 60) return `${diffMins}m ago`;
	if (diffHours < 24) return `${diffHours}h ago`;
	if (diffDays < 7) return `${diffDays}d ago`;
	if (diffWeeks < 4) return `${diffWeeks}w ago`;

	// For older dates, show the actual date
	return then.toLocaleDateString(undefined, {
		month: 'short',
		day: 'numeric'
	});
}

/**
 * Format a date for display in lists or cards.
 *
 * @param date - ISO date string, Date object, or timestamp
 * @returns Formatted date string (e.g., "Dec 24, 2024")
 */
export function formatDate(date: Date | string | number): string {
	const d = date instanceof Date ? date : new Date(date);

	if (isNaN(d.getTime())) {
		return '';
	}

	return d.toLocaleDateString(undefined, {
		year: 'numeric',
		month: 'short',
		day: 'numeric'
	});
}

/**
 * Format a timestamp for display with time.
 *
 * @param date - ISO date string, Date object, or timestamp
 * @returns Formatted datetime string (e.g., "Dec 24, 2024 at 10:30 AM")
 */
export function formatDateTime(date: Date | string | number): string {
	const d = date instanceof Date ? date : new Date(date);

	if (isNaN(d.getTime())) {
		return '';
	}

	return d.toLocaleString(undefined, {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
		hour: 'numeric',
		minute: '2-digit'
	});
}
