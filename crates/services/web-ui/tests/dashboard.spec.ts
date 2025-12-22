import { test, expect } from '@playwright/test';

const API_BASE = 'http://localhost:8000';

test.describe('Dashboard Page', () => {
	test.beforeAll(async ({ request }) => {
		// Ensure test project exists
		const projectRes = await request.post(`${API_BASE}/api/project/ensure`, {
			data: { human_key: '/test/dashboard-e2e' }
		});
		expect(projectRes.ok()).toBeTruthy();
	});

	test('should display dashboard with all sections', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// Check page title
		await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
		await expect(page.getByText('Welcome to MCP Agent Mail')).toBeVisible();

		// Check status cards are visible
		await expect(page.getByText('Backend Status')).toBeVisible();
		await expect(page.getByText('Projects')).toBeVisible();
		await expect(page.getByText('Quick Actions')).toBeVisible();
	});

	test('should show healthy backend status', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// Wait for health check to complete
		const statusIndicator = page.locator('.bg-green-500');
		await expect(statusIndicator).toBeVisible({ timeout: 10000 });

		// Status should show 'ok'
		await expect(page.getByText('ok')).toBeVisible();
	});

	test('should navigate to projects from quick actions', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// Click View Projects link
		await page.getByRole('link', { name: 'View Projects →' }).click();

		// Should navigate to projects page
		await page.waitForURL('/projects');
		await expect(page.getByRole('heading', { name: 'Projects' })).toBeVisible();
	});

	test('should navigate to inbox from quick actions', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// Click Check Inbox link
		await page.getByRole('link', { name: 'Check Inbox →' }).click();

		// Should navigate to inbox page
		await page.waitForURL('/inbox');
		await expect(page.getByRole('heading', { name: 'Inbox' })).toBeVisible();
	});

	test('should display project count', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// Projects card should show a number
		const projectsCard = page.locator('.rounded-xl').filter({ hasText: 'Projects' });
		await expect(projectsCard).toBeVisible();

		// Should have a numeric count
		const countText = await projectsCard.locator('.text-2xl.font-bold').textContent();
		expect(parseInt(countText || '0', 10)).toBeGreaterThanOrEqual(0);
	});

	test('should show recent projects when available', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// Wait for projects to load
		await page.waitForTimeout(2000);

		// Check if Recent Projects section appears
		const recentProjects = page.getByRole('heading', { name: 'Recent Projects' });
		const isVisible = await recentProjects.isVisible();

		if (isVisible) {
			// If visible, should have project links
			const projectLinks = page.locator('ul li a');
			const count = await projectLinks.count();
			expect(count).toBeGreaterThan(0);
		}
	});

	test('keyboard navigation works', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// Tab through interactive elements
		await page.keyboard.press('Tab');
		await page.keyboard.press('Tab');

		// First focusable element should be focused
		const focusedElement = page.locator(':focus');
		await expect(focusedElement).toBeVisible();
	});
});

test.describe('Dashboard Visual Regression', () => {
	test('dashboard screenshot matches baseline', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// Wait for dynamic content
		await page.waitForTimeout(2000);

		// Take visual regression screenshot
		await expect(page).toHaveScreenshot('dashboard.png', {
			fullPage: true,
			maxDiffPixelRatio: 0.05
		});
	});
});

test.describe('Dashboard Mobile', () => {
	test('should be responsive on mobile viewport', async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle');

		// Check all sections are visible
		await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
		await expect(page.getByText('Backend Status')).toBeVisible();
		await expect(page.getByText('Projects')).toBeVisible();

		// Cards should stack on mobile
		const cards = page.locator('.rounded-xl');
		const cardCount = await cards.count();
		expect(cardCount).toBeGreaterThanOrEqual(3);
	});
});
