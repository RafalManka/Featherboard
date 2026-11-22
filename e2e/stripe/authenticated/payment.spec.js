const { test, expect } = require('@playwright/test');

async function stripePost(request, secretKey, endpoint, form) {
  const response = await request.post(`https://api.stripe.com/v1/${endpoint}`, {
    headers: { Authorization: `Bearer ${secretKey}` },
    form,
  });

  if (!response.ok()) {
    throw new Error(`Stripe ${endpoint} failed (${response.status()}): ${await response.text()}`);
  }
  return response.json();
}

test('successful Stripe subscription activates the organization through webhooks', async ({
  page,
  request,
}) => {
  const secretKey = process.env.STRIPE_SECRET_KEY;
  const priceId = process.env.STRIPE_PRICE_ID;

  expect(secretKey).toBeTruthy();
  expect(priceId).toBeTruthy();

  await page.goto('/stripe-e2e/settings/site');
  await expect(page.getByText('Your workspace is on the Free plan.')).toBeVisible();

  const idempotencyKey = await page
    .locator('input[name="idempotency_key"]')
    .inputValue();

  const paymentMethod = await stripePost(request, secretKey, 'payment_methods', {
    type: 'card',
    'card[token]': 'tok_visa',
  });

  const customer = await stripePost(request, secretKey, 'customers', {
    email: 'stripe-admin@example.com',
    name: 'Stripe E2E Tester',
  });

  await stripePost(request, secretKey, `payment_methods/${paymentMethod.id}/attach`, {
    customer: customer.id,
  });

  await stripePost(request, secretKey, `customers/${customer.id}`, {
    'invoice_settings[default_payment_method]': paymentMethod.id,
  });

  await stripePost(request, secretKey, 'subscriptions', {
    customer: customer.id,
    'items[0][price]': priceId,
    'items[0][quantity]': '1',
    payment_behavior: 'error_if_incomplete',
    // e2e-server creates the default organization before this test organization.
    'metadata[featherboard_org_id]': '2',
    'metadata[idempotency_key]': idempotencyKey,
  });

  await expect
    .poll(
      async () => {
        await page.reload();
        return page.getByText('Your subscription is active.').isVisible();
      },
      { timeout: 30_000, intervals: [500, 1_000, 2_000] },
    )
    .toBe(true);
});
