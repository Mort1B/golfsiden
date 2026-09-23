import { expect, test, type Page } from '@playwright/test'
import { routeWorkspace, scoreUrl, trip } from './routeSplittingSupport'

async function profileCards(page: Page) {
  await page.route('**/api/me/tournaments', route => route.fulfill({ json: [
    'Golfturen med et langt turneringsnavn for alle vennene', 'Lagturneringen ved fjorden', 'Høstturneringen',
  ].map((name, index) => ({ tournament: { ...trip, id: `00000000-0000-0000-0000-00000000900${index}`, name }, role: 'admin', player_id: null })) }))
}

async function focusedClearance(page: Page) {
  return page.evaluate(() => {
    const active = document.activeElement
    if (!(active instanceof HTMLElement) || !active.closest('main')) return null
    const box = active.getBoundingClientRect()
    const nav = document.querySelector('.bottom-nav')?.getBoundingClientRect()
    const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2)
    return { name: active.textContent?.slice(0, 70), top: box.top, bottom: box.bottom,
      card: active.matches('.list-card') ? active.getAttribute('href') : null,
      limit: nav && nav.width > innerWidth / 2 ? nav.top : innerHeight,
      uncovered: hit === active || active.contains(hit), outline: getComputedStyle(active).outlineStyle }
  })
}

async function traverseCards(page: Page, key: 'Tab' | 'Shift+Tab', expected: string[]) {
  const reached = new Set<string>()
  for (let index = 0; index < 20; index++) {
    const box = await focusedClearance(page)
    if (box) {
      expect(box.bottom, box.name).toBeLessThanOrEqual(box.limit - 4)
      expect(box.top, box.name).toBeGreaterThanOrEqual(0)
      expect(box.uncovered, box.name).toBe(true)
      expect(box.outline).not.toBe('none')
      if (box.card) reached.add(box.card)
    }
    if (reached.size === expected.length) break
    await page.keyboard.press(key)
  }
  expect([...reached].sort()).toEqual([...expected].sort())
}

for (const width of [320, 390, 699, 700, 1280]) test(`keyboard traversal keeps profile cards clear at ${width}px`, async ({ page }) => {
  const f = await routeWorkspace(page)
  try {
    await profileCards(page)
    await page.setViewportSize({ width, height: 600 })
    await page.goto('/profile')
    await expect(page.locator('.profile-page .list-card')).toHaveCount(3)
    const expected = await page.locator('.profile-page .list-card').evaluateAll(cards => cards.map(card => card.getAttribute('href') ?? ''))
    await traverseCards(page, 'Tab', expected)
    await traverseCards(page, 'Shift+Tab', expected)
    await page.keyboard.press('Tab')
    await expect(page.locator('.profile-page .list-card').nth(1)).toBeFocused()
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    await page.screenshot({ path: `/tmp/golf-focus-profile-${width}.png` })
  } finally { await f.close() }
})

test('mobile safe-area clearance follows navigation and does not leak into public pages', async ({ page, context }) => {
  const f = await routeWorkspace(page)
  try {
    await profileCards(page)
    await page.setViewportSize({ width: 320, height: 600 })
    const devtools = await context.newCDPSession(page)
    await devtools.send('Emulation.setSafeAreaInsetsOverride', { insets: { bottom: 24 } })
    await page.goto('/profile')
    await expect(page.locator('.profile-page .list-card')).toHaveCount(3)
    const expected = await page.locator('.profile-page .list-card').evaluateAll(cards => cards.map(card => card.getAttribute('href') ?? ''))
    expect(await page.evaluate(() => getComputedStyle(document.documentElement).scrollPaddingBottom)).toBe('100px')
    await traverseCards(page, 'Tab', expected)
    await traverseCards(page, 'Shift+Tab', expected)
    await page.screenshot({ path: '/tmp/golf-focus-safe-area.png' })
    await page.setViewportSize({ width: 700, height: 600 })
    expect(await page.evaluate(() => getComputedStyle(document.documentElement).scrollPaddingBottom)).toBe('8px')
    await page.getByRole('button', { name: 'Logg ut', exact: true }).click()
    await expect(page.getByRole('button', { name: 'Logg inn', exact: true })).toBeVisible()
    expect(await page.evaluate(() => getComputedStyle(document.documentElement).scrollPaddingBottom)).toBe('auto')
  } finally { await f.close() }
})

for (const width of [320, 1280]) test(`score controls stay visible during keyboard traversal at ${width}px`, async ({ page }) => {
  const f = await routeWorkspace(page)
  try {
    await page.setViewportSize({ width, height: 600 })
    await page.goto(scoreUrl)
    await expect(page.locator('#current-hole-heading')).toHaveText('8')
    const reached: string[] = []
    for (let index = 0; index < 14; index++) {
      await page.keyboard.press('Tab')
      const box = await focusedClearance(page)
      if (box) {
        expect(box.bottom, box.name).toBeLessThanOrEqual(box.limit - 4)
        expect(box.top, box.name).toBeGreaterThanOrEqual(0)
        expect(box.uncovered, box.name).toBe(true)
      }
      const name = await page.evaluate(() => document.activeElement?.getAttribute('aria-label'))
      if (name) reached.push(name)
    }
    expect(reached).toContain('Legg til ett slag')
    expect(reached).toContain('Trekk fra ett slag')
    expect(f.score.saves).toBe(0)
    await page.screenshot({ path: `/tmp/golf-focus-score-${width}.png` })
  } finally { await f.close() }
})

test('shared back controls meet the 44px target at mobile and desktop widths', async ({ page }) => {
  const f = await routeWorkspace(page)
  f.state.emptyRounds = true
  try {
    await page.goto(`/manage/tournaments/${trip.id}`)
    for (const width of [320, 390, 699, 700, 1280]) {
      await page.setViewportSize({ width, height: 600 })
      const back = page.getByRole('link', { name: 'Tilbake til turneringen', exact: true })
      await expect(back).toBeVisible()
      const box = await back.boundingBox()
      expect(box?.width).toBeGreaterThanOrEqual(44)
      expect(box?.height).toBeGreaterThanOrEqual(44)
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    }
  } finally { await f.close() }
})

test('small tournament section counts have sufficient contrast on their background', async ({ page }) => {
  const f = await routeWorkspace(page)
  try {
    await page.goto(`/tournaments/${trip.id}`)
    const count = page.locator('.section-heading span').first()
    await expect(count).toBeVisible()
    const contrast = await count.evaluate(element => {
      const luminance = (color: string) => {
        const channels = color.match(/[\d.]+/g)?.slice(0, 3).map(Number)
        if (!channels || channels.length !== 3) throw new Error('Expected RGB color')
        return channels.reduce((sum, value, index) => {
          const channel = value / 255
          const weight = [0.2126, 0.7152, 0.0722][index] ?? 0
          return sum + weight * (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4)
        }, 0)
      }
      let ancestor: Element | null = element
      while (ancestor && getComputedStyle(ancestor).backgroundColor === 'rgba(0, 0, 0, 0)') ancestor = ancestor.parentElement
      if (!ancestor) throw new Error('Missing background')
      const text = luminance(getComputedStyle(element).color), background = luminance(getComputedStyle(ancestor).backgroundColor)
      return (Math.max(text, background) + 0.05) / (Math.min(text, background) + 0.05)
    })
    expect(contrast).toBeGreaterThanOrEqual(4.5)
  } finally { await f.close() }
})
