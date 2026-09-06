import { test, expect, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeObject } from '../src/api/decoder'
import { decodeTournamentList, decodeTournamentRounds } from '../src/api/tournaments/decoders'

// Uses and opens a round in an explicitly disposable seeded local database.
test.skip(process.env.GOLF_ROUND_DETAILS_BROWSER !== '1', 'Requires disposable seeded database and GOLF_ROUND_DETAILS_BROWSER=1.')

async function login(page: Page, name: string) {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill(name)
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
}

async function layout(page: Page, width: number) {
  await page.setViewportSize({ width, height: 900 })
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
  for (const link of await page.locator('.round-detail-links a, .round-details .back-button').all()) {
    const box = await link.boundingBox()
    expect(box?.height).toBeGreaterThanOrEqual(44)
    expect(box?.width).toBeGreaterThanOrEqual(44)
  }
}

test('member round details: formats, live opening, navigation and resilient layouts', async ({ page, browser }) => {
  const errors: string[] = []
  const failed: string[] = []
  const badResponses: number[] = []
  const detailRequests: string[] = []
  await login(page, 'anders')
  page.on('pageerror', (error) => errors.push(error.message))
  page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
  page.on('requestfailed', (request) => {
    if (!request.url().includes('/live') && !request.failure()?.errorText.includes('ERR_ABORTED')) failed.push(new URL(request.url()).pathname)
  })
  page.on('response', (response) => { if (response.status() >= 400) badResponses.push(response.status()) })
  page.on('request', (request) => { detailRequests.push(new URL(request.url()).pathname) })
  const api = page.context().request
  const trips = decodeTournamentList(await (await api.get('/api/tournaments')).json())
  const trip = trips.find((item) => item.name === 'Guttas Golf 2026')
  if (!trip) throw new Error('Seed tournament missing')
  const rounds = decodeTournamentRounds(await (await api.get(`/api/tournaments/${trip.id}/rounds`)).json(), trip.id)
  for (const format of ['individual_stroke_play', 'team_scramble', 'two_player_foursomes']) {
    const round = rounds.find((item) => item.scoring_format === format)
    if (!round) throw new Error(`Missing seeded format ${format}`)
    await page.goto(`/rounds/${round.id}`)
    await expect(page.getByRole('heading', { name: 'Flighter', exact: true })).toBeVisible()
    await expect(page.locator('.flight-schedule')).toHaveCount(2)
    await expect(page.getByRole('heading', { name: 'Lag', exact: true })).toHaveCount(format === 'individual_stroke_play' ? 0 : 1)
    await expect(page.getByRole('link', { name: 'Administrer runden' })).toHaveCount(0)
    for (const width of [320, 390, 1280]) {
      await layout(page, width)
      await page.screenshot({ path: `/tmp/golf-round-details-${format}-${width}.png`, fullPage: true })
    }
  }
  expect(detailRequests.filter((path) => /\/teams$|score-access|completion-validation|\/scoring$|\/scores$/.test(path))).toEqual([])

  const round = rounds.find((item) => item.scoring_format === 'individual_stroke_play')
  if (!round) throw new Error('Individual seed missing')
  await page.goto(`/rounds/${round.id}`)
  await expect(page.getByRole('link', { name: 'Åpne scorekort' })).toHaveCount(0)
  const admin = await browser.newContext()
  const adminPage = await admin.newPage()
  await login(adminPage, 'admin')
  const auth = decodeAuthSession(await (await admin.request.get('/api/auth/session')).json())
  const headers = { 'x-csrf-token': auth.csrf_token }
  if (trip.status === 'draft') expect((await admin.request.post(`/api/tournaments/${trip.id}/start`, { headers, data: { expected_tournament_updated_at: trip.updated_at } })).ok()).toBe(true)
  if (round.status === 'draft') expect((await admin.request.post(`/api/rounds/${round.id}/open`, { headers })).ok()).toBe(true)
  // The already-mounted member page must refresh through the existing SSE boundary.
  await expect(page.getByRole('link', { name: 'Åpne scorekort' })).toBeVisible({ timeout: 20000 })
  await page.getByRole('link', { name: 'Åpne scorekort' }).click()
  await expect(page).toHaveURL(new RegExp(`round=${round.id}`))
  await expect(page.getByRole('heading', { name: 'Score', exact: true })).toBeVisible()
  await page.goto(`/rounds/${round.id}`)
  await page.getByRole('link', { name: 'Se rundens resultater' }).click()
  await expect(page).toHaveURL(new RegExp(`scope=round&round=${round.id}`))
  await admin.close()

  // Inject presentation edge cases into actual decoded seed responses only.
  const endpoint = `**/api/rounds/${round.id}/pairings`
  const original = decodeObject(await (await api.get(`/api/rounds/${round.id}/pairings`)).json(), 'pairings')
  if (!Array.isArray(original.flights) || !original.flights[0]) throw new Error('Seed flights missing')
  const flight = decodeObject(original.flights[0], 'flight')
  const longName = 'LangtFlightNavnUtenMellomrom'.repeat(12)
  let mode: 'loading' | 'empty' | 'long' | 'error' = 'loading'
  let release: (() => void) | undefined
  await page.route(endpoint, async (route) => {
    if (mode === 'loading') await new Promise<void>((resolve) => { release = resolve })
    if (mode === 'error') { await route.fulfill({ status: 403, json: { error: { code: 'forbidden', message: 'Ingen tilgang' } } }); return }
    await route.fulfill({ json: { ...original, flights: mode === 'long' ? [{ ...flight, name: longName, tee_time: null, starting_hole: null }] : [] } })
  })
  await page.goto(`/rounds/${round.id}`)
  await expect.poll(() => Boolean(release)).toBe(true)
  await expect(page.getByText('Laster …', { exact: true })).toBeVisible()
  await layout(page, 320)
  mode = 'empty'
  release?.()
  await expect(page.getByText('Ingen flighter er satt opp.')).toBeVisible()
  mode = 'long'
  await page.reload()
  await expect(page.getByRole('heading', { name: longName })).toBeVisible()
  await expect(page.getByText('Starttid ikke satt')).toBeVisible()
  for (const width of [320, 1280]) {
    await layout(page, width)
    await page.screenshot({ path: `/tmp/golf-round-details-long-${width}.png`, fullPage: true })
  }
  expect(errors).toEqual([])
  expect(failed).toEqual([])
  expect(badResponses).toEqual([])
  // Expected denied read is checked separately from the clean happy-path diagnostics.
  mode = 'error'
  await page.reload()
  await expect(page.getByRole('alert')).toBeVisible({ timeout: 20000 })
  await expect(page.getByRole('heading', { name: longName })).toHaveCount(0)
  await layout(page, 320)
  mode = 'empty'
  await page.getByRole('button', { name: 'Prøv igjen' }).click()
  await expect(page.getByText('Ingen flighter er satt opp.')).toBeVisible()
})
