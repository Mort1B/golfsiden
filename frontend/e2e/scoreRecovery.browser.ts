import { decodeReadScorecard } from '../src/api/scorecards'
import { expect, test, type Page } from '@playwright/test'
import { decodeObject } from '../src/api/decoder'
import { offlineEvents, offlineFixture, offlineLayout, queueRows } from './offlineSupport'
import { fourBallFixture } from './fourBallSupport'
import { stablefordFixture } from './stablefordSupport'

test.skip(process.env.GOLF_H1_BROWSER !== '1', 'Requires disposable API')
async function storage(page: Page, fail: boolean) {
  await page.evaluate(failed => {
    if (failed) {
      const original = IDBObjectStore.prototype.put
      IDBObjectStore.prototype.put = function (value: unknown, key?: IDBValidKey) {
        if (this.name === 'pending') throw new DOMException('Device full', 'QuotaExceededError')
        return key === undefined ? original.call(this, value) : original.call(this, value, key)
      }
      document.addEventListener('restore-test-storage', () => { IDBObjectStore.prototype.put = original }, { once: true })
    } else document.dispatchEvent(new Event('restore-test-storage'))
  }, fail)
}
async function refresh(page: Page) { await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange'))) }
for (const format of ['individual_stroke_play', 'team_scramble', 'two_player_foursomes'] as const) {
  test(`${format}: failed correction survives external lock and offline recovery never delivers automatically`, async ({ page, context }) => {
    const f = await offlineFixture(page, format), events = offlineEvents(page)
    for (let h = 1; h <= 18; h++) await f.save(h, 4)
    await f.mutate(`/api/rounds/${f.round.id}/scorecards/${f.owner.type}/${f.owner.id}/confirm`)
    await f.mutate(`/api/rounds/${f.round.id}/complete`)
    await page.goto(f.url()); await page.getByRole('button', { name: 'Korriger score', exact: true }).click()
    await storage(page, true); await page.getByRole('button', { name: 'Legg til ett slag' }).click()
    await expect(page.getByText('Ikke lagret på enheten', { exact: true })).toBeVisible()
    await f.mutate(`/api/rounds/${f.round.id}/lock`)
    const recovery = page.getByRole('region', { name: 'Ulagrede scoreendringer' })
    await expect(recovery).toContainText('5 slag'); await expect(page.getByRole('button', { name: 'Logg ut' })).toBeDisabled()
    await expect(page.locator('.scorecard-owner')).toHaveCount(0)
    if (format === 'individual_stroke_play') {
      await offlineLayout(page, 'h1-recovery')
      await page.setViewportSize({ width: 320, height: 600 })
      await recovery.getByRole('button', { name: 'Forkast ulagret endring' }).scrollIntoViewIfNeeded()
      await recovery.getByRole('button', { name: 'Lagre lokal kopi' }).click({ trial: true })
      await page.screenshot({ path: '/tmp/golf-h1-recovery-controls-320.png' })
    }
    await page.getByRole('link', { name: 'Resultater', exact: true }).click(); await expect(recovery).toBeVisible()
    await context.setOffline(true); await storage(page, false)
    await recovery.getByRole('button', { name: 'Lagre lokal kopi' }).click()
    await expect(recovery).toHaveCount(0)
    const rows = await queueRows(page); expect(rows).toHaveLength(1)
    expect(decodeObject(rows[0], 'pending').phase).toBe('conflict')
    await context.setOffline(false); await refresh(page)
    await page.locator('.pending-scores summary').click()
    await expect(page.getByRole('button', { name: 'Sammenlign scorer' })).toBeVisible()
    expect(decodeObject((await queueRows(page))[0], 'pending').phase).toBe('conflict')
    const canonical = decodeReadScorecard(await (await page.request.get(`/api/rounds/${f.round.id}/scorecards/${f.owner.type}/${f.owner.id}`)).json(), f.round.id, f.owner)
    expect(canonical.holes[0]?.score?.gross_strokes).toBe(4)
    expect(events.errors).toEqual([])
    expect(events.statuses.filter(status => status !== 403)).toEqual([])
    expect(events.failures.filter(error => !['net::ERR_ABORTED', 'net::ERR_INTERNET_DISCONNECTED', 'net::ERR_FAILED'].includes(error))).toEqual([])
  })
}
test('four-ball retains both partner intents on scoring403; discard preserves stable partner identity', async ({ page, browser }) => {
  const f = await fourBallFixture(page, browser), events = offlineEvents(page)
  await page.setViewportSize({ width: 390, height: 844 }); await page.goto(f.url())
  const partners = page.locator('.four-ball-partner')
  await expect(partners).toHaveCount(2); await storage(page, true)
  await partners.nth(1).getByRole('button', { name: 'Plukket opp', exact: true }).click()
  await partners.nth(1).getByRole('button', { name: 'Ja, plukket opp' }).click()
  await partners.nth(0).getByRole('button', { name: /Ett slag mer/ }).click()
  await expect(page.getByRole('button', { name: 'Logg ut' })).toBeDisabled()
  await page.route('**/scoring', route => route.fulfill({ status: 403, json: { error: { code: 'forbidden', message: 'Denied' } } }))
  await refresh(page)
  const recovery = page.getByRole('region', { name: 'Ulagrede scoreendringer' }); await expect(recovery).toBeVisible()
  await expect(recovery.getByRole('heading', { name: 'Hull 1 · spiller 2' })).toBeVisible()
  await expect(recovery).toContainText('Plukket opp'); await expect(recovery).toContainText('5 slag')
  await expect(page.locator('.four-ball-partner')).toHaveCount(0)
  await recovery.getByRole('region', { name: 'Lokal endring 2' }).getByRole('button', { name: 'Forkast ulagret endring' }).click()
  await expect(recovery.getByRole('heading', { name: 'Hull 1 · spiller 2' })).toBeVisible()
  await recovery.getByRole('button', { name: 'Forkast ulagret endring' }).click()
  await expect(page.getByRole('button', { name: 'Logg ut' })).toBeEnabled()
  expect(events.errors).toEqual([]); expect(events.statuses.filter(status => status !== 403)).toEqual([])
})
for (const pickup of [false, true]) test(`Stableford ${pickup ? 'pickup' : 'numeric'} survives metadata failure with local-only recovery`, async ({ page }) => {
  const f = await stablefordFixture(page), events = offlineEvents(page)
  await page.setViewportSize({ width: 320, height: 600 }); await page.goto(f.url()); await expect(page.locator('.four-ball-partner')).toBeVisible()
  await storage(page, true)
  if (pickup) { await page.getByRole('button', { name: 'Plukket opp', exact: true }).click(); await page.getByRole('button', { name: 'Ja, plukket opp' }).click() }
  else await page.getByRole('button', { name: /Ett slag mer/ }).click()
  await expect(page.getByRole('button', { name: 'Logg ut' })).toBeDisabled()
  await page.route('**/completion-validation', route => route.fulfill({ status: 500, json: { error: { code: 'unavailable', message: 'Metadata unavailable' } } }))
  await refresh(page)
  const recovery = page.getByRole('region', { name: 'Ulagrede scoreendringer' }); await expect(recovery).toContainText(pickup ? 'Plukket opp' : '5 slag')
  await expect(page.locator('.stableford-result')).toHaveCount(0)
  await recovery.getByRole('button', { name: 'Forkast ulagret endring' }).click()
  await expect(page.getByRole('button', { name: 'Logg ut' })).toBeEnabled()
  expect(events.errors).toEqual([]); expect(events.statuses.filter(status => status !== 500)).toEqual([])
})
