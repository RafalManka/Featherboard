const { test, expect } = require('@playwright/test');

test('Stripe E2E server is healthy', async ({ request }) => {
  const response = await request.get('/healthz');

  expect(response.status()).toBe(200);
  expect(await response.text()).toBe('ok');
});
