import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e/live',
  fullyParallel: false,
  workers: 1,
  forbidOnly: Boolean(process.env.CI),
  failOnFlakyTests: Boolean(process.env.CI),
  retries: 0,
  reporter: [['html', { open: 'never', outputFolder: 'playwright-report/live' }], ['list']],
  outputDir: 'test-results/live',
  use: {
    ...devices['Desktop Chrome'],
    baseURL: 'http://localhost:1430',
    viewport: { width: 1440, height: 900 },
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [{ name: 'live-chromium' }],
  webServer: [
    {
      command: 'node e2e/support/live-server.mjs',
      url: 'http://127.0.0.1:1431/api/v2/auth/config',
      reuseExistingServer: false,
      timeout: 240_000,
    },
    {
      command: 'npm run dev -- --host 127.0.0.1 --port 1430 --strictPort',
      url: 'http://127.0.0.1:1430',
      reuseExistingServer: false,
      timeout: 120_000,
      env: { CAIVEN_E2E_API_TARGET: 'http://127.0.0.1:1431' },
    },
  ],
});
