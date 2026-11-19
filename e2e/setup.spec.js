const { test, expect } = require('@playwright/test');

test('setup page displays the organization form', async ({ page }) => {
  const response = await page.goto('/setup');

  expect(response).not.toBeNull();
  expect(response.status()).toBe(200);
  await expect(page).toHaveTitle('Create organisation · Featherboard');
  await expect(page.getByRole('heading', { name: 'Name your organisation' })).toBeVisible();
  await expect(page.getByLabel('Organisation name')).toBeVisible();
  await expect(page.getByLabel('Organisation URL')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Continue' })).toBeVisible();
});
