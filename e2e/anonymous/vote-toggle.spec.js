const { test, expect } = require('@playwright/test');

test('anonymous voting toggles on and off', async ({ page }) => {
  const title = `Vote toggle ${Date.now()}`;

  await page.goto('/default/ideas/new');
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Description').fill('Testing vote toggle behavior.');
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/default\/ideas\/\d+$/);

  const voteButton = page.locator('.vote-btn');
  const voteCount = voteButton.locator('.vote-count');

  await expect(voteCount).toHaveText('0');
  await voteButton.click();
  await expect(voteButton).toHaveClass(/voted/);
  await expect(voteCount).toHaveText('1');

  await voteButton.click();
  await expect(voteButton).not.toHaveClass(/voted/);
  await expect(voteCount).toHaveText('0');
});
