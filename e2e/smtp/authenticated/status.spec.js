const { test, expect } = require('@playwright/test');

test('admin can move an idea to the planned roadmap', async ({ page }) => {
  const title = `Status test ${Date.now()}`;

  await page.goto('/acme-e2e/ideas/new');

  await page.getByLabel('Title').fill(title);
  await page
    .getByLabel('Description')
    .fill('Testing the admin status workflow.');
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);

  const statusSelect = page.locator('select[name="status"]');
  await statusSelect.selectOption('planned');
  await page.getByRole('button', { name: 'Update status' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);
  await expect(statusSelect).toHaveValue('planned');

  await page.goto('/acme-e2e/roadmap');
  await expect(page).toHaveTitle('Roadmap');
  await expect(page.getByRole('heading', { name: 'Roadmap' })).toBeVisible();
  await expect(page.getByRole('link', { name: title })).toBeVisible();
});
