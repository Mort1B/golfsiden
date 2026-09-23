import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e', testMatch: ['routeSplitting.browser.ts', 'matchCancellation.browser.ts', 'matchResultGate.browser.ts', 'returnLoading.browser.ts', 'returnOrdering.browser.ts', 'navigationAccessibility.browser.ts'],
  workers: 1, timeout: 60_000, outputDir: '/tmp/golf-route-browser-results',
  use: { baseURL: 'http://127.0.0.1:4179', browserName: 'chromium', channel: 'chrome', headless: true, screenshot: 'only-on-failure', actionTimeout: 10_000 },
  webServer: { command: 'npm run preview -- --host 127.0.0.1 --port 4179 --strictPort', url: 'http://127.0.0.1:4179', reuseExistingServer: false },
})
