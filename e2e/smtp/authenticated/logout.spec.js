const { test, expect } = require('@playwright/test');

test('authenticated users can log out', async ({ page }) => {
  await page.goto('/acme-e2e');
  await page.getByLabel('Account menu').click();
  await page.getByRole('link', { name: 'Log out' }).click();

  await expect(page).toHaveURL(/\/acme-e2e$/);
  await expect(page.getByRole('link', { name: 'Log in' })).toBeVisible();
  await expect(page.getByLabel('Account menu')).not.toBeVisible();
});
