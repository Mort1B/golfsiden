import { test, expect, type Page } from '@playwright/test'
import { routeWorkspace, trip, matchRound, matchCard, matchListUrl } from './routeSplittingSupport'
const results = `/tournaments/${trip.id}/match-results`
const listPath = `/api/rounds/${matchRound.id}/match-play/matches`
const tablePath = `/api/tournaments/${trip.id}/match-table`
const manage = `/manage/tournaments/${trip.id}?round=${matchRound.id}#pairings`

async function workspace(page: Page) {
  const f = await routeWorkspace(page)
  const state = { hold: true, reads: 0, fail: '', aborted: 0 }
  const releases: Array<() => void> = []
  page.on('requestfailed', request => { if (new URL(request.url()).pathname === listPath && request.failure()?.errorText === 'net::ERR_ABORTED') state.aborted++ })
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname
    if (path === `/api/tournaments/${trip.id}/rounds`) return route.fulfill({ json: [matchRound] })
    if (path === `/api/tournaments/${trip.id}`) return route.fulfill({ json: { ...trip, counted_rounds: null } })
    if (path === `/api/rounds/${matchRound.id}/pairings`) return route.fulfill({ json: {
      round_id: matchRound.id, tournament_id: trip.id, status: matchRound.status, scoring_format: matchRound.scoring_format,
      updated_at: matchRound.updated_at, active_entrants: matchCard.opponents.map(p => ({ player_id: p.player_id, display_name: p.display_name, status: 'active', player_active: true })),
      inactive_entrants: [], teams: [], flights: [], legacy_individual_groups: [],
    } })
    if (path === `/api/rounds/${matchRound.id}/match-play/completion`) return route.fulfill({ json: {
      format: 'singles_match_play', round_id: matchRound.id, status: matchRound.status, matches: [], ready_to_complete: false, ready_to_lock: false,
    } })
    if (path === state.fail) return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Matchdata er midlertidig utilgjengelig' } } })
    if (path === tablePath) return route.fulfill({ json: { tournament_id: trip.id, entries: matchCard.opponents.map(p => ({ player_id: p.player_id, display_name: p.display_name,
      half_points: 0, played: 0, wins: 0, draws: 0, losses: 0, position: null })) } })
    if (path === listPath) {
      state.reads++
      const card = Object.fromEntries(Object.entries(matchCard).filter(([key]) => !['revision', 'accepted_events'].includes(key)))
      card.opponents = [{ ...matchCard.opponents[0], display_name: f.state.auth?.display_name ?? 'Tidligere konto' }, matchCard.opponents[1]]
      if (state.hold) await new Promise<void>(resolve => releases.push(resolve))
      return route.fulfill({ json: { round_id: matchRound.id, matches: [card], writable_match_ids: [matchCard.match_id] } })
    }
    return route.fallback()
  })
  return { ...f, flow: state, release() { state.hold = false; for (const release of releases.splice(0)) release() } }
}

for (const width of [320, 390, 1280]) test(`pending result and management navigation preserves the shared list at ${width}px`, async ({ page }) => {
  await page.setViewportSize({ width, height: width === 320 ? 600 : 900 })
  const f = await workspace(page)
  try {
    await page.goto(results)
    await expect.poll(() => f.flow.reads).toBeGreaterThan(0)
    const beforeNavigation = f.flow.aborted
    await page.getByRole('link', { name: 'Turneringen', exact: true }).click()
    await expect.poll(() => f.flow.aborted).toBeGreaterThan(beforeNavigation)
    await page.getByRole('link', { name: 'Åpne administrasjon', exact: true }).click()
    await page.getByRole('link', { name: 'Spillegrupper', exact: true }).click()
    await page.locator('.round-pairing-card').getByRole('button', { name: 'Vis', exact: true }).click()
    f.release()
    await expect(page.getByRole('region', { name: `Matchoppsett ${matchRound.name}` })).toBeVisible()
    await expect(page.getByRole('combobox', { name: 'Første spiller', exact: true })).toHaveValue(matchCard.opponents[0].player_id)
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    await page.screenshot({ path: `/tmp/cancel-management-${width}.png` })
    // New document starts with the unchanged management-origin consumer and a pending read.
    f.flow.hold = true
    const before = f.flow.reads
    await page.goto(manage)
    await expect.poll(() => f.flow.reads).toBeGreaterThan(before)
    await page.getByRole('link', { name: 'Se matcher og bekreftelser', exact: true }).click()
    await expect(page).toHaveURL(matchListUrl)
    f.release()
    await expect(page.locator('.match-list article')).toHaveCount(1)
    await expect(page.locator('.match-list')).toContainText(f.state.auth?.display_name ?? '')
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  } finally { f.release(); await f.close() }
})

test('logout cancels pending protected list reads and a new account loads fresh content', async ({ page }) => {
  const f = await workspace(page)
  try {
    await page.goto(results)
    await expect.poll(() => f.flow.reads).toBeGreaterThan(0)
    const former = f.state.auth?.display_name ?? ''
    const beforeLogout = f.flow.aborted
    await page.getByRole('button', { name: 'Logg ut', exact: true }).click()
    await expect(page.getByRole('heading', { name: 'Logg inn', exact: true })).toBeVisible()
    await expect.poll(() => f.flow.aborted).toBeGreaterThan(beforeLogout)
    f.state.loginAs = { ...f.state.loginAs, user_id: '00000000-0000-0000-0000-000000000099', username: 'replacement', display_name: 'Ny matchkonto' }
    f.release()
    await page.getByLabel('Brukernavn', { exact: true }).fill('replacement')
    await page.getByLabel('Passord', { exact: true }).fill('synthetic-unused-password')
    await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
    await expect(page.locator('.match-list')).toContainText('Ny matchkonto')
    await expect(page.locator('.match-list')).not.toContainText(former)
  } finally { f.release(); await f.close() }
})

for (const path of [listPath, tablePath]) test(`uncancelled error on ${path.endsWith('/matches') ? 'list' : 'table'} remains recoverable`, async ({ page }) => {
  const f = await workspace(page)
  try {
    f.release(); f.flow.fail = path
    await page.goto(results)
    await expect(page.getByRole('alert').filter({ hasText: 'Matchdata er midlertidig utilgjengelig' })).toBeVisible()
    f.flow.fail = ''
    await page.getByRole('button', { name: 'Prøv igjen', exact: true }).click()
    await expect(page.locator('.match-list article')).toHaveCount(1)
    expect(f.errors.length).toBeGreaterThan(0)
    expect(f.errors.every(error => error === `HTTP 503 ${path}`)).toBe(true)
    f.errors.splice(0)
  } finally { f.release(); await f.close() }
})
