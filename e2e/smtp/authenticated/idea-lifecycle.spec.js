const {test, expect} = require('@playwright/test');

test("Admin creates an idea", async ({page}) => {
    await page.goto('/acme-e2e/ideas/new');

    await page.getByLabel('Title').fill('Customers need dark mode');
    await page
        .getByLabel('Description')
        .fill('Customers want a dark mode option for using the feedback board at night.');
    await page.getByRole('button', { name: 'Submit' }).click();

    await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);

    const heading = page.getByRole('heading', { name: 'Customers need dark mode' });
    await expect(heading).toBeVisible();

    // verify the idea appears on the board and can be opened

    await page.goto('/acme-e2e');

    const ideaLink = page.getByRole('link', { name: 'Customers need dark mode' });
    await ideaLink.click();

    await expect(page).toHaveURL(/\/acme-e2e\/ideas\/\d+$/);

    const voteButton = page.locator('.vote-btn');

    await expect(voteButton).toBeVisible();
    await voteButton.click();

    await expect(voteButton).toHaveClass(/voted/);

    await page.getByLabel('Comment').fill('This would be very useful.');
    await page.getByRole('button', { name: 'Comment' }).click();

    await expect(page.getByText('This would be very useful.')).toBeVisible();

})