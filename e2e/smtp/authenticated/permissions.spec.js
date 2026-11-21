const fs = require('fs');
const { test, expect } = require('@playwright/test');

const EMAIL_FILE = '/private/tmp/featherboard-e2e-email.eml';

test('non-admin users can view ideas but not administer the site', async ({
  page,
  browser,
}) => {
  const ideaTitle = `Permission test ${Date.now()}`;

  await page.goto('/acme-e2e/ideas/new');
  await page.getByLabel('Title').fill(ideaTitle);
  await page.getByLabel('Description').fill('Testing non-admin permissions.');
  await page.getByRole('button', { name: 'Submit' }).click();
  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);

  const memberContext = await browser.newContext({
    storageState: { cookies: [], origins: [] },
  });
  const memberPage = await memberContext.newPage();

  try {
    await memberPage.goto('/acme-e2e/login');
    await memberPage.getByLabel('Email address').fill('member@example.com');
    await memberPage
      .getByRole('button', { name: 'Send me a sign-in link' })
      .click();

    let magicLink = '';
    await expect.poll(() => {
      try {
        const email = fs
          .readFileSync(EMAIL_FILE, 'utf8')
          .replace(/=\r?\n/g, '')
          .replace(/=3D/g, '=');
        magicLink = email.match(
          /http:\/\/127\.0\.0\.1:3101\/acme-e2e\/login\/check-email\/[a-zA-Z0-9-]+/,
        )?.[0] ?? '';
      } catch {
        magicLink = '';
      }
      return magicLink;
    }).toBeTruthy();

    await memberPage.goto(magicLink);
    await expect(memberPage).toHaveURL(/\/acme-e2e$/);

    const settingsResponse = await memberPage.goto('/acme-e2e/settings/site');
    expect(settingsResponse).not.toBeNull();
    expect(settingsResponse.status()).toBe(403);

    await memberPage.goto(page.url());
    await expect(memberPage.getByRole('heading', { name: ideaTitle })).toBeVisible();
    await expect(
      memberPage.getByRole('button', { name: 'Update status' }),
    ).not.toBeVisible();
    await expect(
      memberPage.getByRole('button', { name: 'Delete idea' }),
    ).not.toBeVisible();
  } finally {
    await memberContext.close();
  }
});
