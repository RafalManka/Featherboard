const { test, expect } = require('@playwright/test');

test('site settings render correctly on a mobile viewport', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 900 });
  await page.goto('/acme-e2e/settings/site');

  await expect(page).toHaveTitle('Settings');
  await expect(page.getByRole('heading', { name: 'Billing' })).toBeVisible();
  await expect(page).toHaveScreenshot('mobile-site-settings.png', { fullPage: true });
});

test('user settings render correctly on a mobile viewport', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 900 });
  await page.goto('/acme-e2e/settings/user');

  await expect(page).toHaveTitle('User settings');
  await expect(page.getByRole('heading', { name: 'Notifications' })).toBeVisible();
  await expect(page).toHaveScreenshot('mobile-user-settings.png', { fullPage: true });
});
