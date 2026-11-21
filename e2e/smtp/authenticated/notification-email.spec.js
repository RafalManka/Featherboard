const fs = require('fs');
const { test, expect } = require('@playwright/test');

const EMAIL_FILE = '/private/tmp/featherboard-e2e-email.eml';

test('admin receives an absolute-link email for a member comment', async ({
  page,
  browser,
}) => {
  const ideaTitle = `Notification idea ${Date.now()}`;
  const commentBody = 'A member left this notification test comment.';

  await page.goto('/acme-e2e/ideas/new');
  await page.getByLabel('Title').fill(ideaTitle);
  await page.getByLabel('Description').fill('Testing comment notifications.');
  await page.getByRole('button', { name: 'Submit' }).click();
  await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);

  const ideaId = page.url().match(/\/ideas\/(\d+)$/)?.[1];
  expect(ideaId).toBeTruthy();

  const memberContext = await browser.newContext({
    storageState: { cookies: [], origins: [] },
  });
  const memberPage = await memberContext.newPage();

  try {
    await memberPage.goto('/acme-e2e/login');
    await memberPage.getByLabel('Email address').fill('notification-member@example.com');
    await memberPage
      .getByRole('button', { name: 'Send me a sign-in link' })
      .click();

    let loginLink = '';
    await expect.poll(() => {
      try {
        const email = fs
          .readFileSync(EMAIL_FILE, 'utf8')
          .replace(/=\r?\n/g, '')
          .replace(/=3D/g, '=');
        loginLink = email.match(
          /http:\/\/127\.0\.0\.1:3101\/acme-e2e\/login\/check-email\/[a-zA-Z0-9-]+/,
        )?.[0] ?? '';
      } catch {
        loginLink = '';
      }
      return loginLink;
    }).toBeTruthy();

    await memberPage.goto(loginLink);
    await memberPage.goto(`/acme-e2e/ideas/${ideaId}`);
    await memberPage.getByLabel('Comment').fill(commentBody);
    await memberPage.getByRole('button', { name: 'Comment' }).click();

    await expect(memberPage.getByText(commentBody)).toBeVisible();

    await expect.poll(() => {
      try {
        return fs.readFileSync(EMAIL_FILE, 'utf8');
      } catch {
        return '';
      }
    }).toContain(`http://127.0.0.1:3101/acme-e2e/ideas/${ideaId}`);

    const email = fs.readFileSync(EMAIL_FILE, 'utf8');
    expect(email).toContain('New comment on:');
    expect(email).toContain(commentBody);
  } finally {
    await memberContext.close();
  }
});
