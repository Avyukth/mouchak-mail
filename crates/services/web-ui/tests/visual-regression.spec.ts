import { test, expect } from '@playwright/test';

const API_BASE = 'http://localhost:8000';

/**
 * Visual Regression Tests
 *
 * These tests capture screenshots for visual comparison.
 * On first run, they create baseline images in tests/*.spec.ts-snapshots/
 * On subsequent runs, they compare against baselines.
 *
 * Usage:
 *   bun run playwright test visual-regression.spec.ts --update-snapshots  # Create/update baselines
 *   bun run playwright test visual-regression.spec.ts                      # Compare against baselines
 */

test.describe('Visual Regression - All Routes', () => {
	test.beforeAll(async ({ request }) => {
		// Create test data for visual consistency
		const projectRes = await request.post(`${API_BASE}/api/project/ensure`, {
			data: { human_key: '/visual-test/regression' }
		});
		if (projectRes.ok()) {
			const project = await projectRes.json();

			// Create test agent
			await request.post(`${API_BASE}/api/agent/register`, {
				data: {
					project_slug: project.slug,
					name: 'VisualTestAgent',
					program: 'playwright',
					model: 'visual-test',
					task_description: 'Visual regression testing'
				}
			});

			// Create test message
			await request.post(`${API_BASE}/api/message/send`, {
				data: {
					project_slug: project.slug,
					sender_name: 'VisualTestAgent',
					recipient_names: ['VisualTestAgent'],
					subject: 'Visual Test Message',
					body_md: 'This message is used for visual regression testing.',
					importance: 'normal'
				}
			});
		}
	});

	test('dashboard page visual', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');
		await page.waitForTimeout(1000);

		await expect(page).toHaveScreenshot('dashboard-desktop.png', {
			fullPage: true,
			maxDiffPixelRatio: 0.05
		});
	});

	test('projects list page visual', async ({ page }) => {
		await page.goto('/projects');
		await page.waitForLoadState('networkidle');
		await page.waitForTimeout(1000);

		// Wait for table to load
		const table = page.locator('table');
		const emptyState = page.locator('text=No projects yet');
		await expect(table.or(emptyState)).toBeVisible({ timeout: 10000 });

		await expect(page).toHaveScreenshot('projects-list-desktop.png', {
			fullPage: true,
			maxDiffPixelRatio: 0.05
		});
	});

	test('inbox page visual', async ({ page }) => {
		await page.goto('/inbox');
		await page.waitForLoadState('networkidle');
		await page.waitForTimeout(1000);

		await expect(page.getByRole('heading', { name: 'Inbox' })).toBeVisible();

		await expect(page).toHaveScreenshot('inbox-desktop.png', {
			fullPage: true,
			maxDiffPixelRatio: 0.05
		});
	});

	test('agents page visual', async ({ page }) => {
		await page.goto('/agents');
		await page.waitForLoadState('networkidle');
		await page.waitForTimeout(1000);

		await expect(page.getByRole('heading', { name: 'All Agents' })).toBeVisible({ timeout: 10000 });

		await expect(page).toHaveScreenshot('agents-desktop.png', {
			fullPage: true,
			maxDiffPixelRatio: 0.05
		});
	});
});

test.describe('Visual Regression - Dark Mode', () => {
	test.use({ colorScheme: 'dark' });

	test('dashboard dark mode', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');
		await page.waitForTimeout(1000);

		await expect(page).toHaveScreenshot('dashboard-dark.png', {
			fullPage: true,
			maxDiffPixelRatio: 0.05
		});
	});

	test('inbox dark mode', async ({ page }) => {
		await page.goto('/inbox');
		await page.waitForLoadState('networkidle');
		await page.waitForTimeout(1000);

		await expect(page).toHaveScreenshot('inbox-dark.png', {
			fullPage: true,
			maxDiffPixelRatio: 0.05
		});
	});
});

test.describe('Visual Regression - Reduced Motion', () => {
	test('respects prefers-reduced-motion', async ({ page }) => {
		await page.emulateMedia({ reducedMotion: 'reduce' });
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		const reducedMotion = await page.evaluate(() => {
			return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		});
		expect(reducedMotion).toBe(true);

		await expect(page).toHaveScreenshot('dashboard-reduced-motion.png', {
			fullPage: true,
			maxDiffPixelRatio: 0.05
		});
	});
});
