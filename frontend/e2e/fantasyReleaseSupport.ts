import { expect, type Browser, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeArray, decodeObject, decodeUuid } from '../src/api/decoder'
import { decodeClaimReceipt } from '../src/api/playerClaimDecoders'
import { decodeRound, decodeTournament, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { results, roundResult } from '../src/api/fantasy/results'
import type { ScoringFormat } from '../src/api/types'

export function at<T>(items: readonly T[], index: number): T {
  const item = items[index]
  if (item === undefined) throw new Error(`Missing fixture index ${index}`)
  return item
}
export async function releaseFixture(page: Page, formats: ScoringFormat[], handicaps: number[]) {
  const stamp = `${Date.now()}_${Math.floor(Math.random() * 10000)}`
  const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
  const names = handicaps.map((_, i) => `Spiller ${String(i + 1).padStart(2, '0')} med langt navn i Fantasy-akseptansen`)
  const created = await page.request.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username: `release_${stamp}`, password: 'local-release-fixture-password' }, player: { display_name: at(names, 0), handicap_index: at(handicaps, 0) } },
    tournament: { name: 'Fantasy gjennom hele turneringen', description: '', start_date: day, end_date: day, counted_rounds: 1, mandatory_round_number: null },
    rounds: formats.map((scoring_format, i) => ({ round_number: i + 1, name: `Akseptrunde ${i + 1}`, round_date: day, scoring_format })),
  } })
  expect(created.status()).toBe(201)
  const body = decodeObject(await created.json(), 'created'), auth = decodeAuthSession(body.session), tournament = decodeTournament(body.tournament)
  if (!auth.player_id) throw new Error('Missing creator player')
  const players = [auth.player_id]
  const mutate = async (path: string, data: unknown = {}, method = 'POST') => {
    const response = await page.request.fetch(path, { method, data, headers: { 'x-csrf-token': auth.csrf_token } })
    expect(response.ok(), `${path}: ${response.status()}`).toBe(true)
    return response
  }
  for (let i = 1; i < handicaps.length; i++) {
    const player = decodeObject(await (await mutate(`/api/tournaments/${tournament.id}/players`, { display_name: at(names, i), handicap_index: at(handicaps, i) })).json(), 'player')
    players.push(decodeUuid(player.player_id, 'player_id'))
  }
  const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${tournament.id}/rounds`)).json(), tournament.id)
  const base = `/api/tournaments/${tournament.id}/fantasy`
  await mutate(base, { enabled: true }, 'PUT')
  await mutate(`${base}/entry`)
  async function configure(index: number, order: number[], teams: boolean, par = 4) {
    const r = at(rounds, index)
    const configured = decodeRound(await (await mutate(`/api/rounds/${r.id}/course-configuration`, {
      expected_round_updated_at: r.updated_at,
      selection: { source: 'manual', course_name: 'Fast testbane', location: null, tee: { category: 'male', name: 'Gul', course_rating: par * 18, slope_rating: 113,
        holes: Array.from({ length: 18 }, (_, i) => ({ par, stroke_index: i + 1, distance: null })) } },
    }, 'PUT')).json())
    const members = order.map(i => ({ player_id: at(players, i) }))
    const pairs = teams ? Array.from({ length: order.length / 2 }, (_, i) => ({ id: crypto.randomUUID(), name: `Lag ${i + 1}`, members: [at(members, i * 2), at(members, i * 2 + 1)], schedule_flight_id: null })) : []
    const flights = Array.from({ length: Math.ceil(members.length / 4) }, (_, i) => ({ id: crypto.randomUUID(), name: `Flight ${i + 1}`, starting_hole: 1, tee_time: null, members: members.slice(i * 4, i * 4 + 4) }))
    await mutate(`/api/rounds/${r.id}/pairings`, { expected_round_updated_at: configured.updated_at, teams: pairs, flights, legacy_conversions: [] }, 'PUT')
    return pairs
  }
  async function start() {
    const fresh = decodeTournament(await (await page.request.get(`/api/tournaments/${tournament.id}`)).json())
    await mutate(`/api/tournaments/${tournament.id}/start`, { expected_tournament_updated_at: fresh.updated_at })
  }
  const board = async (index: number) => {
    const r = at(rounds, index)
    return roundResult(await (await page.request.get(`${base}/rounds/${r.id}/results`)).json(), tournament.id, r.id)
  }
  const totals = async () => results(await (await page.request.get(`${base}/results`)).json(), tournament.id)
  return { page, auth, tournament, rounds, players, names, base, mutate, configure, start, board, totals, url: `/tournaments/${tournament.id}/fantasy` }
}
export type ReleaseFixture = Awaited<ReturnType<typeof releaseFixture>>
export async function claimManager(f: ReleaseFixture, browser: Browser, playerIndex: number) {
  const receipt = decodeClaimReceipt(await (await f.mutate(`/api/tournaments/${f.tournament.id}/players/${at(f.players, playerIndex)}/claim`)).json())
  const context = await browser.newContext({ baseURL: 'http://127.0.0.1:5173' }), page = await context.newPage()
  const response = await page.request.post(`/api/player-claims/${receipt.claim_id}/register`, { data: { token: receipt.token, account: { username: `release_manager_${crypto.randomUUID().replaceAll('-', '').slice(0, 12)}`, password: 'local-release-manager-password' } } })
  expect(response.status()).toBe(201)
  const auth = decodeAuthSession(decodeObject(await response.json(), 'claim').session)
  expect((await page.request.post(`${f.base}/entry`, { headers: { 'x-csrf-token': auth.csrf_token } })).status()).toBe(204)
  return { context, page, auth }
}
export async function selectFour(f: ReleaseFixture, roundIndex: number, picks: number[], captain: number) {
  await f.page.goto(f.url)
  await f.page.getByLabel('Runde for Min firer').selectOption(at(f.rounds, roundIndex).id)
  await expect(f.page.getByRole('checkbox', { name: at(f.names, 0), exact: true })).toBeVisible()
  for (let i = 0; i < f.names.length; i++) if (!picks.includes(i)) await f.page.getByRole('checkbox', { name: at(f.names, i), exact: true }).uncheck()
  for (const i of picks) await f.page.getByRole('checkbox', { name: at(f.names, i), exact: true }).check()
  await f.page.getByLabel('Kaptein · doble poeng').selectOption(at(f.players, captain))
  await f.page.getByRole('button', { name: 'Lagre firer og kaptein' }).click()
  await expect(f.page.getByText('Lagret lag · revisjon 1')).toBeVisible()
  await f.page.goto('about:blank')
}
export async function scoreTeam(f: ReleaseFixture, roundIndex: number, team: string, firstGross: number) {
  const round = at(f.rounds, roundIndex), path = `/api/rounds/${round.id}/scorecards/team/${team}`
  const card = decodeObject(await (await f.page.request.get(`${path}/scoring`)).json(), 'card')
  const holes = decodeArray(card.holes, 'holes', (v, p) => decodeUuid(decodeObject(v, p).hole_id, `${p}.hole_id`))
  for (const [i, hole_id] of holes.entries()) await f.mutate(`/api/rounds/${round.id}/scores`, { owner: { type: 'team', id: team }, hole_id, gross_strokes: i === 0 ? firstGross : 4 }, 'PUT')
  await f.mutate(`${path}/confirm`)
}
export function browserEvidence(page: Page) {
  const errors: string[] = [], failures: string[] = [], responses: Promise<void>[] = [], conflicts: number[] = []
  page.on('pageerror', e => errors.push(e.message))
  page.on('console', m => { if (m.type() === 'error' && !m.text().includes('Failed to load resource')) errors.push(m.text()) })
  page.on('requestfailed', r => { if (!r.url().includes('/live') && !r.failure()?.errorText.includes('ERR_ABORTED')) failures.push(new URL(r.url()).pathname) })
  page.on('response', r => { if (r.status() >= 400) responses.push((async () => {
    const body = decodeObject(await r.json(), 'error'), error = decodeObject(body.error, 'error.error')
    expect(r.status()).toBe(409); expect(error.code).toBe('fantasy_conflict'); conflicts.push(r.status())
  })()) })
  return async () => { await Promise.all(responses); expect(errors).toEqual([]); expect(failures).toEqual([]); console.log(`Fantasy release browser: no console or failed requests; ${conflicts.length} retryable projection conflicts.`) }
}
