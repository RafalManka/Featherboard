const { test, expect } = require('@playwright/test');
const { waitForMagicLink } = require('../helpers');

test('create Stripe E2E organization and save admin session', async ({ page }) => {
  await page.goto('/setup');
  await expect(page).toHaveTitle('Create organisation · Featherboard');

  await page.getByLabel('Organisation name').fill('Stripe E2E');
  await page.getByLabel('Organisation URL').fill('stripe-e2e');
  await page.getByRole('button', { name: 'Continue' }).click();

  await expect(page).toHaveURL(/\/setup\/admin$/);
  await page.getByLabel('Email address').fill('stripe-admin@example.com');
  await page.getByRole('button', { name: 'Send sign-in link' }).click();
  await expect(page.getByText('please check your email')).toBeVisible();

  const magicLink = await waitForMagicLink(
    expect,
    /http:\/\/127\.0\.0\.1:3103\/setup\/confirm\/[a-zA-Z0-9-]+/,
  );

  await page.goto(magicLink);
  await expect(page).toHaveURL(/\/stripe-e2e$/);
  await page.context().storageState({ path: 'playwright/.auth/stripe-admin.json' });
});
