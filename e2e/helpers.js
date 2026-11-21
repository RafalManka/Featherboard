const fs = require('fs');

const EMAIL_FILE = '/private/tmp/featherboard-e2e-email.eml';

async function createIdea(page, { orgSlug, title, description }) {
  await page.goto(`/${orgSlug}/ideas/new`);
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Description').fill(description);
  await page.getByRole('button', { name: 'Submit' }).click();
  await page.waitForURL(new RegExp(`/${orgSlug}/ideas/\\d+$`));

  return page.url().match(/\/ideas\/(\d+)$/)?.[1];
}

function readEmail() {
  try {
    return fs
      .readFileSync(EMAIL_FILE, 'utf8')
      .replace(/=\r?\n/g, '')
      .replace(/=3D/g, '=');
  } catch {
    return '';
  }
}

async function waitForMagicLink(expect, pattern) {
  await expect.poll(() => readEmail().match(pattern)?.[0] ?? '').toBeTruthy();
  return readEmail().match(pattern)?.[0] ?? '';
}

async function waitForEmail(expect, text) {
  await expect.poll(() => readEmail()).toContain(text);
  return readEmail();
}

module.exports = {
  EMAIL_FILE,
  createIdea,
  readEmail,
  waitForEmail,
  waitForMagicLink,
};
