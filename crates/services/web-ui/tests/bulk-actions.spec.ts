import { test, expect } from '@playwright/test';

const API_BASE = process.env.API_BASE || 'http://localhost:9765';

test.describe('Bulk Selection on Projects', () => {
	test.beforeAll(async ({ request }) => {
		// Create test projects for bulk selection testing
		const testProjects = [
			'bulk-test-1-' + Date.now().toString(36),
			'bulk-test-2-' + Date.now().toString(36),
			'bulk-test-3-' + Date.now().toString(36),
		];

		for (const name of testProjects) {
			await request.post(`${API_BASE}/api/project/ensure`, {
				data: { human_key: name }
			});
		}
	});

	test('project cards have selection checkboxes', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');

		// Wait for projects to load
		await page.waitForSelector('.grid a[href^="/projects/"]', { timeout: 10000 });

		// Each project card should have a checkbox
		const checkboxes = page.locator('[data-testid="project-select-checkbox"]');
		await expect(checkboxes.first()).toBeVisible();
	});

	test('select all checkbox toggles all visible items', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');

		// Wait for projects to load
		await page.waitForSelector('.grid a[href^="/projects/"]', { timeout: 10000 });

		const selectAll = page.locator('[data-testid="select-all-checkbox"]');
		await selectAll.check();

		// All item checkboxes should be checked
		const itemCheckboxes = page.locator('[data-testid="project-select-checkbox"]');
		const count = await itemCheckboxes.count();

		for (let i = 0; i < Math.min(count, 5); i++) {
			await expect(itemCheckboxes.nth(i)).toBeChecked();
		}
	});

	test('selecting items shows bulk action bar', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');

		// Wait for projects to load
		await page.waitForSelector('.grid a[href^="/projects/"]', { timeout: 10000 });

		const firstCheckbox = page.locator('[data-testid="project-select-checkbox"]').first();
		await firstCheckbox.check();

		// Bulk action bar should appear
		const actionBar = page.locator('[data-testid="bulk-action-bar"]');
		await expect(actionBar).toBeVisible();

		// Should show selection count
		await expect(actionBar).toContainText('1 selected');
	});

	test('bulk action bar has delete button', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');

		await page.waitForSelector('.grid a[href^="/projects/"]', { timeout: 10000 });

		await page.locator('[data-testid="project-select-checkbox"]').first().check();

		const actionBar = page.locator('[data-testid="bulk-action-bar"]');
		await expect(actionBar.locator('button:has-text("Delete")')).toBeVisible();
	});

	test('bulk action bar has export button', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');

		await page.waitForSelector('.grid a[href^="/projects/"]', { timeout: 10000 });

		await page.locator('[data-testid="project-select-checkbox"]').first().check();

		const actionBar = page.locator('[data-testid="bulk-action-bar"]');
		await expect(actionBar.locator('[data-testid="bulk-export-button"]')).toBeVisible();
	});

	test('clear selection button deselects all', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');

		await page.waitForSelector('.grid a[href^="/projects/"]', { timeout: 10000 });

		// Select all first
		await page.locator('[data-testid="select-all-checkbox"]').check();

		// Clear selection
		await page.locator('[data-testid="clear-selection-button"]').click();

		// All checkboxes should be unchecked
		const checkboxes = page.locator('[data-testid="project-select-checkbox"]');
		const count = await checkboxes.count();

		for (let i = 0; i < Math.min(count, 5); i++) {
			await expect(checkboxes.nth(i)).not.toBeChecked();
		}

		// Bulk action bar should be hidden
		const actionBar = page.locator('[data-testid="bulk-action-bar"]');
		await expect(actionBar).not.toBeVisible();
	});

	test('escape key clears selection', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');

		await page.waitForSelector('.grid a[href^="/projects/"]', { timeout: 10000 });

		await page.locator('[data-testid="project-select-checkbox"]').first().check();

		// Verify action bar is visible
		await expect(page.locator('[data-testid="bulk-action-bar"]')).toBeVisible();

		// Press Escape
		await page.keyboard.press('Escape');

		// Action bar should be hidden
		await expect(page.locator('[data-testid="bulk-action-bar"]')).not.toBeVisible();
	});

	test('selecting multiple items updates count', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');

		await page.waitForSelector('.grid a[href^="/projects/"]', { timeout: 10000 });

		const checkboxes = page.locator('[data-testid="project-select-checkbox"]');
		const count = await checkboxes.count();

		if (count >= 2) {
			await checkboxes.nth(0).check();
			await checkboxes.nth(1).check();

			const actionBar = page.locator('[data-testid="bulk-action-bar"]');
			await expect(actionBar).toContainText('2 selected');
		}
	});

	test('unchecking item updates count', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');

		await page.waitForSelector('.grid a[href^="/projects/"]', { timeout: 10000 });

		const checkboxes = page.locator('[data-testid="project-select-checkbox"]');
		const count = await checkboxes.count();

		if (count >= 2) {
			await checkboxes.nth(0).check();
			await checkboxes.nth(1).check();

			// Uncheck one
			await checkboxes.nth(0).uncheck();

			const actionBar = page.locator('[data-testid="bulk-action-bar"]');
			await expect(actionBar).toContainText('1 selected');
		}
	});

	test('select all shows indeterminate when partial selection', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');

		await page.waitForSelector('.grid a[href^="/projects/"]', { timeout: 10000 });

		const checkboxes = page.locator('[data-testid="project-select-checkbox"]');
		const count = await checkboxes.count();

		if (count >= 2) {
			// Select only first item
			await checkboxes.nth(0).check();

			// Select all checkbox should show indeterminate state
			const selectAll = page.locator('[data-testid="select-all-checkbox"]');
			// Note: indeterminate is a property, not an attribute
			const isIndeterminate = await selectAll.evaluate((el: HTMLInputElement) => el.indeterminate);
			expect(isIndeterminate).toBe(true);
		}
	});
});
