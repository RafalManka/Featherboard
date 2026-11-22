const {defineConfig} = require('@playwright/test');

const stripeE2EEnabled = process.env.STRIPE_E2E === '1';
const selectedProject = process.env.E2E_PROJECT;
const shouldStart = (...projects) =>
    !selectedProject || projects.includes(selectedProject);

module.exports = defineConfig({
    testDir: './e2e',
    timeout: 30_000,
    fullyParallel: false,
    workers: 1,
    reporter: 'list',
    use: {
        browserName: 'chromium',
        viewport: {width: 1280, height: 900},
        trace: 'retain-on-failure',
        // Uncomment for a visible, slowed-down browser during local debugging.
        // headless: false,
        // launchOptions: {
        //   slowMo: 1000,
        // },
    },
    projects: [
        {
            name: 'independent',
            testMatch: /independent\/.*\.spec\.js/,
            use: {
                baseURL: 'http://127.0.0.1:3100',
            },
        },
        {
            name: 'anonymous',
            testMatch: /anonymous\/.*\.spec\.js/,
            use: {
                baseURL: 'http://127.0.0.1:3102',
            },
        },
        {
            name: 'smtp-setup',
            testMatch: /smtp\/setup.*\.spec\.js/,
            use: {
                baseURL: 'http://127.0.0.1:3101',
            },
        },
        {
            name: 'smtp-authenticated',
            dependencies: ['smtp-setup'],
            testMatch: /smtp\/authenticated\/.*\.spec\.js/,
            use: {
                baseURL: 'http://127.0.0.1:3101',
                storageState: 'playwright/.auth/admin.json',
            },
        },
        ...(stripeE2EEnabled ? [{
            name: 'stripe-setup',
            testMatch: /stripe\/setup\.spec\.js/,
            use: {
                baseURL: 'http://127.0.0.1:3103',
            },
        }, {
            name: 'stripe-authenticated',
            dependencies: ['stripe-setup'],
            testMatch: /stripe\/authenticated\/.*\.spec\.js/,
            use: {
                baseURL: 'http://127.0.0.1:3103',
                storageState: 'playwright/.auth/stripe-admin.json',
            },
        }] : []),
    ],
    webServer: [
        ...(shouldStart('independent') ? [{
            command:
                'rm -f /private/tmp/featherboard-playwright.db /private/tmp/featherboard-playwright.db-shm /private/tmp/featherboard-playwright.db-wal && DATABASE_URL=sqlite:/private/tmp/featherboard-playwright.db?mode=rwc PORT=3100 PUBLIC_URL=http://127.0.0.1:3100 cargo run --bin featherboard',
            url: 'http://127.0.0.1:3100/healthz',
            reuseExistingServer: false,
            timeout: 120_000,
        }] : []),
        ...(shouldStart('anonymous') ? [{
            command:
                'DATABASE_URL=sqlite:/private/tmp/featherboard-playwright-anonymous.db?mode=rwc PORT=3102 PUBLIC_URL=http://127.0.0.1:3102 cargo run --bin featherboard',
            url: 'http://127.0.0.1:3102/healthz',
            reuseExistingServer: false,
            timeout: 120_000,
        }] : []),
        ...(shouldStart('smtp') ? [{
            command: 'cargo run --features e2e --bin e2e-server',
            url: 'http://127.0.0.1:3101/healthz',
            reuseExistingServer: false,
            timeout: 120_000,
        }] : []),
        ...(stripeE2EEnabled && shouldStart('stripe') ? [{
            command: './scripts/start-stripe-e2e.sh',
            url: 'http://127.0.0.1:3103/healthz',
            reuseExistingServer: false,
            timeout: 120_000,
        }] : []),
    ],
});
