const { test, expect } = require('@playwright/test');

test('anonymous visitors can post, vote, and comment', async ({ page }) => {
  const title = `Anonymous feedback ${Date.now()}`;
  const description = 'This feedback was submitted without signing in.';
  const commentName = 'Anonymous Commenter';
  const commentContent = 'I like this idea very much - 10/10';

  await page.goto('/default/ideas/new');

  await expect(page).toHaveTitle('Submit an idea');
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Description').fill(description);
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/default\/ideas\/\d+$/);
  await expect(page.getByRole('heading', { name: title })).toBeVisible();
  await expect(page.getByText(description)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Update status' })).not.toBeVisible();
  await expect(page.getByRole('button', { name: 'Delete idea' })).not.toBeVisible();

  const settingsResponse = await page.goto('/default/settings/site');
  expect(settingsResponse).not.toBeNull();
  expect(settingsResponse.status()).toBe(403);
  await page.goBack();

  const voteButton = page.locator('.vote-btn');
  await expect(voteButton).toBeVisible();
  await voteButton.click();
  await expect(voteButton).toHaveClass(/voted/);

  await page.getByRole('textbox', { name: 'Name' }).fill(commentName);
  await page.getByRole('textbox', { name: 'Comment' }).fill(commentContent);
  await page.getByRole('button', { name: 'Comment' }).click();

  await expect(page).toHaveURL(/\/default\/ideas\/\d+$/);
  const commentCard = page.locator('.comment-card').filter({
    hasText: commentContent,
  });
  await expect(commentCard).toContainText(commentName);
  await expect(commentCard).toContainText(commentContent);

  await page.getByRole('link', { name: '← Back to ideas' }).click();
  await expect(page).toHaveURL(/\/default$/);
  await expect(page.getByRole('link', { name: title })).toBeVisible();
});
