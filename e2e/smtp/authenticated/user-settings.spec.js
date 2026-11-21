const { test, expect } = require('@playwright/test');

test('users can save notification preferences', async ({ page }) => {
  await page.goto('/acme-e2e/settings/user');

  const comments = page.getByRole('checkbox', { name: 'New comments' });
  const statusChanges = page.getByRole('checkbox', { name: 'Status changes' });
  const shipped = page.getByRole('checkbox', { name: 'Ideas you follow are shipped' });

  await comments.check();
  await statusChanges.check();
  await shipped.check();
  await page.getByRole('button', { name: 'Save preferences' }).click();

  await expect(page).toHaveURL(/\/acme-e2e\/settings\/user$/);
  await expect(comments).toBeChecked();
  await expect(statusChanges).toBeChecked();
  await expect(shipped).toBeChecked();
});
