import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  testMatch: 'iphone*.browser.ts',
  workers: 1,
  timeout: 90_000,
  outputDir: '/tmp/golf-iphone-results',
  use: {
    baseURL: 'http://127.0.0.1:5173',
    headless: true,
    screenshot: 'only-on-failure',
    actionTimeout: 10_000,
    hasTouch: true,
  },
  projects: [
    { name: 'chrome', use: { browserName: 'chromium', channel: 'chrome' } },
    { name: 'webkit', use: { browserName: 'webkit' } },
  ],
})
