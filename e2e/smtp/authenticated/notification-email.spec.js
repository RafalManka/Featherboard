const { test, expect } = require('@playwright/test');
const {
  createIdea,
  readEmail,
  waitForEmail,
  waitForMagicLink,
} = require('../../helpers');

test('admin receives an absolute-link email for a member comment', async ({
  page,
  browser,
}) => {
  const ideaTitle = `Notification idea ${Date.now()}`;
  const commentBody = 'A member left this notification test comment.';

  const ideaId = await createIdea(page, {
    orgSlug: 'acme-e2e',
    title: ideaTitle,
    description: 'Testing comment notifications.',
  });
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

    const loginLink = await waitForMagicLink(
      expect,
      /http:\/\/127\.0\.0\.1:3101\/acme-e2e\/login\/check-email\/[a-zA-Z0-9-]+/,
    );

    await memberPage.goto(loginLink);
    await memberPage.goto(`/acme-e2e/ideas/${ideaId}`);
    await memberPage.getByLabel('Comment').fill(commentBody);
    await memberPage.getByRole('button', { name: 'Comment' }).click();

    await expect(memberPage.getByText(commentBody)).toBeVisible();

    await waitForEmail(
      expect,
      `http://127.0.0.1:3101/acme-e2e/ideas/${ideaId}`,
    );

    const email = readEmail();
    expect(email).toContain('New comment on:');
    expect(email).toContain(commentBody);
  } finally {
    await memberContext.close();
  }
});

test('members receive status and shipped changelog notifications', async ({
  page,
  browser,
}) => {
  const ideaTitle = `Notification lifecycle idea ${Date.now()}`;
  const changelogTitle = `Notification release ${Date.now()}`;

  const memberContext = await browser.newContext({
    storageState: { cookies: [], origins: [] },
  });
  const memberPage = await memberContext.newPage();

  try {
    await memberPage.goto('/acme-e2e/login');
    await memberPage.getByLabel('Email address').fill('lifecycle-member@example.com');
    await memberPage
      .getByRole('button', { name: 'Send me a sign-in link' })
      .click();

    const loginLink = await waitForMagicLink(
      expect,
      /http:\/\/127\.0\.0\.1:3101\/acme-e2e\/login\/check-email\/[a-zA-Z0-9-]+/,
    );

    await memberPage.goto(loginLink);
    await memberPage.goto('/acme-e2e/ideas/new');
    await memberPage.getByLabel('Title').fill(ideaTitle);
    await memberPage
      .getByLabel('Description')
      .fill('Testing status and shipped notifications.');
    await memberPage.getByRole('button', { name: 'Submit' }).click();
    await expect(memberPage).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);

    const ideaId = memberPage.url().match(/\/ideas\/(\d+)$/)?.[1];
    expect(ideaId).toBeTruthy();

    await page.goto(`/acme-e2e/ideas/${ideaId}`);
    await page.locator('select[name="status"]').selectOption('planned');
    await page.getByRole('button', { name: 'Update status' }).click();

    await waitForEmail(
      expect,
      `http://127.0.0.1:3101/acme-e2e/ideas/${ideaId}`,
    );

    let email = readEmail();
    expect(email).toContain('Status changed to Planned:');
    expect(email).toContain(ideaTitle);

    await page.goto('/acme-e2e/changelogs/new');
    await page.getByLabel('Title').fill(changelogTitle);
    await page
      .getByLabel('Description')
      .fill('Testing the changelog-created notification.');
    await page.getByRole('button', { name: 'Submit' }).click();
    await expect(page).toHaveURL(/\/acme-e2e\/changelogs\/\d+$/);

    const changelogId = page.url().match(/\/changelogs\/(\d+)$/)?.[1];
    expect(changelogId).toBeTruthy();
    const changelogUrl = `http://127.0.0.1:3101/acme-e2e/changelogs/${changelogId}`;

    await waitForEmail(expect, changelogUrl);

    email = readEmail();
    expect(email).toContain('New changelog published:');
    expect(email).toContain(changelogTitle);

    await page.goto(`/acme-e2e/ideas/${ideaId}`);
    await page.locator('select[name="changelog_id"]').selectOption(changelogId);
    await page.getByRole('button', { name: 'Assign changelog' }).click();

    await waitForEmail(expect, changelogUrl);

    email = readEmail();
    expect(email).toContain('Your feedback is now shipped:');
    expect(email).toContain(changelogTitle);
  } finally {
    await memberContext.close();
  }
});
