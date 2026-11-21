const { test, expect } = require('@playwright/test');
const { createIdea } = require('../../helpers');

test('public board filters ideas by status', async ({ page }) => {
  const title = `Planned idea ${Date.now()}`;

  await createIdea(page, {
    orgSlug: 'acme-e2e',
    title,
    description: 'Testing the public status filter.',
  });
  const statusSelect = page.locator('select[name="status"]');
  await statusSelect.selectOption('planned');
  await page.getByRole('button', { name: 'Update status' }).click();

  await page.goto('/acme-e2e?status=planned&sort=new');
  await expect(page.getByRole('link', { name: title })).toBeVisible();

  await page.goto('/acme-e2e?status=open&sort=new');
  await expect(page.getByRole('link', { name: title })).not.toBeVisible();
});
