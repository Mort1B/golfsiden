import { test, expect, type Page, type APIRequestContext } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeObject } from '../src/api/decoder'
import { decodeTournamentList, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodeCompletionValidation, decodeScoringScorecard, ownerTypeForFormat } from '../src/api/scorecards'

test.skip(process.env.GOLF_FLIGHT_PROGRESS_BROWSER !== '1', 'Requires fresh disposable seed and GOLF_FLIGHT_PROGRESS_BROWSER=1.')

async function login(page: Page, username: string) {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill(username)
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
}
async function mutate(api: APIRequestContext, path: string, csrf: string, data: unknown = {}, method = 'POST') {
  const result = await api.fetch(path, { method, data, headers: { 'x-csrf-token': csrf } })
  expect(result.ok(), `${method} ${path}: ${result.status()}`).toBe(true)
}
async function layout(page: Page, name: string) {
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    await page.locator('.flight-progress').scrollIntoViewIfNeeded()
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    await page.screenshot({ path: `/tmp/golf-flight-progress-${name}-${width}.png`, fullPage: true })
  }
}

test('flight progress counts cards once and follows live scores, confirmations and final release/re-hide', async ({ page, browser }) => {
  await login(page, 'anders')
  const errors: string[] = []
  const failed: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
  page.on('response', (response) => { if (response.status() >= 400) failed.push(`${response.status()} ${new URL(response.url()).pathname}`) })
  page.on('requestfailed', (request) => { if (!request.url().includes('/live') && !request.failure()?.errorText.includes('ERR_ABORTED')) failed.push(new URL(request.url()).pathname) })
  const admin = await browser.newContext()
  const adminPage = await admin.newPage()
  await login(adminPage, 'admin')
  const api = admin.request
  const auth = decodeAuthSession(await (await api.get('/api/auth/session')).json())
  const trips = decodeTournamentList(await (await api.get('/api/tournaments')).json())
  const trip = trips.find((item) => item.name === 'Guttas Golf 2026')
  if (!trip) throw new Error('Fresh seed required')
  const rounds = decodeTournamentRounds(await (await api.get(`/api/tournaments/${trip.id}/rounds`)).json(), trip.id)
  await mutate(api, `/api/tournaments/${trip.id}/start`, auth.csrf_token, { expected_tournament_updated_at: trip.updated_at })
  const panel = page.locator('.flight-progress')
  for (const format of ['team_scramble', 'two_player_foursomes', 'individual_stroke_play']) {
    const round = rounds.find((item) => item.scoring_format === format)
    if (!round) throw new Error(`Missing format ${format}`)
    await mutate(api, `/api/rounds/${round.id}/open`, auth.csrf_token)
    await page.goto(`/rounds/${round.id}`)
    const count = format === 'individual_stroke_play' ? 4 : 2
    await expect(panel.getByText(`0/${count * 18} hullregistreringer fordelt på ${count} scorekort`)).toHaveCount(2)
    await layout(page, format)
    const progress = decodeCompletionValidation(await (await api.get(`/api/rounds/${round.id}/completion-validation`)).json(), round.id, ownerTypeForFormat(round.scoring_format))
    const first = progress.owners[0]
    if (!first) throw new Error('Required owner missing')
    const card = decodeScoringScorecard(await (await api.get(`/api/rounds/${round.id}/scorecards/${first.owner.type}/${first.owner.id}/scoring`)).json(), round.id, first.owner)
    const hole = card.holes[0]
    if (!hole) throw new Error('Hole missing')
    await mutate(api, `/api/rounds/${round.id}/scores`, auth.csrf_token, { owner: first.owner, hole_id: hole.hole_id, gross_strokes: hole.par }, 'PUT')
    await expect(panel.getByText(`1/${count * 18} hullregistreringer fordelt på ${count} scorekort`)).toBeVisible()
  }
  const final = rounds.find((round) => round.round_number === 5)
  if (!final) throw new Error('Final missing')
  await mutate(api, `/api/rounds/${final.id}/open`, auth.csrf_token)
  await page.goto(`/rounds/${final.id}`)
  await expect(panel.getByText('Kun hull 1–9 vises. Fullføring og bekreftelse er skjult.')).toBeVisible()
  const progress = decodeCompletionValidation(await (await api.get(`/api/rounds/${final.id}/completion-validation`)).json(), final.id, ownerTypeForFormat(final.scoring_format))
  const first = progress.owners[0]
  if (!first) throw new Error('Final owner missing')
  const ownerPath = `/api/rounds/${final.id}/scorecards/${first.owner.type}/${first.owner.id}`
  const card = decodeScoringScorecard(await (await api.get(`${ownerPath}/scoring`)).json(), final.id, first.owner)
  for (const hole of card.holes) await mutate(api, `/api/rounds/${final.id}/scores`, auth.csrf_token, { owner: first.owner, hole_id: hole.hole_id, gross_strokes: hole.par }, 'PUT')
  await mutate(api, `${ownerPath}/confirm`, auth.csrf_token)
  await expect(panel.getByText('9/36 synlige hullregistreringer fordelt på 4 scorekort')).toBeVisible()
  await expect(panel.getByText(/fullført|Bekreftet/)).toHaveCount(0)
  await layout(page, 'hidden')
  const visibilityPath = `/api/tournaments/${trip.id}/final-round-visibility`
  const visibility = async (hidden: boolean) => {
    const current = decodeObject(await (await api.get(visibilityPath)).json(), 'visibility')
    await mutate(api, visibilityPath, auth.csrf_token, { back_nine_hidden: hidden, expected_visibility_updated_at: current.visibility_updated_at }, 'PATCH')
  }
  await visibility(false)
  await expect(panel.getByText('1/4 fullført · 1/4 bekreftet')).toBeVisible()
  await expect(panel.getByText('18/72 hullregistreringer fordelt på 4 scorekort')).toBeVisible()
  await layout(page, 'released')
  const hole = card.holes[0]
  if (!hole) throw new Error('Final hole missing')
  await mutate(api, `/api/rounds/${final.id}/scores`, auth.csrf_token, { owner: first.owner, hole_id: hole.hole_id, gross_strokes: hole.par + 1 }, 'PUT')
  await expect(panel.getByText('1/4 fullført · 0/4 bekreftet')).toBeVisible()
  await mutate(api, `${ownerPath}/confirm`, auth.csrf_token)
  await expect(panel.getByText('1/4 fullført · 1/4 bekreftet')).toBeVisible()
  await visibility(true)
  await expect(panel.getByText('9/36 synlige hullregistreringer fordelt på 4 scorekort')).toBeVisible()
  await expect(panel.getByText(/fullført|Bekreftet|18\/72/)).toHaveCount(0)
  expect(errors).toEqual([])
  expect(failed).toEqual([])

  // Deliberate edge states are injected only after real API/live checks.
  const path = `**/api/rounds/${final.id}/completion-validation`
  let mode: 'pending' | 'empty' | 'error' | 'long' = 'pending'
  let release: (() => void) | undefined
  await page.route(path, async (route) => {
    if (mode === 'pending') await new Promise<void>((resolve) => { release = resolve })
    if (mode === 'error') { await route.fulfill({ status: 403, json: { error: { code: 'forbidden', message: 'Ingen tilgang' } } }); return }
    await route.fulfill({ json: { round_id: final.id, status: 'open', visibility: { mode: 'front_nine' }, ready_to_complete: null, ready_to_lock: null, issues: [],
      owners: mode === 'long' ? progress.owners.map((owner) => ({ ...owner, owner_name: 'LangtSpillernavnUtenMellomrom'.repeat(10), holes_scored: 9, required_holes: 9, complete: null, confirmed: null })) : [] } })
  })
  await page.reload()
  await expect.poll(() => Boolean(release)).toBe(true)
  await expect(panel.getByRole('status')).toBeVisible()
  await layout(page, 'loading')
  mode = 'empty'
  release?.()
  await expect(panel.getByText('Ingen scorekort med flighttilknytning er tilgjengelige.')).toBeVisible()
  await layout(page, 'empty')
  mode = 'long'
  await page.reload()
  await expect(panel.getByText('36/36 synlige hullregistreringer fordelt på 4 scorekort')).toHaveCount(2)
  await layout(page, 'long')
  mode = 'error'
  await page.reload()
  await expect(panel.getByRole('alert')).toBeVisible({ timeout: 20000 })
  await expect(panel.getByText(/36\/36/)).toHaveCount(0)
  await layout(page, 'error')
  mode = 'empty'
  await panel.getByRole('button', { name: 'Prøv igjen' }).click()
  await expect(panel.getByText('Ingen scorekort med flighttilknytning er tilgjengelige.')).toBeVisible()
  await admin.close()
})
