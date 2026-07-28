import { defineConfig, devices } from '@playwright/test';

const common = {
  baseURL: 'http://127.0.0.1:1430',
  trace: 'retain-on-failure' as const,
  screenshot: 'only-on-failure' as const,
  video: 'retain-on-failure' as const,
};

export default defineConfig({
  testDir: './e2e/mock',
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  failOnFlakyTests: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: [['html', { open: 'never' }], ['list']],
  outputDir: 'test-results/mock',
  use: common,
  projects: [
    {
      name: 'desktop-chromium',
      use: { ...devices['Desktop Chrome'], ...common, viewport: { width: 1440, height: 900 } },
    },
    {
      name: 'mobile-chromium',
      use: { ...devices['Pixel 7'], ...common },
    },
  ],
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 1430 --strictPort',
    url: 'http://127.0.0.1:1430',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
