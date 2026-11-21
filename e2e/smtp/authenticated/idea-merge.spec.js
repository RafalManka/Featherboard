const { test, expect } = require('@playwright/test');

test('admin can merge a duplicate idea into a canonical idea', async ({ page }) => {
  const canonicalTitle = `Canonical idea ${Date.now()}`;
  const duplicateTitle = `Duplicate idea ${Date.now()}`;

  for (const title of [canonicalTitle, duplicateTitle]) {
    await page.goto('/acme-e2e/ideas/new');
    await page.getByLabel('Title').fill(title);
    await page.getByLabel('Description').fill(`Description for ${title}.`);
    await page.getByRole('button', { name: 'Submit' }).click();
    await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);
  }

  const duplicateId = page.url().match(/\/ideas\/(\d+)$/)?.[1];
  expect(duplicateId).toBeTruthy();

  await page.goto(`/acme-e2e/ideas/${duplicateId}`);
  const mergeSelect = page.locator('select[name="canonical_idea_id"]');
  await mergeSelect.selectOption({ label: canonicalTitle });
  await page.getByRole('button', { name: 'Merge' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);
  await expect(page.getByRole('heading', { name: canonicalTitle })).toBeVisible();

  await page.goto('/acme-e2e');
  await expect(page.getByRole('link', { name: canonicalTitle })).toBeVisible();
  await expect(page.getByRole('link', { name: duplicateTitle })).not.toBeVisible();
});
