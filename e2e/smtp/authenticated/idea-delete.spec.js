const { test, expect } = require('@playwright/test');

test('admin can delete an idea', async ({ page }) => {
  const title = `Delete idea ${Date.now()}`;

  await page.goto('/acme-e2e/ideas/new');
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Description').fill('This idea will be deleted.');
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);
  await page.getByRole('button', { name: 'Delete idea' }).click();

  await expect(page).toHaveURL(/\/acme-e2e$/);
  await expect(page.getByRole('link', { name: title })).not.toBeVisible();
});
