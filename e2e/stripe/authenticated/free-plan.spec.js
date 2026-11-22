const { test, expect } = require('@playwright/test');

test('new organizations see the Free plan', async ({ page }) => {
  await page.goto('/stripe-e2e/settings/site');

  await expect(page.getByRole('heading', { name: 'Billing' })).toBeVisible();
  await expect(
    page.getByText('Your workspace is on the Free plan.'),
  ).toBeVisible();
  await expect(page.getByRole('button', { name: 'Upgrade' })).toBeVisible();
  await expect(
    page.getByRole('button', { name: /Manage subscription/ }),
  ).not.toBeVisible();
});
