import { test, expect } from '@playwright/test'
import { routeWorkspace } from './routeSplittingSupport'
import { scoreUrl, scorecard, round, owner } from './returnLoadingSupport'
import { queueRows } from './offlineSupport'
import { decodeObject, decodeString, decodeNumber } from '../src/api/decoder'
import { session } from '../src/features/tournaments/lifecycle/__tests__/fixtures'
for (const width of [320, 390, 1280]) test(`storage outage yields, retains and recovers the exact score operation at ${width}px`, async ({ page }) => {
 await page.setViewportSize({ width, height: width === 320 ? 600 : 900 })
 const f = await routeWorkspace(page)
 const card = scorecard(), requests: unknown[] = []
 const scoreId = '00000000-0000-0000-0000-000000000088'
 try {
  await page.route('**/api/rounds/*/scorecards/player/*/scoring', route => route.fulfill({ json: card }))
  await page.route('**/scores/conditional', async route => {
   const request = decodeObject(route.request().postDataJSON(), 'synthetic score request'); requests.push(request)
   const hole = card.holes.find(h => h.hole_id === request.hole_id)
   if (!hole) throw new Error('missing synthetic hole')
   const gross = decodeNumber(request.gross_strokes, 'gross')
   hole.score = { id: scoreId, round_id: round.id, hole_id: hole.hole_id, owner, gross_strokes: gross, revision: '1',
    submitted_by: session.user_id, submitted_at: '2026-09-23T10:00:00Z', updated_at: '2026-09-23T10:00:00Z' }
   hole.net_strokes = gross; card.gross_total = gross; card.net_total = gross; card.holes_scored = 1
   await route.fulfill({ json: { request_id: decodeString(request.request_id, 'request'), applied_score: { score_id: scoreId, revision: '1' } } })
  })
  await page.goto(scoreUrl)
  await expect(page.getByRole('button', { name: /Registrer par/ })).toBeEnabled()
  await page.evaluate(() => { Object.defineProperty(navigator, 'onLine', { configurable: true, get: () => false }); window.dispatchEvent(new Event('offline')) })
  await page.getByRole('button', { name: /Registrer par/ }).click()
  await expect.poll(async () => (await queueRows(page)).length).toBe(1)
  const before = decodeObject((await queueRows(page))[0], 'before'), head = before.head
  const observed = await page.evaluate(async () => {
   const db = indexedDB, open = db.open.bind(db)
   let failures = 0
   db.open = function(name, version) {
    if (name === 'golf-pending-scores-v1' && ++failures <= 50) throw new Error('Synthetic bounded storage outage')
    return version === undefined ? open(name) : open(name, version)
   }
   document.addEventListener('restore-score-test-storage', () => { db.open = open }, { once: true })
   const timer = new Promise<number>(resolve => setTimeout(() => resolve(failures), 0))
   Object.defineProperty(navigator, 'onLine', { configurable: true, get: () => true })
   window.dispatchEvent(new Event('online'))
   return timer
  })
  // The broken runtime consumes all 50 fault attempts before this task can run.
  expect(observed).toBeGreaterThan(0); expect(observed).toBeLessThanOrEqual(2)
  const panel = page.locator('#pending-scores')
  await panel.locator('summary').click()
  await expect(panel).toContainText('lagringsfeil')
  await expect(panel.getByRole('button', { name: 'Prøv lokal lagring igjen', exact: true })).toBeVisible()
  expect(requests).toEqual([])
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  const retry = panel.getByRole('button', { name: 'Prøv lokal lagring igjen', exact: true })
  expect((await retry.boundingBox())?.height).toBeGreaterThanOrEqual(44)
  await retry.click({ trial: true })
  await page.screenshot({ path: `/tmp/golf-score-retry/storage-error-${width}.png`, fullPage: true })
  await page.evaluate(() => document.dispatchEvent(new Event('restore-score-test-storage')))
  const retained = decodeObject((await queueRows(page))[0], 'retained')
  expect(retained.head).toEqual(head); expect(retained.generation).toBe(before.generation)
  await retry.click()
  await expect.poll(() => requests.length).toBe(1)
  expect(requests[0]).toEqual(head)
  await expect.poll(async () => (await queueRows(page)).length).toBe(0)
  await expect(panel).toContainText('Ingen ventende endringer på denne enheten.')
  await expect(panel).not.toContainText('lagringsfeil')
  await page.screenshot({ path: `/tmp/golf-score-retry/recovered-${width}.png`, fullPage: true })
  console.log(`width=${width} storage attempts before zero-delay timer=${observed}; exact operation delivered once`)
 } finally { await f.close() }
})
