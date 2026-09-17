import { expect, test } from '@playwright/test'
import { offlineEvents, offlineFixture, offlineLayout } from './offlineSupport'

test.skip(process.env.GOLF_M2_BROWSER !== '1', 'Requires disposable API')
for (const [view, status] of [['card', 401], ['history', 403], ['round', 404]] as const) {
  test(`${view}: cached private data disappears after ${status} while SSE stays healthy`, async ({ page }) => {
    const f = await offlineFixture(page), events = offlineEvents(page)
    await f.save(1, 7)
    const name = 'Spiller med et svært langt navn som fører score uten forbindelse'
    const target = view === 'card' ? `/api/rounds/${f.round.id}/scorecards/player/${f.owner.id}`
      : view === 'history' ? `/api/tournaments/${f.tournament.id}/leaderboards/gross` : `/api/rounds/${f.round.id}/leaderboards/gross`
    const path = view === 'card' ? `/tournaments/${f.tournament.id}/rounds/${f.round.id}/scorecards/player/${f.owner.id}?metric=gross&view=summary`
      : view === 'history' ? `/tournaments/${f.tournament.id}/results/players/${f.owner.id}?metric=gross`
      : `/leaderboard?tournament=${f.tournament.id}&scope=round&round=${f.round.id}&metric=gross`
    let liveRequests = 0
    page.on('request', request => { if (new URL(request.url()).pathname.endsWith('/live')) liveRequests++ })
    await page.addInitScript(() => {
      const Native = EventSource
      window.EventSource = class extends Native {
        constructor(url: string | URL, options?: EventSourceInit) {
          super(url, options)
          this.addEventListener('open', () => { document.documentElement.dataset.testLiveOpen = String(Number(document.documentElement.dataset.testLiveOpen ?? '0') + 1) })
          this.addEventListener('error', () => { document.documentElement.dataset.testLiveError = String(Number(document.documentElement.dataset.testLiveError ?? '0') + 1) })
        }
      }
    })
    await page.setViewportSize({ width: 390, height: 844 }); await page.goto(path)
    const resultName = page.locator('.main-content').getByText(name, { exact: true })
    await expect(resultName).toBeVisible()
    await expect.poll(() => page.evaluate(() => Number(document.documentElement.dataset.testLiveOpen ?? '0')), { timeout: 30_000 }).toBeGreaterThan(0)
    const initialLive = liveRequests; expect(initialLive).toBeGreaterThan(0)
    let responseStatus = 500
    await page.route(url => url.pathname === target, route => responseStatus === 200 ? route.continue() : route.fulfill({ status: responseStatus, json: { error: { code: 'private_result_test', message: responseStatus === 500 ? 'Temporary failure' : 'Access denied' } } }))
    const refresh = async () => {
      const response = page.waitForResponse(r => new URL(r.url()).pathname === target)
      await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')))
      await response
    }
    await refresh(); await expect(resultName).toBeVisible()
    responseStatus = status; await refresh()
    await expect(resultName).toHaveCount(0)
    await expect(page.locator('.main-content a[href*="scorecards"], .main-content a[href*="results/players"]')).toHaveCount(0)
    expect(liveRequests).toBe(initialLive)
    expect(await page.evaluate(() => Number(document.documentElement.dataset.testLiveError ?? '0'))).toBe(0)
    if (view === 'card') await offlineLayout(page, 'm2-denied')
    responseStatus = 500; await refresh(); await expect(resultName).toHaveCount(0)
    const retry = page.getByRole('button', { name: 'Prøv igjen', exact: true }).first()
    await expect(retry).toBeVisible(); await retry.click(); await expect(resultName).toHaveCount(0)
    responseStatus = 200; await refresh(); await expect(resultName).toBeVisible()
    if (view === 'card') await offlineLayout(page, 'm2-recovered')
    expect(events.errors).toEqual([])
    expect(events.statuses.filter(value => ![status, 500].includes(value))).toEqual([])
    expect(events.failures.filter(error => !['net::ERR_ABORTED', 'net::ERR_INTERNET_DISCONNECTED', 'net::ERR_FAILED'].includes(error))).toEqual([])
  })
}
