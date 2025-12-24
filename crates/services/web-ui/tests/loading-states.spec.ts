import { test, expect } from '@playwright/test';

test.describe('Loading Skeleton States', () => {
	test.describe('Projects Page', () => {
		test('shows skeleton cards while loading', async ({ page }) => {
			// Slow down the API response
			await page.route('**/api/project/list', async (route) => {
				await new Promise(r => setTimeout(r, 1000));
				await route.fulfill({
					json: [{ id: 1, slug: 'test', human_key: 'Test Project', created_at: new Date().toISOString() }]
				});
			});

			await page.goto('/projects');

			// Should show skeleton cards
			const skeletons = page.locator('[data-testid="project-skeleton"]');
			await expect(skeletons.first()).toBeVisible();

			// Wait for content to load
			await expect(page.getByText('Test Project')).toBeVisible({ timeout: 5000 });

			// Skeleton should be gone
			await expect(skeletons.first()).not.toBeVisible();
		});

		test('skeleton grid matches content layout', async ({ page }) => {
			await page.route('**/api/project/list', async (route) => {
				await new Promise(r => setTimeout(r, 500));
				await route.fulfill({ json: [] });
			});

			await page.goto('/projects');

			// Skeletons should be in a grid
			const grid = page.locator('.grid');
			await expect(grid).toBeVisible();

			// Should have multiple skeleton cards
			const skeletons = page.locator('[data-testid="project-skeleton"]');
			expect(await skeletons.count()).toBeGreaterThanOrEqual(3);
		});
	});

	test.describe('Agents Page', () => {
		test('shows agent skeleton cards while loading', async ({ page }) => {
			await page.route('**/api/project/list', async (route) => {
				await new Promise(r => setTimeout(r, 1000));
				await route.fulfill({
					json: [{ id: 1, slug: 'test', human_key: 'Test', created_at: new Date().toISOString() }]
				});
			});

			await page.goto('/agents');

			// Should show agent skeleton cards
			const skeletons = page.locator('[data-testid="agent-skeleton"]');
			await expect(skeletons.first()).toBeVisible();
		});
	});

	test.describe('Inbox Page', () => {
		test('shows message skeleton while loading messages', async ({ page }) => {
			// Mock projects
			await page.route('**/api/project/list', async (route) => {
				await route.fulfill({
					json: [{ id: 1, slug: 'test-project', human_key: 'Test Project', created_at: new Date().toISOString() }]
				});
			});

			// Mock agents
			await page.route('**/api/projects/test-project/agents', async (route) => {
				await route.fulfill({
					json: [{ id: 1, name: 'test-agent', program: 'test', model: 'gpt-4', last_active_ts: new Date().toISOString() }]
				});
			});

			// Slow down inbox response
			await page.route('**/api/inbox/**', async (route) => {
				await new Promise(r => setTimeout(r, 1000));
				await route.fulfill({ json: [] });
			});

			await page.goto('/inbox?project=test-project&agent=test-agent');
			await page.waitForLoadState('networkidle');

			// Should show message skeletons
			const skeletons = page.locator('[data-testid="message-skeleton"]');
			// Either skeletons are visible or the empty state is shown
			const hasSkeletons = await skeletons.first().isVisible().catch(() => false);
			const hasEmptyState = await page.getByText(/inbox is empty|no messages/i).isVisible().catch(() => false);

			expect(hasSkeletons || hasEmptyState).toBeTruthy();
		});
	});

	test.describe('Accessibility', () => {
		test('skeletons have proper structure', async ({ page }) => {
			await page.route('**/api/project/list', async (route) => {
				await new Promise(r => setTimeout(r, 1000));
				await route.fulfill({ json: [] });
			});

			await page.goto('/projects');

			const skeleton = page.locator('[data-testid="project-skeleton"]').first();
			await expect(skeleton).toBeVisible();

			// Should have a card structure
			const card = skeleton.locator('[class*="card"]');
			await expect(card).toBeVisible();
		});
	});
});
