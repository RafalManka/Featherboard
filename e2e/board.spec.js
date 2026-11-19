const { test, expect } = require('@playwright/test');

test('default board renders correctly', async ({ page }) => {
  const response = await page.goto('/default');

  expect(response).not.toBeNull();
  expect(response.status()).toBe(200);
  await expect(page).toHaveTitle('Ideas');
  await expect(page.getByRole('heading', { name: 'Ideas' })).toBeVisible();
  await expect(page.getByText('No ideas yet. Be the first to share feedback!')).toBeVisible();
  await expect(page).toHaveScreenshot('default-board.png', { fullPage: true });
});
