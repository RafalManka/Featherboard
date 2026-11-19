const { test, expect } = require('@playwright/test');

test('setup is unavailable when SMTP is not configured', async ({ page }) => {
  const response = await page.goto('/setup');

  expect(response).not.toBeNull();
  expect(response.status()).toBe(404);
});
