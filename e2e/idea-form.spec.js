const { test, expect } = require('@playwright/test');

test('idea form validates title and description lengths', async ({ page }) => {
  const response = await page.goto('/default/ideas/new');

  expect(response).not.toBeNull();
  expect(response.status()).toBe(200);
  await expect(page).toHaveTitle('Submit an idea');
  await expect(page.getByLabel('Title')).toBeVisible();
  await expect(page.getByLabel('Description')).toBeVisible();

  await page.getByLabel('Title').fill('Hi');
  await page.getByLabel('Description').fill('Too short');
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page.getByText('Title must be at least 3 characters')).toBeVisible();
  await expect(page.getByText('Description must be at least 10 characters')).toBeVisible();
});
