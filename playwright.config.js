const { defineConfig } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './e2e',
  testIgnore: ['**/setup-email.spec.js'],
  timeout: 30_000,
  fullyParallel: true,
  reporter: 'list',
  use: {
    baseURL: 'http://127.0.0.1:3100',
    browserName: 'chromium',
    viewport: { width: 1280, height: 900 },
    trace: 'retain-on-failure',
    // Uncomment for a visible, slowed-down browser during local debugging.
    // headless: false,
    // launchOptions: {
    //   slowMo: 1000,
    // },
  },
  webServer: {
    command: 'DATABASE_URL=sqlite:/private/tmp/featherboard-playwright.db?mode=rwc PORT=3100 PUBLIC_URL=http://127.0.0.1:3100 cargo run --bin featherboard',
    url: 'http://127.0.0.1:3100/healthz',
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
