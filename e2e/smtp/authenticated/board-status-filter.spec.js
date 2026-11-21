const { test, expect } = require('@playwright/test');

test('public board filters ideas by status', async ({ page }) => {
  const title = `Planned idea ${Date.now()}`;

  await page.goto('/acme-e2e/ideas/new');
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Description').fill('Testing the public status filter.');
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);
  const statusSelect = page.locator('select[name="status"]');
  await statusSelect.selectOption('planned');
  await page.getByRole('button', { name: 'Update status' }).click();

  await page.goto('/acme-e2e?status=planned&sort=new');
  await expect(page.getByRole('link', { name: title })).toBeVisible();

  await page.goto('/acme-e2e?status=open&sort=new');
  await expect(page.getByRole('link', { name: title })).not.toBeVisible();
});
