const { test, expect } = require('@playwright/test');
const { createIdea } = require('../../helpers');

test('admin can delete an idea', async ({ page }) => {
  const title = `Delete idea ${Date.now()}`;

  await createIdea(page, {
    orgSlug: 'acme-e2e',
    title,
    description: 'This idea will be deleted.',
  });
  await page.getByRole('button', { name: 'Delete idea' }).click();

  await expect(page).toHaveURL(/\/acme-e2e$/);
  await expect(page.getByRole('link', { name: title })).not.toBeVisible();
});
