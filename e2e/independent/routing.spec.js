const { test, expect } = require('@playwright/test');

test('root redirects to the first organization', async ({ page }) => {
  await page.goto('/');

  await expect(page).toHaveURL(/\/default$/);
  await expect(page).toHaveTitle('Ideas');
});

test('unknown organization pages return not found', async ({ page }) => {
  const response = await page.goto('/does-not-exist');

  expect(response).not.toBeNull();
  expect(response.status()).toBe(404);
});
