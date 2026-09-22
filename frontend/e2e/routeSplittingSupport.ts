import { expect, type Page } from '@playwright/test'
import type { AuthSession } from '../src/api/auth'
import { decodeObject } from '../src/api/decoder'
import { matchFixture, matchIds } from '../src/api/matchPlay/fixtures'
import { publicFixture, shareId, shareSecret } from '../src/api/resultSharing/__tests__/fixtures'
import { tournament, session } from '../src/features/tournaments/lifecycle/__tests__/fixtures'
import { liveServer, mockWorkspace, round, scoreUrl } from './returnLoadingSupport'
export { shareId, shareSecret, scoreUrl }
export const trip = { ...tournament, name: 'En lang turneringstittel for lasting og navigasjon' }
export const matchRound = { ...round, id: matchIds.round, name: 'Matchspill med lange navn', scoring_format: 'singles_match_play' as const }
export const matchCard = { ...matchFixture(), tournament_id: trip.id, round_id: matchRound.id }
export const matchListUrl = `/rounds/${matchRound.id}/matches`
export const matchScoreUrl = `${matchListUrl}/${matchCard.match_id}/score`
export const profileChunk = /\/assets\/ProfilePage-[^/]+\.js$/

export async function routeWorkspace(page: Page) {
  const live = await liveServer()
  const score = await mockWorkspace(page, live.url)
  const first = { ...session, display_name: 'Andreas med et langt navn fra golfklubben ved fjorden', expires_at: '2099-01-01T00:00:00Z' }
  const state = { auth: first as AuthSession | null, loginAs: first, membership: true, role: 'admin' as 'admin' | 'viewer', emptyRounds: false, profileError: false }
  const paths: string[] = [], errors: string[] = [], allowedChunks: RegExp[] = []
  page.on('request', request => { paths.push(new URL(request.url()).pathname) })
  page.on('pageerror', error => errors.push(error.message))
  page.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push(message.text()) })
  page.on('response', response => {
    const path = new URL(response.url()).pathname
    if (response.status() >= 400 && !(response.status() === 503 && (path.endsWith('/scores/conditional') || path === '/api/me/profile'))) errors.push(`HTTP ${response.status()} ${path}`)
  })
  page.on('requestfailed', request => {
    const path = new URL(request.url()).pathname, failure = request.failure()?.errorText
    if (failure !== 'net::ERR_ABORTED' && !allowedChunks.some(pattern => pattern.test(path))) errors.push(`${path} ${failure}`)
  })
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname
    const currentTrip = state.emptyRounds ? { ...trip, status: 'draft' } : trip
    if (path === '/api/auth/session') return route.fulfill({ json: state.auth })
    if (path === '/api/auth/logout') { state.auth = null; return route.fulfill({ status: 204 }) }
    if (path === '/api/auth/login') { state.auth = state.loginAs; return route.fulfill({ json: state.auth }) }
    if (path === '/api/me/profile' && state.profileError) return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Profilen er midlertidig utilgjengelig' } } })
    if (path === '/api/me/profile') return route.fulfill({ json: { ...state.auth, version: 1, handicap: 0, player_active: true, player_updated_at: trip.updated_at } })
    if (path === '/api/me/tournaments') return route.fulfill({ json: state.membership ? [{ tournament: currentTrip, role: state.role, player_id: state.auth?.player_id }] : [] })
    if (path === `/api/tournaments/${trip.id}`) return route.fulfill({ json: currentTrip })
    if (path === `/api/tournaments/${trip.id}/players`) return route.fulfill({ json: { players: [], handicap_correction: { state: 'locked', reason: 'round_opened' } } })
    if (path === `/api/tournaments/${trip.id}/rounds`) return route.fulfill({ json: state.emptyRounds ? [] : [round] })
    if (path === `/api/tournaments/${trip.id}/result-share`) return route.fulfill({ json: { tournament_id: trip.id, grant: null } })
    if (path === `/api/rounds/${matchRound.id}`) return route.fulfill({ json: matchRound })
    const readCard = Object.fromEntries(Object.entries(matchCard).filter(([key]) => !['revision', 'accepted_events'].includes(key)))
    if (path === `/api/rounds/${matchRound.id}/match-play/matches`) return route.fulfill({ json: { round_id: matchRound.id, matches: [readCard], writable_match_ids: [matchCard.match_id] } })
    if (path === `/api/rounds/${matchRound.id}/match-play/matches/${matchCard.match_id}`) return route.fulfill({ json: readCard })
    if (path === `/api/rounds/${matchRound.id}/match-play/matches/${matchCard.match_id}/scoring`) return route.fulfill({ json: matchCard })
    if (path === `/api/public/results/${shareId}`) {
      const payload = decodeObject(route.request().postDataJSON(), 'public fixture request')
      expect(payload.token).toBe(shareSecret)
      return route.fulfill({ json: publicFixture(payload.metric === 'net' ? 'net' : 'gross') })
    }
    return route.fallback()
  })
  return { state, paths, errors, allowedChunks, score, live, async close() { expect(errors).toEqual([]); await live.close() } }
}

export async function routeLayout(page: Page, name: string) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  await page.screenshot({ path: `/tmp/route-split-${name}-${page.viewportSize()?.width}.png`, fullPage: false })
}
