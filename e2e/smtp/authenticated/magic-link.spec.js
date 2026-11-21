const { test, expect } = require('@playwright/test');
const { waitForMagicLink } = require('../../helpers');

test('email login sends an absolute magic link', async ({ page }) => {
  await page.goto('/acme-e2e/logout');
  await expect(page).toHaveURL(/\/acme-e2e$/);

  await page.goto('/acme-e2e/login');
  await page.getByLabel('Email address').fill('admin@example.com');
  await page.getByRole('button', { name: 'Send me a sign-in link' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/login\/check-email$/);

  const magicLink = await waitForMagicLink(
    expect,
    /http:\/\/127\.0\.0\.1:3101\/acme-e2e\/login\/check-email\/[a-zA-Z0-9-]+/,
  );

  expect(magicLink).toMatch(/^http:\/\/127\.0\.0\.1:3101\//);

  await page.goto(magicLink);
  await expect(page).toHaveURL(/\/acme-e2e$/);
  await expect(page).toHaveTitle('Ideas');
  await expect(page.getByLabel('Account menu')).toBeVisible();
});
