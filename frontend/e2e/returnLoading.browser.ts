import { test, expect, type Page } from '@playwright/test'
import { liveServer, mockWorkspace, scoreUrl } from './returnLoadingSupport'

async function layout(page: Page, name: string) {
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const control of await page.locator('.score-page button:visible, .score-page select:visible').all()) {
      expect((await control.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await control.isEnabled()) await control.click({ trial: true })
    }
    await page.evaluate(() => window.scrollTo(0, 0))
    await page.screenshot({ path: `/tmp/golf-return-${name}-${width}.png`, fullPage: true })
  }
}
async function restored(page: Page) {
  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
}
async function stopped(live: Awaited<ReturnType<typeof liveServer>>, page: Page) {
  const before = live.connections
  live.stop()
  await expect(page.getByText('Forbindelsen er brutt. Prøver å koble til igjen.')).toBeVisible()
  await expect.poll(() => live.connections).toBeGreaterThan(before)
}

test.beforeEach(async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', error => errors.push(error.message))
  page.on('console', msg => {
    if (msg.type() === 'error' && !msg.text().startsWith('Failed to load resource:')) errors.push(msg.text())
  })
  page.on('response', response => {
    if (response.status() >= 400 && ![401, 403, 503].includes(response.status())) errors.push(`Unexpected HTTP ${response.status()}`)
  })
  page.on('requestfailed', request => {
    const failure = request.failure()?.errorText ?? ''
    if (!new URL(request.url()).pathname.endsWith('/live') && !['net::ERR_ABORTED', 'net::ERR_INTERNET_DISCONNECTED'].includes(failure)) errors.push(failure)
  })
  await page.exposeFunction('assertNoBrowserErrors', () => expect(errors).toEqual([]))
})
test.afterEach(async ({ page }) => {
  if (!page.isClosed()) await page.evaluate(async () => {
    const check = Reflect.get(window, 'assertNoBrowserErrors') as () => Promise<void>
    await check()
  })
})

for (const event of ['pageshow', 'visibilitychange', 'online'] as const) {
  test(`return via ${event} restarts a stopped native stream without navigation`, async ({ page }) => {
    const live = await liveServer()
    try {
      const state = await mockWorkspace(page, live.url)
      await page.goto(scoreUrl)
      await expect(page.locator('#current-hole-heading')).toHaveText('8')
      await stopped(live, page)
      await expect(page.getByRole('button', { name: 'Legg til ett slag' })).toBeEnabled()
      await layout(page, `disconnected-${event}`)
      live.resume()
      const reads = state.reads
      if (event === 'pageshow') await restored(page)
      else await page.evaluate(type => (type === 'visibilitychange' ? document : window).dispatchEvent(new Event(type)), event)
      await expect(page.getByRole('button', { name: 'Legg til ett slag' })).toBeEnabled()
      await expect(page.locator('#current-hole-heading')).toHaveText('8')
      await expect.poll(() => state.reads).toBeGreaterThan(reads)
      await layout(page, `recovered-${event}`)
      const connections = live.connections
      for (let i = 0; i < 3; i += 1) await restored(page)
      await expect(page.getByRole('button', { name: 'Legg til ett slag' })).toBeEnabled()
      expect(live.connections).toBe(connections)
    } finally { await live.close() }
  })
}

test('durable score permits navigation through delayed, failed and successful recovery', async ({ page }) => {
  const live = await liveServer()
  try {
    const state = await mockWorkspace(page, live.url)
    await page.goto(scoreUrl)
    await page.getByRole('button', { name: /Registrer par/ }).click()
    await expect(page.locator('.score-sync')).toContainText('Lagret på denne enheten')
    await stopped(live, page)
    await expect(page.getByRole('button', { name: 'Neste' })).toBeEnabled()
    let release: () => void = () => undefined
    state.pending = new Promise<void>(resolve => { release = resolve })
    live.resume(); await restored(page)
    await expect(page.getByText(/Oppdaterer scoretilgang og rundestatus/)).toBeVisible()
    await layout(page, 'refreshing-durable-score')
    state.fail = true; state.pending = null; release()
    await expect(page.getByText('Noe kunne ikke oppdateres. Viste data beholdes.')).toBeVisible()
    await page.getByRole('button', { name: 'Neste' }).click()
    await expect(page.locator('#current-hole-heading')).toHaveText('9')
    await layout(page, 'refresh-error')
    state.fail = false
    await page.getByRole('button', { name: 'Prøv oppdatering' }).click()
    await expect(page.getByRole('heading', { name: 'Spiller med et langt navn' })).toBeVisible()
    await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Turnering', exact: true }).click()
    await expect(page).toHaveURL(/\/tournaments/)
    await page.getByRole('link', { name: 'Lokale scoreendringer (1)' }).click()
    await page.locator('.pending-scores summary').click()
    await expect(page.getByText('Lokalt: 4 slag')).toBeVisible()
    await page.getByRole('button', { name: 'Forkast lokal endring' }).click()
    await page.getByRole('button', { name: 'Ja, fjern lokal kopi' }).click()
    await expect(page.locator('.pending-scores summary')).toContainText('(0)')
    state.empty = true; await restored(page)
    await expect(page.getByText('Runden har ingen kvalifiserte scorekort')).toBeVisible()
    await layout(page, 'empty')
  } finally { await live.close() }
})

