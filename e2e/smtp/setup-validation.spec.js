const { test, expect } = require('@playwright/test');

test('setup validates organization name and URL', async ({ page }) => {
  await page.goto('/setup');

  await page.getByLabel('Organisation name').fill('A');
  await page.getByLabel('Organisation URL').fill('Invalid URL');
  await page.getByRole('button', { name: 'Continue' }).click();

  await expect(
    page.getByText('Use an organisation name between 2 and 80 characters.'),
  ).toBeVisible();
  await expect(
    page.getByText('Use 3–63 lowercase letters or numbers, separated by single hyphens.'),
  ).toBeVisible();
  await expect(page).toHaveURL(/\/setup$/);
});
