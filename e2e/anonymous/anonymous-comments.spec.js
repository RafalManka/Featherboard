const { test, expect } = require('@playwright/test');

test('anonymous visitors can validate comments and post replies', async ({ page }) => {
  const title = `Comment test ${Date.now()}`;

  await page.goto('/default/ideas/new');
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Description').fill('Testing comments and replies.');
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/default\/ideas\/\d+$/);

  await page.getByRole('button', { name: 'Comment' }).click();
  await expect(page.getByText('Please provide your name and a comment.')).toBeVisible();

  await page.getByRole('textbox', { name: 'Name' }).fill('Original commenter');
  await page.getByRole('textbox', { name: 'Comment' }).fill('This is the original comment.');
  await page.getByRole('button', { name: 'Comment' }).click();
  await expect(page.getByText('This is the original comment.')).toBeVisible();

  const comment = page.locator('.comment-card').filter({
    hasText: 'This is the original comment.',
  });
  await comment.locator('summary').click();
  await comment.locator('input[name="author_name"]').fill('Reply author');
  await comment.getByPlaceholder('Reply').fill('This is a reply.');
  await comment.getByRole('button', { name: 'Reply' }).click();

  await expect(page.getByText('This is a reply.')).toBeVisible();
});
