import { expect, test } from '@playwright/test'
import { routeWorkspace, scoreUrl, trip } from './routeSplittingSupport'

import { iphoneViewports } from './iphoneSupport'

for (const [width, height] of iphoneViewports) test(`shell and score fit ${width}x${height}`, async ({ page }, info) => {
  const f = await routeWorkspace(page)
  try {
    await page.setViewportSize({ width, height })
    await page.goto(scoreUrl)
    await expect(page.locator('#current-hole-heading')).toHaveText('8')
    await page.screenshot({ path: `/tmp/iphone-${info.project.name}-score-${width}-${height}.png` })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const link of await page.locator('.nav-link').all()) {
      const box = await link.boundingBox()
      expect(box?.width).toBeGreaterThanOrEqual(44)
      expect(box?.height).toBeGreaterThanOrEqual(44)
      expect(box?.y).toBeGreaterThanOrEqual(0)
      expect((box?.y ?? 0) + (box?.height ?? 0)).toBeLessThanOrEqual(height)
    }
    await page.goto('/profile')
    await expect(page.locator('.profile-page')).toBeVisible()
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    await page.goto(`/tournaments/${trip.id}`)
    await expect(page.locator('.detail-header')).toBeVisible()
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    expect(f.score.saves).toBe(0)
  } finally { await f.close() }
})

test('long tournament titles wrap instead of expanding cards', async ({ page }, info) => {
  const f = await routeWorkspace(page)
  try {
    await page.route('**/api/tournaments', route => route.fulfill({ json: [{ ...trip, name: 'SommerturneringenForAlleGolfvenneneFraTrondheimOgOmegn2026' }] }))
    await page.setViewportSize({ width: 393, height: 852 })
    await page.goto('/tournaments')
    await expect(page.locator('.list-card')).toBeVisible()
    await page.screenshot({ path: `/tmp/iphone-${info.project.name}-long-title.png` })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(393)
  } finally { await f.close() }
})

test('cutout and home-indicator space protects shell and public forms', async ({ page, context, browserName }, info) => {
  test.skip(browserName !== 'chromium', 'Safe-area injection uses Chrome CDP; real iPhone checks remain separate')
  const f = await routeWorkspace(page)
  try {
    await page.setViewportSize({ width: 852, height: 393 })
    const cdp = await context.newCDPSession(page)
    await cdp.send('Emulation.setSafeAreaInsetsOverride', { insets: { top: 0, left: 59, right: 59, bottom: 21 } })
    await page.goto('/profile')
    await expect(page.locator('.profile-page')).toBeVisible()
    await page.screenshot({ path: `/tmp/iphone-${info.project.name}-safe-landscape.png` })
    const account = await page.locator('.session-bar').boundingBox()
    expect((account?.x ?? 0) + (account?.width ?? 0)).toBeLessThanOrEqual(852 - 59)
    const nav = await page.locator('.nav-link').first().boundingBox()
    expect(nav?.x).toBeGreaterThanOrEqual(59)
    await page.setViewportSize({ width: 393, height: 852 })
    await cdp.send('Emulation.setSafeAreaInsetsOverride', { insets: { top: 59, left: 0, right: 0, bottom: 34 } })
    await page.goto('/profile')
    await expect(page.locator('.profile-page')).toBeVisible()
    expect((await page.locator('.session-bar').boundingBox())?.y).toBeGreaterThanOrEqual(59)
    for (const link of await page.locator('.nav-link').all()) expect((await link.boundingBox())?.height).toBeGreaterThanOrEqual(44)
    await page.getByRole('button', { name: 'Logg ut', exact: true }).click()
    await expect(page.getByRole('button', { name: 'Logg inn', exact: true })).toBeVisible()
    expect(await page.locator('input').first().evaluate(el => parseFloat(getComputedStyle(el).fontSize))).toBeGreaterThanOrEqual(16)
  } finally { await f.close() }
})
