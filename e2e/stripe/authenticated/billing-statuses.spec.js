const { test, expect } = require('@playwright/test');

test.describe.configure({ mode: 'serial' });

const freeStatuses = ['free', 'canceled', 'incomplete', 'incomplete_expired', 'unpaid', 'paused'];
const proStatuses = ['active', 'trialing', 'past_due'];

for (const status of freeStatuses) {
  test(`Stripe ${status} status shows the Free plan`, async ({ page, request }) => {
    const response = await request.post(`/e2e/billing/stripe-e2e/${status}`);
    expect(response.ok()).toBeTruthy();

    await page.goto('/stripe-e2e/settings/site');
    await expect(page.getByText('Your workspace is on the Free plan.')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Upgrade' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Manage subscription' })).toHaveCount(0);
  });
}

for (const status of proStatuses) {
  test(`Stripe ${status} status shows the Pro subscription`, async ({ page, request }) => {
    const response = await request.post(`/e2e/billing/stripe-e2e/${status}`);
    expect(response.ok()).toBeTruthy();

    await page.goto('/stripe-e2e/settings/site');
    await expect(page.getByText('Your subscription is active.')).toBeVisible();
    await expect(page.getByText(status, { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Manage subscription' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Upgrade' })).toHaveCount(0);
  });
}
