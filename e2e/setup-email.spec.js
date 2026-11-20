const fs = require('fs');
const { test, expect } = require('@playwright/test');

const EMAIL_FILE = '/private/tmp/featherboard-e2e-email.eml';

test('configured email setup completes through the magic link', async ({ page }) => {
  await page.goto('/setup');
  await expect(page).toHaveTitle('Create organisation · Featherboard');

  await page.getByLabel('Organisation name').fill('Acme E2E');
  await page.getByLabel('Organisation URL').fill('acme-e2e');
  await page.getByRole('button', { name: 'Continue' }).click();

  await expect(page).toHaveURL(/\/setup\/admin$/);
  await page.getByLabel('Email address').fill('admin@example.com');
  await page.getByRole('button', { name: 'Send sign-in link' }).click();
  await expect(page.getByText('please check your email')).toBeVisible();

  let magicLink = '';
  await expect.poll(() => {
    try {
      const email = fs
        .readFileSync(EMAIL_FILE, 'utf8')
        .replace(/=\r?\n/g, '')
        .replace(/=3D/g, '=');
      magicLink = email.match(
        /http:\/\/127\.0\.0\.1:3101\/setup\/confirm\/[a-zA-Z0-9-]+/,
      )?.[0] ?? '';
    } catch {
      magicLink = '';
    }
    return magicLink;
  }).toBeTruthy();

  await page.goto(magicLink);
  await expect(page).toHaveURL(/\/acme-e2e$/);
  await expect(page).toHaveTitle('Ideas');
});
