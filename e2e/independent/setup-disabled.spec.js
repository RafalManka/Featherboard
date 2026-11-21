const { test, expect } = require('@playwright/test');

test('setup is unavailable when SMTP is not configured', async ({ page }) => {
  const response = await page.goto('/setup');

  expect(response).not.toBeNull();
  expect(response.status()).toBe(404);
});

test('email login reports SMTP unavailable when SMTP is not configured', async ({
  page,
}) => {
  await page.goto('/default/login');
  await page.getByLabel('Email address').fill('unavailable@example.com');

  const responsePromise = page.waitForResponse((response) =>
    response.url().endsWith('/default/login') && response.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Send me a sign-in link' }).click();
  const response = await responsePromise;

  expect(response.status()).toBe(503);
  await expect(page.getByText('SMTP is unavailable')).toBeVisible();
});
