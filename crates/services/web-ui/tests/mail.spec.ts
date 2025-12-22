import { test, expect } from '@playwright/test';

const sampleMessages = [
	{
		id: 101,
		project_slug: 'alpha',
		sender_name: 'Alice',
		recipient_names: ['Bob'],
		subject: 'Alpha status',
		body_md: 'First message body',
		importance: 'normal',
		thread_id: null,
		created_ts: '2025-01-01T10:00:00Z',
		created_relative: '2h ago',
		excerpt: 'First message body'
	},
	{
		id: 102,
		project_slug: 'beta',
		sender_name: 'Bob',
		recipient_names: ['Alice', 'Cara'],
		subject: 'Beta alert',
		body_md: 'Second message body',
		importance: 'high',
		thread_id: 'thread-9',
		created_ts: '2025-01-02T11:00:00Z',
		created_relative: '1h ago',
		excerpt: 'Second message body'
	}
];

async function mockUnifiedInbox(page) {
	await page.route('**/api/unified-inbox**', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				messages: sampleMessages,
				total_count: sampleMessages.length
			})
		});
	});
}

test('mail page renders split view and updates detail selection', async ({ page }) => {
	await mockUnifiedInbox(page);
	await page.goto('/mail');

	const listPane = page.getByTestId('mail-list');
	const detailPane = page.getByTestId('mail-detail');

	await expect(listPane).toBeVisible();
	await expect(detailPane).toBeVisible();
	await expect(page.getByTestId('mail-detail-subject')).toHaveText('Alpha status');

	await page.getByTestId('mail-item-102').click();
	await expect(page.getByTestId('mail-detail-subject')).toHaveText('Beta alert');
});

test('mail page keyboard shortcuts drive focus and selection', async ({ page }) => {
	await mockUnifiedInbox(page);
	await page.goto('/mail');

	await page.keyboard.press('j');
	await expect(page.getByTestId('mail-detail-subject')).toHaveText('Beta alert');

	await page.keyboard.press('k');
	await expect(page.getByTestId('mail-detail-subject')).toHaveText('Alpha status');

	await page.keyboard.press('/');
	await expect(page.getByTestId('mail-search-input')).toBeFocused();

	await page.keyboard.press('f');
	await expect(page.getByTestId('mail-root')).toHaveAttribute('data-fullscreen', 'true');

	await page.keyboard.press('Escape');
	await expect(page.getByTestId('mail-empty-detail')).toBeVisible();
});

test('mail page stacks list and detail on mobile viewport', async ({ page }) => {
	await page.setViewportSize({ width: 390, height: 844 });
	await mockUnifiedInbox(page);
	await page.goto('/mail');

	const listBox = await page.getByTestId('mail-list').boundingBox();
	const detailBox = await page.getByTestId('mail-detail').boundingBox();

	expect(listBox).not.toBeNull();
	expect(detailBox).not.toBeNull();
	if (!listBox || !detailBox) return;

	expect(detailBox.y).toBeGreaterThan(listBox.y);
});
