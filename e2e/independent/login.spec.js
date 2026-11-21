const { test, expect } = require('@playwright/test');

test('email login page renders correctly', async ({ page }) => {
  await page.goto('/default/login');

  await expect(page).toHaveTitle('Log in');
  await expect(page.getByRole('heading', { name: 'Sign in to Default' })).toBeVisible();
  await expect(page.getByLabel('Email address')).toBeVisible();
  await expect(page).toHaveScreenshot('login-page.png', { fullPage: true });
});
