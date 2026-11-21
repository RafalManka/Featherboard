const { test, expect } = require('@playwright/test');

test('changelog form validates title and description lengths', async ({ page }) => {
  await page.goto('/acme-e2e/changelogs/new');

  await page.getByLabel('Title').fill('Hi');
  await page.getByLabel('Description').fill('Too short');
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page.getByText('Title must be at least 3 characters')).toBeVisible();
  await expect(page.getByText('Description must be at least 10 characters')).toBeVisible();
});
