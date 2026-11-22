const { test, expect } = require('@playwright/test');

test('billing is clearly marked as not configured without Stripe', async ({ page }) => {
  await page.goto('/acme-e2e/settings/site');

  await expect(
    page.getByText(
      'Your workspace is on the Free plan. Billing is not configured for this instance.',
    ),
  ).toBeVisible();
  await expect(page.getByRole('button', { name: 'Upgrade' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Manage subscription' })).toHaveCount(0);
});
