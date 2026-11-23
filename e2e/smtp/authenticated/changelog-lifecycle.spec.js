const { test, expect } = require('@playwright/test');

test('admin can publish an idea through a changelog', async ({ page }) => {
  const ideaTitle = 'Release idea';
  const changelogTitle = 'Release';
  const ideaDescription = 'Testing the changelog assignment workflow.';
  const changelogDescription = 'Testing a published changelog entry.';

  await page.goto('/acme-e2e/ideas/new');
  await page.getByLabel('Title').fill(ideaTitle);
  await page.getByLabel('Description').fill(ideaDescription);
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);
  const ideaUrl = page.url();
  const ideaId = ideaUrl.match(/\/ideas\/(\d+)$/)?.[1];
  expect(ideaId).toBeTruthy();

  await page.goto('/acme-e2e/changelogs/new');
  await expect(page).toHaveTitle('Submit a changelog');
  await page.getByLabel('Title').fill(changelogTitle);
  await page.getByLabel('Description').fill(changelogDescription);
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/changelogs\/\d+$/);
  const changelogUrl = page.url();
  const changelogId = changelogUrl.match(/\/changelogs\/(\d+)$/)?.[1];
  expect(changelogId).toBeTruthy();

  await page.goto(`/acme-e2e/ideas/${ideaId}`);
  const changelogSelect = page.locator('select[name="changelog_id"]');
  await changelogSelect.selectOption({ label: changelogTitle });
  await page.getByRole('button', { name: 'Assign changelog' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);
  await expect(changelogSelect).toHaveValue(changelogId);

  await page.goto(`/acme-e2e/changelogs/${changelogId}`);
  await expect(page).toHaveTitle(changelogTitle);
  await expect(page.getByRole('heading', { name: changelogTitle })).toBeVisible();
  await expect(page.getByText(changelogDescription)).toBeVisible();
  await expect(page.getByRole('link', { name: ideaTitle })).toBeVisible();
  await expect(page).toHaveScreenshot('changelog-detail.png', {
    fullPage: true,
    mask: [
      page.locator('.idea-main h1'),
      page.locator('.idea-description'),
      page.locator('time'),
      page.locator('.sidebar-shipped-ideas'),
    ],
  });
});
