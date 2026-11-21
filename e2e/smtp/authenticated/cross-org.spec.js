const fs = require('fs');
const { test, expect } = require('@playwright/test');

const EMAIL_FILE = '/private/tmp/featherboard-e2e-email.eml';

test('organization data and administration stay isolated', async ({ browser }) => {
  const secondOrgContext = await browser.newContext({
    storageState: { cookies: [], origins: [] },
  });
  const secondOrgPage = await secondOrgContext.newPage();
  const secondOrgTitle = `Second-org idea ${Date.now()}`;

  try {
    await secondOrgPage.goto('/setup');
    await secondOrgPage.getByLabel('Organisation name').fill('Beta E2E');
    await secondOrgPage.getByLabel('Organisation URL').fill('beta-e2e');
    await secondOrgPage.getByRole('button', { name: 'Continue' }).click();

    await expect(secondOrgPage).toHaveURL(/\/setup\/admin$/);
    await secondOrgPage.getByLabel('Email address').fill('beta-admin@example.com');
    await secondOrgPage.getByRole('button', { name: 'Send sign-in link' }).click();

    let setupLink = '';
    await expect.poll(() => {
      try {
        const email = fs
          .readFileSync(EMAIL_FILE, 'utf8')
          .replace(/=\r?\n/g, '')
          .replace(/=3D/g, '=');
        setupLink = email.match(
          /http:\/\/127\.0\.0\.1:3101\/setup\/confirm\/[a-zA-Z0-9-]+/,
        )?.[0] ?? '';
      } catch {
        setupLink = '';
      }
      return setupLink;
    }).toBeTruthy();

    await secondOrgPage.goto(setupLink);
    await expect(secondOrgPage).toHaveURL(/\/beta-e2e$/);

    await secondOrgPage.goto('/beta-e2e/ideas/new');
    await secondOrgPage.getByLabel('Title').fill(secondOrgTitle);
    await secondOrgPage
      .getByLabel('Description')
      .fill('This idea belongs only to the second organization.');
    await secondOrgPage.getByRole('button', { name: 'Submit' }).click();
    await expect(secondOrgPage).toHaveURL(/\/beta-e2e\/ideas\/\d+$/);

    const ideaId = secondOrgPage.url().match(/\/ideas\/(\d+)$/)?.[1];
    expect(ideaId).toBeTruthy();

    const settingsResponse = await secondOrgPage.goto('/acme-e2e/settings/site');
    expect(settingsResponse).not.toBeNull();
    expect(settingsResponse.status()).toBe(403);

    await secondOrgPage.goto('/acme-e2e');
    await expect(secondOrgPage.getByRole('link', { name: secondOrgTitle })).not.toBeVisible();

    const crossOrgIdeaResponse = await secondOrgPage.goto(`/acme-e2e/ideas/${ideaId}`);
    expect(crossOrgIdeaResponse).not.toBeNull();
    expect(crossOrgIdeaResponse.status()).toBe(404);
  } finally {
    await secondOrgContext.close();
  }
});
