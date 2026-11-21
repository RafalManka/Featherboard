const { test, expect } = require('@playwright/test');

test('admin can create, vote on, and comment on an idea', async ({ page }) => {
  const title = `Admin feedback ${Date.now()}`;

  await page.goto('/acme-e2e/ideas/new');

  await page.getByLabel('Title').fill(title);
  await page
    .getByLabel('Description')
    .fill('Testing the authenticated idea workflow.');
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);
  await expect(page.getByRole('heading', { name: title })).toBeVisible();

  await page.goto('/acme-e2e');
  await page.getByRole('link', { name: title }).click();
  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);

  const voteButton = page.locator('.vote-btn');
  await expect(voteButton).toBeVisible();
  await voteButton.click();
  await expect(voteButton).toHaveClass(/voted/);

  await page.getByLabel('Comment').fill('This would be very useful.');
  await page.getByRole('button', { name: 'Comment' }).click();
  await expect(page.getByText('This would be very useful.')).toBeVisible();
});
