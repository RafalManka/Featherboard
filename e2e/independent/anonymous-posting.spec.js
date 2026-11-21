const { test, expect } = require('@playwright/test');

test('anonymous visitors can post an idea', async ({ page }) => {
  const title = `Anonymous feedback ${Date.now()}`;
  const description = 'This feedback was submitted without signing in.';

  await page.goto('/default/ideas/new');

  await expect(page).toHaveTitle('Submit an idea');
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Description').fill(description);
  await page.getByRole('button', { name: 'Submit' }).click();

  await expect(page).toHaveURL(/\/default\/ideas\/\d+$/);
  await expect(page.getByRole('heading', { name: title })).toBeVisible();
  await expect(page.getByText(description)).toBeVisible();

  await page.goto('/default');
  await expect(page.getByRole('link', { name: title })).toBeVisible();

});
