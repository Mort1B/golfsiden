import { test, expect, type Page } from '@playwright/test'
import { decodeTournamentList, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import type { Round } from '../src/api/types'
import type { PairingValidation } from '../src/api/roundLifecycle'
import type { RoundCompletionValidation } from '../src/api/scorecards'

test.skip(process.env.GOLF_ORGANIZER_BROWSER !== '1', 'Requires disposable local services and GOLF_ORGANIZER_BROWSER=1.')

async function layout(page: Page, state: string) {
  const summary = page.getByRole('region', { name: 'Dette trenger oppfølging' })
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const control of await summary.locator('a:visible, button:visible').all()) {
      expect((await control.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await control.isEnabled()) await control.click({ trial: true })
    }
    await summary.screenshot({ path: `/tmp/golf-organizer-${state}-${width}.png` })
  }
}

async function login(page: Page) {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill('admin')
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
}

function opening(round: Round): PairingValidation {
  return { round_id: round.id, ready: true, issues: [], missing_players: [], ineligible_players: [], team_sizes: [],
    missing_flight_players: [], ineligible_flight_players: [], flight_sizes: [], legacy_individual_groups: [], split_teams: [] }
}
function completion(round: Round): RoundCompletionValidation {
  const owners = [{ owner: { type: 'player' as const, id: '00000000-0000-0000-0000-000000001001' },
    owner_name: 'Spiller med et svært langt navn som må være leselig på mobil', holes_scored: 18, required_holes: 18, complete: true, confirmed: round.status === 'completed' }]
  if (round.status === 'open') owners.push({ ...owners[0], owner: { type: 'player', id: '00000000-0000-0000-0000-000000001002' },
    owner_name: 'Ufullstendig spiller', holes_scored: 1, required_holes: 18, complete: false, confirmed: false })
  return { round_id: round.id, status: round.status, owners, visibility: { mode: 'full' }, ready_to_complete: false,
    ready_to_lock: round.status === 'completed', issues: round.status === 'completed' ? [{ code: 'round_not_open', message: 'completed' }]
      : [{ code: 'incomplete_scorecards', message: 'incomplete' }, { code: 'unconfirmed_scorecards', message: 'unconfirmed' }, { code: 'round_not_completed', message: 'open' }] }
}

