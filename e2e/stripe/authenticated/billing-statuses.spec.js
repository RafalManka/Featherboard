const { test, expect } = require('@playwright/test');

test.describe.configure({ mode: 'serial' });

test.afterAll(async ({ request }) => {
  const response = await request.post('/e2e/billing/stripe-e2e/free');
  expect(response.ok()).toBeTruthy();
});

const freeStatuses = ['free', 'canceled', 'incomplete', 'incomplete_expired', 'unpaid', 'paused'];
const proStatuses = [
  { status: 'active', message: 'Your subscription is active.' },
  { status: 'trialing', message: 'Your trial is active.' },
  {
    status: 'past_due',
    message: 'There is a problem with your payment. Please update your billing details.',
  },
];

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

for (const { status, message } of proStatuses) {
  test(`Stripe ${status} status shows the Pro subscription`, async ({ page, request }) => {
    const response = await request.post(`/e2e/billing/stripe-e2e/${status}`);
    expect(response.ok()).toBeTruthy();

    await page.goto('/stripe-e2e/settings/site');
    await expect(page.getByText(message)).toBeVisible();
    await expect(page.getByText(status, { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Manage subscription' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Upgrade' })).toHaveCount(0);
  });
}
