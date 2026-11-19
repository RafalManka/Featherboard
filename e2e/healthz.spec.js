const { test, expect } = require('@playwright/test');

test('healthz endpoint is healthy', async ({ page }) => {
  const response = await page.goto('/healthz');

  expect(response).not.toBeNull();
  expect(response.status()).toBe(200);
  await expect(page.locator('body')).toHaveText('ok');
});
