import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  testMatch: '**/*.browser.ts',
  workers: 1,
  timeout: 240_000,
  outputDir: '/tmp/golf-lifecycle-browser-results',
  use: {
    baseURL: 'http://127.0.0.1:5173',
    browserName: 'chromium',
    channel: 'chrome',
    headless: true,
    screenshot: 'only-on-failure',
    trace: 'off',
    actionTimeout: 15_000,
  },
})
