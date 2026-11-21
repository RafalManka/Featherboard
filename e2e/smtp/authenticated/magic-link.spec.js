const fs = require('fs');
const { test, expect } = require('@playwright/test');

const EMAIL_FILE = '/private/tmp/featherboard-e2e-email.eml';

test('email login sends an absolute magic link', async ({ page }) => {
  await page.goto('/acme-e2e/logout');
  await expect(page).toHaveURL(/\/acme-e2e$/);

  await page.goto('/acme-e2e/login');
  await page.getByLabel('Email address').fill('admin@example.com');
  await page.getByRole('button', { name: 'Send me a sign-in link' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/login\/check-email$/);

  let magicLink = '';
  await expect.poll(() => {
    try {
      const email = fs
        .readFileSync(EMAIL_FILE, 'utf8')
        .replace(/=\r?\n/g, '')
        .replace(/=3D/g, '=');
      magicLink = email.match(
        /http:\/\/127\.0\.0\.1:3101\/acme-e2e\/login\/check-email\/[a-zA-Z0-9-]+/,
      )?.[0] ?? '';
    } catch {
      magicLink = '';
    }
    return magicLink;
  }).toBeTruthy();

  expect(magicLink).toMatch(/^http:\/\/127\.0\.0\.1:3101\//);

  await page.goto(magicLink);
  await expect(page).toHaveURL(/\/acme-e2e$/);
  await expect(page).toHaveTitle('Ideas');
  await expect(page.getByLabel('Account menu')).toBeVisible();
});
