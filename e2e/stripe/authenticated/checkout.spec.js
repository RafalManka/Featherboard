const { test, expect } = require('@playwright/test');

test('admin can start a Stripe Checkout session', async ({ page }) => {
  await page.goto('/stripe-e2e/settings/site');

  await expect(
    page.getByText('Your workspace is on the Free plan.'),
  ).toBeVisible();

  await page.getByRole('button', { name: 'Upgrade' }).click();
  await expect(page).toHaveURL(/checkout\.stripe\.com/);
});