test('expired session returns to sign-in without navigation', async ({ page }) => {
  const live = await liveServer()
  try {
    const state = await mockWorkspace(page, live.url)
    await page.goto(scoreUrl)
    await expect(page.locator('#current-hole-heading')).toHaveText('8')
    state.expired = true
    await restored(page)
    await expect(page).toHaveURL(/\/login/)
    await expect(page.getByLabel('Brukernavn', { exact: true })).toBeVisible()
  } finally { await live.close() }
})

test('read cards fail closed and recover to the restricted prefix; revoked access stays hidden', async ({ page }) => {
  const live = await liveServer()
  try {
    const state = await mockWorkspace(page, live.url)
    state.readOnly = true
    await page.goto(scoreUrl.replace('hole=8', 'hole=18'))
    await expect(page.locator('#current-hole-heading')).toHaveText('18')
    await stopped(live, page)
    await expect(page.locator('#current-hole-heading')).toHaveCount(0)
    state.restricted = true
    live.resume(); await restored(page)
    await expect(page.locator('#current-hole-heading')).toHaveText('9')
    await expect(page.getByText('Hull 10–18 er skjult til administratoren frigir finalens bakni.')).toBeVisible()
    await layout(page, 'restricted')
    await stopped(live, page)
    state.denied = true
    live.resume(); await restored(page)
    await expect(page.getByText('Du har ikke tilgang')).toBeVisible()
    await expect(page.locator('#current-hole-heading')).toHaveCount(0)
  } finally { await live.close() }
})

test('internal return, Back/Forward, reload, and empty-card resume preserve their selections', async ({ page }) => {
  const live = await liveServer()
  try {
    await mockWorkspace(page, live.url)
    await page.goto(scoreUrl)
    await expect(page.locator('#current-hole-heading')).toHaveText('8')
    await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Turnering', exact: true }).click()
    await expect(page.getByRole('heading', { name: 'Dine turneringer' })).toBeVisible()
    await page.goBack()
    await expect(page.locator('#current-hole-heading')).toHaveText('8')
    await page.goForward()
    await expect(page.getByRole('heading', { name: 'Dine turneringer' })).toBeVisible()
    await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Score', exact: true }).click()
    await expect(page.locator('#current-hole-heading')).toHaveText('1')
    await page.reload()
    await expect(page.locator('#current-hole-heading')).toHaveText('1')
  } finally { await live.close() }
})

test('offline return and frozen-page resume recover; a round locked while away becomes read-only', async ({ page, context }) => {
  const live = await liveServer()
  try {
    const state = await mockWorkspace(page, live.url)
    await page.goto(scoreUrl)
    await expect(page.locator('#current-hole-heading')).toHaveText('8')
    await stopped(live, page)
    await context.setOffline(true)
    await layout(page, 'offline')
    live.resume()
    await context.setOffline(false)
    await page.evaluate(() => window.dispatchEvent(new Event('online')))
    await expect(page.getByRole('button', { name: 'Legg til ett slag' })).toBeEnabled()
    const cdp = await context.newCDPSession(page)
    await cdp.send('Page.setWebLifecycleState', { state: 'frozen' })
    state.readOnly = true
    await cdp.send('Page.setWebLifecycleState', { state: 'active' })
    await restored(page)
    await expect(page.getByText('Du kan se dette scorekortet, men ikke føre score for det.')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Legg til ett slag' })).toHaveCount(0)
    await expect(page.locator('#current-hole-heading')).toHaveText('8')
    await cdp.detach()
  } finally { await live.close() }
})
