const { test, expect } = require('@playwright/test');

test('deleting a changelog preserves and unlinks its ideas', async ({ page }) => {
  const ideaTitle = `Delete changelog idea ${Date.now()}`;
  const changelogTitle = `Temporary release ${Date.now()}`;

  await page.goto('/acme-e2e/ideas/new');
  await page.getByLabel('Title').fill(ideaTitle);
  await page
    .getByLabel('Description')
    .fill('This idea should survive changelog deletion.');
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);
  const ideaId = page.url().match(/\/ideas\/(\d+)$/)?.[1];
  expect(ideaId).toBeTruthy();

  await page.goto('/acme-e2e/changelogs/new');
  await page.getByLabel('Title').fill(changelogTitle);
  await page
    .getByLabel('Description')
    .fill('This changelog will be deleted.');
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/changelogs\/\d+$/);
  const changelogId = page.url().match(/\/changelogs\/(\d+)$/)?.[1];
  expect(changelogId).toBeTruthy();

  await page.goto(`/acme-e2e/ideas/${ideaId}`);
  const changelogSelect = page.locator('select[name="changelog_id"]');
  await changelogSelect.selectOption({ label: changelogTitle });
  await page.getByRole('button', { name: 'Assign changelog' }).click();
  await expect(changelogSelect).toHaveValue(changelogId);

  await page.goto(`/acme-e2e/changelogs/${changelogId}`);
  await page.getByRole('button', { name: 'Delete changelog' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/changelogs$/);
  await expect(page.getByRole('link', { name: changelogTitle })).not.toBeVisible();

  await page.goto(`/acme-e2e/ideas/${ideaId}`);
  await expect(page.getByRole('heading', { name: ideaTitle })).toBeVisible();
  await expect(page.locator('select[name="changelog_id"]')).toHaveValue('');
});
