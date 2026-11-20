const { defineConfig } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './e2e',
  testIgnore: ['**/setup.spec.js'],
  timeout: 30_000,
  fullyParallel: false,
  reporter: 'list',
  use: {
    baseURL: 'http://127.0.0.1:3101',
    browserName: 'chromium',
    viewport: { width: 1280, height: 900 },
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'cargo run --features e2e --bin e2e-server',
    url: 'http://127.0.0.1:3101/healthz',
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
