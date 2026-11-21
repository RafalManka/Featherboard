const { test, expect } = require('@playwright/test');

test('empty roadmap renders correctly', async ({ page }) => {
  await page.goto('/default/roadmap');

  await expect(page).toHaveTitle('Roadmap');
  await expect(page.getByRole('heading', { name: 'Roadmap' })).toBeVisible();
  await expect(page).toHaveScreenshot('empty-roadmap.png', { fullPage: true });
});

test('empty changelog list renders correctly', async ({ page }) => {
  await page.goto('/default/changelogs');

  await expect(page).toHaveTitle('Changelogs');
  await expect(page.getByRole('heading', { name: 'Changelogs' })).toBeVisible();
  await expect(page).toHaveScreenshot('empty-changelogs.png', { fullPage: true });
});

test('board renders correctly on a mobile viewport', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 900 });
  await page.goto('/default');

  await expect(page).toHaveTitle('Ideas');
  await expect(page.getByRole('heading', { name: 'Ideas' })).toBeVisible();
  await expect(page).toHaveScreenshot('mobile-board.png', { fullPage: true });
});

test('roadmap renders correctly on a mobile viewport', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 900 });
  await page.goto('/default/roadmap');

  await expect(page).toHaveTitle('Roadmap');
  await expect(page).toHaveScreenshot('mobile-roadmap.png', { fullPage: true });
});

test('changelog list renders correctly on a mobile viewport', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 900 });
  await page.goto('/default/changelogs');

  await expect(page).toHaveTitle('Changelogs');
  await expect(page).toHaveScreenshot('mobile-changelogs.png', { fullPage: true });
});
