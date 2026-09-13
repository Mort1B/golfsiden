import { test, expect } from '@playwright/test'
import { publicFixture, shareId, shareSecret } from '../src/api/resultSharing/__tests__/fixtures'
import type { PublicResults } from '../src/api/resultSharing/contracts'
import { sharingEvents, sharingLayout } from './resultSharingSupport'

test.skip(process.env.GOLF_RESULT_SHARING_BROWSER !== '1', 'Requires GOLF_RESULT_SHARING_BROWSER=1.')
test.use({ screenshot: 'off', trace: 'off' })
test('public loading/error/empty/long rows, visible polling, return/offline and same-grant hash ownership', async ({ page }) => {
  const events = sharingEvents(page)
  let mode: 'ready' | 'loading' | 'error' | 'empty' | 'hidden' = 'ready'
  let release: (() => void) | undefined
  let deferred = Promise.resolve()
  let reads = 0
  let expires = '2099-01-01T00:00:00Z'
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname
    if (!path.startsWith('/api/')) return route.continue()
    if (path === '/api/auth/session') return route.fulfill({ status: 401, json: { error: { code: 'unauthorized', message: 'unauthorized' } } })
    if (path !== `/api/public/results/${shareId}`) throw new Error('Unexpected private request')
    reads++
    const input: unknown = route.request().postDataJSON()
    if (typeof input !== 'object' || input === null || !('token' in input) || input.token !== shareSecret) return route.fulfill({ status: 404, json: { error: { code: 'result_share_unavailable', message: 'unavailable' } } })
    if (mode === 'loading') await deferred
    if (mode === 'error') return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'unavailable' } } })
    const metric = 'metric' in input && input.metric === 'net' ? 'net' : 'gross'
    const board: PublicResults = publicFixture(metric); board.expires_at = expires
    if (mode === 'empty' || mode === 'hidden') board.entries = []
    if (mode === 'hidden') board.visibility = { mode: 'front_nine' }
    await route.fulfill({ json: board })
  })
  const visit = async () => { try { await page.goto(`/results/shared/${shareId}#token=${shareSecret}`) } catch { throw new Error('Public test navigation failed') } }
  await page.clock.install()
  await visit()
  await expect(page.getByRole('heading', { name: publicFixture().tournament_name })).toBeVisible()
  await sharingLayout(page, 'public-longnames', '.public-results-page')
  await page.getByRole('button', { name: 'Netto', exact: true }).click()
  await expect(page.getByRole('heading', { name: 'Netto sammenlagt' })).toBeVisible()
  await sharingLayout(page, 'public-net', '.public-results-page')
  const previousReads = reads
  await page.clock.fastForward(15_000)
  await expect.poll(() => reads).toBeGreaterThan(previousReads)
  mode = 'loading'; deferred = new Promise(resolve => { release = resolve })
  await page.getByRole('button', { name: 'Oppdater resultater' }).click()
  await expect(page.locator('.public-result-row')).toHaveCount(0)
  await sharingLayout(page, 'public-loading', '.public-results-page')
  mode = 'error'; release?.()
  await expect(page.getByRole('alert')).toContainText('Tidligere resultater er skjult')
  await sharingLayout(page, 'public-error', '.public-results-page')
  mode = 'hidden'; await page.getByRole('button', { name: 'Prøv igjen', exact: true }).click()
  await expect(page.getByText('Finalens bakni er skjult. Bare synlige resultater inngår her.')).toBeVisible()
  await expect(page.locator('.public-result-row')).toHaveCount(0)
  await page.getByRole('button', { name: 'Brutto', exact: true }).click()
  await expect(page.getByText('Ingen resultater å vise ennå.')).toBeVisible()
  await sharingLayout(page, 'public-empty', '.public-results-page')
  mode = 'ready'; await page.getByRole('button', { name: 'Oppdater resultater' }).click()
  await expect(page.locator('.public-result-row')).toHaveCount(3)
  mode = 'loading'; deferred = new Promise(resolve => { release = resolve })
  await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')))
  await expect(page.locator('.public-result-row')).toHaveCount(0)
  await page.evaluate(() => { window.location.hash = `token=${'b'.repeat(43)}` })
  await expect(page.getByRole('alert')).toContainText('Den kan være utløpt')
  mode = 'ready'; release?.()
  await expect(page.locator('.public-result-row')).toHaveCount(0)
  const terminalReads = reads
  await page.clock.fastForward(30_000)
  expect(reads).toBe(terminalReads)
  await page.evaluate(() => { window.location.hash = '' })
  await expect(page.getByRole('alert')).toContainText('Åpne hele lenken')
  await page.goBack()
  await expect(page.getByRole('alert')).toContainText('Den kan være utløpt')
  await page.goBack()
  await expect(page.locator('.public-result-row')).toHaveCount(3)
  await page.context().setOffline(true)
  await page.evaluate(() => window.dispatchEvent(new Event('offline')))
  await expect(page.locator('.public-result-row')).toHaveCount(0)
  await page.context().setOffline(false)
  await page.evaluate(() => window.dispatchEvent(new Event('online')))
  await expect(page.locator('.public-result-row')).toHaveCount(3)
  expires = new Date(await page.evaluate(() => Date.now()) + 1000).toISOString()
  await page.getByRole('button', { name: 'Oppdater resultater' }).click()
  await expect(page.locator('.public-result-row')).toHaveCount(3)
  await page.clock.fastForward(1000)
  await expect(page.getByRole('alert')).toContainText('Den kan være utløpt')
  await sharingLayout(page, 'public-expired', '.public-results-page')
  expect(events.errors).toEqual([]); expect(events.privateReads).toEqual([])
  expect(events.statuses.every(status => [401, 404, 503].includes(status))).toBe(true)
  expect(events.failed.every(error => ['net::ERR_ABORTED', 'net::ERR_INTERNET_DISCONNECTED'].includes(error))).toBe(true)
})
