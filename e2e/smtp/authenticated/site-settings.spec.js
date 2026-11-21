const { test, expect } = require('@playwright/test');

test('admin can save branding and GitHub settings safely', async ({ page }) => {
  await page.goto('/acme-e2e/settings/site');

  const brandingForm = page.locator('form[action$="/settings/site/branding"]');
  await brandingForm.getByLabel('Accent color').fill('#123456');
  await brandingForm.getByLabel('Logo URL').fill('https://example.com/logo.svg');
  await brandingForm.getByRole('button', { name: 'Save' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/settings\/site$/);
  await expect(page.getByLabel('Accent color')).toHaveValue('#123456');
  await expect(page.getByLabel('Logo URL')).toHaveValue('https://example.com/logo.svg');

  const githubForm = page.locator('form[action$="/settings/site/github"]');
  await githubForm.getByLabel('Client ID').fill('github-client-id');
  await githubForm.getByLabel('Client Secret').fill('github-client-secret');
  await githubForm.getByRole('button', { name: 'Save' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/settings\/site$/);
  await expect(page.getByLabel('Client ID')).toHaveValue('github-client-id');
  await expect(page.getByLabel('Client Secret')).toHaveValue('********');

  await githubForm.getByLabel('Client ID').fill('updated-client-id');
  await githubForm.getByLabel('Client Secret').fill('********');
  await githubForm.getByRole('button', { name: 'Save' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/settings\/site$/);
  await expect(page.getByLabel('Client ID')).toHaveValue('updated-client-id');
  await expect(page.getByLabel('Client Secret')).toHaveValue('********');
});
