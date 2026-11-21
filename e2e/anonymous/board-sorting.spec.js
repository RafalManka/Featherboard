const { test, expect } = require('@playwright/test');

async function createIdea(page, title) {
  await page.goto('/default/ideas/new');
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Description').fill(`Description for ${title}.`);
  await page.getByRole('button', { name: 'Submit' }).click();
  await expect(page).toHaveURL(/\/default\/ideas\/\d+$/);
}

test('public board sorts ideas by newest and top votes', async ({ page }) => {
  const olderTitle = `Older idea ${Date.now()}`;
  const newerTitle = `Newer idea ${Date.now()}`;

  await createIdea(page, olderTitle);
  const voteButton = page.locator('.vote-btn');
  await voteButton.click();
  await expect(voteButton).toHaveClass(/voted/);

  await createIdea(page, newerTitle);

  await page.goto('/default?sort=new');
  let titles = await page.locator('.idea-title').allTextContents();
  expect(titles.indexOf(newerTitle)).toBeLessThan(titles.indexOf(olderTitle));

  await page.goto('/default?sort=top');
  titles = await page.locator('.idea-title').allTextContents();
  expect(titles.indexOf(olderTitle)).toBeLessThan(titles.indexOf(newerTitle));

  await expect(page).toHaveScreenshot('populated-board.png', {
    fullPage: false,
    maxDiffPixels: 500,
    style: '.idea-card:nth-child(n + 3) { display: none !important; }',
    mask: [
      page.locator('.idea-title'),
      page.locator('.idea-excerpt'),
      page.locator('time'),
    ],
  });
});