test('ordered tasks, exact repeatable destinations, refresh and membership loss', async ({ page }) => {
  await login(page)
  const trips = decodeTournamentList(await (await page.request.get('/api/tournaments')).json())
  const trip = trips.find(item => item.name === 'Guttas Golf 2026')
  if (!trip) throw new Error('Missing seed tournament')
  const stored = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${trip.id}/rounds`)).json(), trip.id)
  const statuses = ['draft', 'open', 'completed', 'draft', 'locked'] as const
  const rounds = stored.map((round, index) => ({ ...round, status: statuses[index] ?? 'locked', scoring_format: 'individual_stroke_play' as const,
    name: `Et langt rundenavn for å kontrollere oppfølging og lenker på mobil ${round.round_number}` }))
  const first = rounds[0]; const fourth = rounds[3]
  if (!first || !fourth) throw new Error('Missing seed rounds')
  const errors: string[] = []; const failed: string[] = []; const mutations: string[] = []
  page.on('pageerror', e => errors.push(e.message))
  page.on('console', m => { if (m.type() === 'error' && !m.text().startsWith('Failed to load resource:')) errors.push(m.text()) })
  page.on('request', r => { if (r.method() !== 'GET' && new URL(r.url()).pathname.startsWith('/api/')) mutations.push(r.method()) })
  page.on('response', r => {
    if (r.status() >= 400 && !(r.status() === 503 && new URL(r.url()).pathname === `/api/rounds/${first.id}/pairing-validation`)) failed.push(`${r.status()} ${new URL(r.url()).pathname}`)
  })
  let empty = false; let locked = false; let denied = false; let fail = false
  let release: () => void = () => { throw new Error('Read not started') }
  const pending = new Promise<void>(resolve => { release = resolve })
  await page.route(`**/api/tournaments/${trip.id}`, route => route.fulfill({ json: { ...trip, status: 'active' } }))
  await page.route('**/api/me/tournaments', route => route.fulfill({ json: [{ tournament: { ...trip, status: 'active' }, role: denied ? 'viewer' : 'admin', player_id: null }] }))
  await page.route(`**/api/tournaments/${trip.id}/rounds`, route => route.fulfill({ json: empty ? [] : [...rounds].reverse().map(round => locked ? { ...round, status: 'locked' } : round) }))
  for (const round of rounds) {
    await page.route(`**/api/rounds/${round.id}`, route => route.fulfill({ json: locked ? { ...round, status: 'locked' } : round }))
    await page.route(`**/api/rounds/${round.id}/pairing-validation`, async route => {
      if (round.id === first.id) {
        await pending
        if (fail) return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Test failure' } } })
      }
      const validation = opening(round)
      if (round.id === fourth.id) {
        validation.ready = false
        validation.missing_flight_players = [{ player_id: '00000000-0000-0000-0000-000000001001', display_name: 'One' }, { player_id: '00000000-0000-0000-0000-000000001002', display_name: 'Two' }]
        validation.issues = [{ code: 'missing_flight_assignment', message: 'flight' }, { code: 'missing_course', message: 'course' }]
      }
      return route.fulfill({ json: validation })
    })
    await page.route(`**/api/rounds/${round.id}/completion-validation`, route => route.fulfill({ json: completion(round) }))
  }
  await page.goto(`/manage/tournaments/${trip.id}`)
  const summary = page.getByRole('region', { name: 'Dette trenger oppfølging' })
  await expect(summary.getByText(/Kontrollerer Runde 1/)).toBeVisible()
  await layout(page, 'loading')
  release()
  await expect(summary.getByText('Runde 1 er klar til å åpnes.')).toBeVisible()
  await expect(summary.getByText('Runde 2: 1 scorekort må bekreftes.')).toBeVisible()
  await expect(summary.getByText('Runde 2 har 1 ufullstendig scorekort.')).toBeVisible()
  await expect(summary.getByText('Runde 3 er klar til å låses.')).toBeVisible()
  await expect(summary.getByText('Runde 4 har 2 spillere uten flight.')).toBeVisible()
  expect(await summary.getByRole('heading', { level: 3 }).allTextContents()).toEqual(rounds.slice(0, 4).map(round => `Runde ${round.round_number}: ${round.name}`))
  await expect(summary.getByRole('link', { name: /Bekreft scorekort for/ })).toHaveAttribute('href', /owner_type=player.*view=summary/)
  await layout(page, 'populated-long')
  await summary.getByRole('link', { name: 'Se åpning' }).focus()
  await page.keyboard.press('Enter')
  await expect(page.locator('#lifecycle')).toBeFocused()
  await expect(page).toHaveURL(new RegExp(`round=${first.id}#lifecycle$`))
  for (const destination of ['pairings', 'courses'] as const) {
    const label = destination === 'pairings' ? 'Fordel i flighter' : 'Se baneoppsett'
    const editor = page.locator(`#${destination === 'pairings' ? 'pairing' : 'course'}-editor-${fourth.id}`)
    await summary.getByRole('link', { name: label }).click()
    await expect(editor).toBeVisible()
    await expect(page.locator(`#${destination}`)).toBeFocused()
    const card = editor.locator('..')
    if (destination === 'pairings') await card.getByRole('button', { name: 'Lukk', exact: true }).click()
    else await card.getByRole('button', { name: 'Endre', exact: true }).click()
    await expect(editor).toBeHidden()
    await summary.getByRole('link', { name: label }).click()
    await expect(editor).toBeVisible()
  }
  fail = true
  await summary.getByRole('button', { name: 'Oppdater oversikten' }).click()
  await expect(summary.getByRole('alert')).toContainText('kunne ikke hentes')
  await expect(summary.getByText('Runde 1 er klar til å åpnes.')).toHaveCount(0)
  await layout(page, 'error')
  fail = false
  await summary.getByRole('button', { name: 'Prøv igjen for runde 1' }).click()
  await expect(summary.getByText('Runde 1 er klar til å åpnes.')).toBeVisible()
  locked = true
  await summary.getByRole('button', { name: 'Oppdater oversikten' }).click()
  await expect(summary.getByText(/Alle rundene er låst/)).toBeVisible()
  await layout(page, 'all-locked')
  empty = true
  await summary.getByRole('button', { name: 'Oppdater oversikten' }).click()
  await expect(summary.getByText('Ingen runder å følge opp ennå.')).toBeVisible()
  await layout(page, 'empty')
  denied = true
  await summary.getByRole('button', { name: 'Oppdater oversikten' }).click()
  await expect(page.getByRole('heading', { name: 'Ingen tilgang' })).toBeVisible()
  await expect(summary).toHaveCount(0)
  expect(mutations).toEqual([])
  expect(errors).toEqual([])
  expect(failed).toEqual([])
})
