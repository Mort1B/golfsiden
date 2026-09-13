import { expect, type Page } from '@playwright/test'
export function sharingEvents(page: Page) {
  const errors: string[] = []; const statuses: number[] = []; const failed: string[] = []; const privateReads: string[] = []
  page.on('pageerror', () => errors.push('pageerror'))
  page.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push('console error') })
  page.on('response', response => { if (response.status() >= 400) statuses.push(response.status()) })
  page.on('requestfailed', request => failed.push(request.failure()?.errorText ?? 'failed'))
  page.on('request', request => {
    const path = new URL(request.url()).pathname
    if (path.startsWith('/api/') && path !== '/api/auth/session' && !path.startsWith('/api/public/results/')) privateReads.push(path)
  })
  return { errors, statuses, failed, privateReads }
}
export async function sharingLayout(page: Page, name: string, selector: string) {
  for (const [width, height] of [[320, 600], [390, 844], [1280, 900]] as const) {
    await page.setViewportSize({ width, height })
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true)
    for (const control of await page.locator(`${selector} button:visible, ${selector} input:visible`).all()) {
      expect((await control.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await control.isEnabled()) { await control.evaluate(element => element.scrollIntoView({ block: 'center' })); await control.click({ trial: true }) }
    }
    await page.locator(selector).screenshot({ path: `/tmp/golf-sharing-${name}-${width}.png`, mask: [page.getByLabel('Offentlig resultatlenke', { exact: true })] })
  }
}
export function capability(url: string): { id: string; token: string } {
  const parsed = new URL(url)
  const id = parsed.pathname.split('/').at(-1)
  const token = new URLSearchParams(parsed.hash.slice(1)).get('token')
  if (!id || !token) throw new Error('Invalid test capability')
  return { id, token }
}
