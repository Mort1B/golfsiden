import { test, expect } from '@playwright/test'
import { routeWorkspace, routeLayout, selectPlayerListing, trip, matchRound, matchCard } from './routeSplittingSupport'

const listPath = `/api/rounds/${matchRound.id}/match-play/matches`
const tablePath = `/api/tournaments/${trip.id}/match-table`
const paths = {
  direct: `/tournaments/${trip.id}/match-results`,
  history: `/tournaments/${trip.id}/results/players/${matchCard.opponents[0].player_id}?metric=net`,
  global: `/leaderboard?tournament=${trip.id}&scope=tournament&metric=net`,
}

for (const [name, path] of Object.entries(paths)) for (const width of [320, 390, 1280]) {
  test(`gated ${name} decodes restricted finals without exposing old content at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: width === 320 ? 600 : 900 })
    const f = await routeWorkspace(page)
    let hold = true, restricted = false, empty = false, tableReads = 0, listReads = 0, listCompleted = 0, listAborts = 0
    const releases: Array<() => void> = []
    const release = () => { hold = false; for (const done of releases.splice(0)) done() }
    page.on('requestfinished', request => {
      if (new URL(request.url()).pathname === listPath) listCompleted++
    })
    page.on('requestfailed', request => {
      if (new URL(request.url()).pathname === listPath && request.failure()?.errorText === 'net::ERR_ABORTED') listAborts++
    })
    await page.route('**/api/**', async route => {
      const endpoint = new URL(route.request().url()).pathname
      if (endpoint === '/api/tournaments') return route.fulfill({ json: [{ ...trip, counted_rounds: null }] })
      if (endpoint === `/api/tournaments/${trip.id}/rounds`) return route.fulfill({ json: [matchRound] })
      if (endpoint === tablePath) {
        tableReads++
        if (hold) await new Promise<void>(resolve => releases.push(resolve))
        return route.fulfill({ json: { tournament_id: trip.id, entries: empty ? [] : matchCard.opponents.map((p, i) => ({
          player_id: p.player_id, display_name: `${p.display_name} fra golfklubben ved den lange fjorden`,
          half_points: restricted || i === 1 ? 0 : 2, played: restricted ? 0 : 1,
          wins: !restricted && i === 0 ? 1 : 0, draws: 0, losses: !restricted && i === 1 ? 1 : 0, position: restricted ? null : i + 1,
        })) } })
      }
      if (endpoint === listPath) {
        listReads++
        const { revision: _revision, accepted_events: _events, ...card } = matchCard
        void _revision; void _events
        const front = Array.from({ length: 9 }, (_, i) => ({ type: 'hole', hole_number: i + 1, outcome: 'halved',
          basis: { type: 'numeric', first_gross: 4, second_gross: 4, agreed: true } }))
        const full = { ...card, resolved_holes: 9, events: [...front, { type: 'concession', conceding_player_id: card.opponents[1].player_id, communicated: true, after_hole: 9 }],
          finish: { type: 'conceded', winner: 'first' }, confirmed: true, half_points: [2, 0] }
        const hidden = { ...card, events: front, resolved_holes: 9, holes: card.holes.slice(0, 9), visibility: { mode: 'front_nine' }, confirmed: null, correction_pending: null }
        return route.fulfill({ json: selectPlayerListing({ round_id: matchRound.id, matches: empty ? [] : [restricted ? hidden : full], writable_match_ids: restricted || empty ? [] : [card.match_id] }, route.request().url()) })
      }
      return route.fallback()
    })
    const noPrivateDom = async () => {
      await expect(page.locator('.match-list, .match-table, .match-actions')).toHaveCount(0)
      await expect(page.getByRole('heading', { name: matchRound.name, exact: true })).toHaveCount(0)
      await expect(page.getByText('Bekreftet resultat', { exact: true })).toHaveCount(0)
      await expect(page.getByText(/fra golfklubben ved den lange fjorden/)).toHaveCount(0)
    }
    try {
      await page.goto(path)
      await expect.poll(() => tableReads).toBeGreaterThan(0)
      await expect.poll(() => f.live.connections).toBeGreaterThan(0)
      await noPrivateDom(); expect(listReads).toBe(0)
      await routeLayout(page, `gate-initial-${name}`)
      release()
      await expect(page.getByText('Bekreftet resultat', { exact: true })).toBeVisible()
      await routeLayout(page, `gate-full-${name}`)
      const before = listReads, beforeCompleted = listCompleted, beforeAborts = listAborts, beforeTables = tableReads
      hold = true; restricted = true
      const restrictedResponse = page.waitForResponse(response => new URL(response.url()).pathname === listPath)
      f.live.emit('visibility')
      await expect.poll(() => tableReads).toBeGreaterThan(beforeTables)
      await noPrivateDom()
      await (await restrictedResponse).finished()
      await noPrivateDom()
      await routeLayout(page, `gate-hidden-${name}`)
      // Exercise the real freshness deadline once; the other cases reopen while fresh.
      const stale = name === 'direct' && width === 320
      if (stale) { await page.waitForTimeout(21_000); await noPrivateDom() }
      release()
      await expect(page.getByText('Fullføring og poeng er skjult til finalen frigis.', { exact: true })).toBeVisible()
      await expect(page.getByRole('link', { name: 'Før match', exact: true })).toHaveCount(0)
      await expect(page.getByText('Bekreftet resultat', { exact: true })).toHaveCount(0)
      await expect.poll(() => listReads - before).toBe(stale ? 2 : 1)
      await expect.poll(() => listCompleted - beforeCompleted).toBe(stale ? 2 : 1)
      expect(listAborts - beforeAborts).toBe(0)
      await routeLayout(page, `gate-restricted-${name}`)
      await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight))
      await routeLayout(page, `gate-restricted-bottom-${name}`)
      if (width < 700) {
        const card = await page.locator('.match-list article').boundingBox()
        const nav = await page.getByRole('navigation', { name: 'Hovedmeny' }).boundingBox()
        expect(card && nav && card.y + card.height <= nav.y).toBe(true)
      }
      empty = true; f.live.emit('visibility')
      await expect(page.getByText('Ingen matcher er satt opp for dette valget.', { exact: true })).toBeVisible()
      await routeLayout(page, `gate-empty-${name}`)
    } finally { release(); await f.close() }
  })
}
