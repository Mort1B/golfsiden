// @vitest-environment jsdom
import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../api/client'
import { ApiHttpError } from '../api/http'
import { matchApi, matchKeys, type MatchPlayerListing, type MatchTable } from '../api/matchPlay'
import { matchFixture, matchIds } from '../api/matchPlay/fixtures'
import { tournamentKeys } from '../api/tournaments'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { publishSessionTransition } from '../features/auth/sessionTransition'
import { tournament, round, session } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { MatchRound } from '../features/matchPlay/MatchRound'
import { MatchResultsPage } from './MatchResultsPage'
import { PlayerHistoryPage } from './PlayerHistoryPage'
import { LeaderboardPage } from './LeaderboardPage'

class Source extends EventTarget {
  static current: Source | undefined
  readyState = 1
  constructor() { super(); Source.current = this }
  close() { this.readyState = 2 }
  emit(type: string) { this.dispatchEvent(new Event(type)) }
}
const trip = { ...tournament, id: matchIds.tournament, counted_rounds: null }
const matchRound = { ...round, id: matchIds.round, tournament_id: trip.id, status: 'open' as const, scoring_format: 'singles_match_play' as const }
const table: MatchTable = { tournament_id: trip.id, entries: [{ player_id: matchIds.first, display_name: 'Private table name', position: null, half_points: 0, played: 0, wins: 0, draws: 0, losses: 0 }] }
const listing: MatchPlayerListing = { round_id: matchRound.id, player_id: matchIds.first, matches: [matchFixture()], writable_match_ids: [matchIds.match] }
const paths = [
  `/tournaments/${trip.id}/match-results`,
  `/tournaments/${trip.id}/results/players/${matchIds.first}?metric=net`,
  `/leaderboard?tournament=${trip.id}&scope=tournament&metric=net`,
]
function deferred<T>() {
  let resolve: (value: T) => void = () => undefined, reject: (reason: Error) => void = () => undefined
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
let client: QueryClient
function mount(path = paths[0] ?? '') {
  client = new QueryClient({ defaultOptions: { queries: { staleTime: 20_000, retry: false } } })
  const router = createMemoryRouter([
    { path: '/tournaments/:tournamentId/match-results', element: <MatchResultsPage /> },
    { path: '/tournaments/:tournamentId/results/players/:playerId', element: <PlayerHistoryPage /> },
    { path: '/leaderboard', element: <LeaderboardPage /> },
    { path: '/round-list', element: <MatchRound roundId={matchRound.id} /> },
  ], { initialEntries: [path] })
  const tree = (user = session) => {
    const auth: AuthContextValue = { session: user, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
    return <QueryClientProvider client={client}><AuthContext value={auth}><RouterProvider router={router} /></AuthContext></QueryClientProvider>
  }
  publishSessionTransition(client, session)
  const view = render(tree())
  return { router, replaceAccount() { const next = { ...session, user_id: 'replacement-user' }; publishSessionTransition(client, next); view.rerender(tree(next)) } }
}
async function emit(type: string) { await act(async () => { if (!Source.current) throw new Error('No source'); Source.current.emit(type) }) }
async function populated() { await screen.findByRole('heading', { name: /Andreas.*Bjørn/ }); await waitFor(() => expect(client.isFetching()).toBe(0)) }
function noPrivateDom() {
  expect(document.querySelectorAll('.match-list, .match-table, .match-actions')).toHaveLength(0)
  expect(screen.queryByText(matchRound.name)).toBeNull()
  expect(screen.queryByText(/Private table name/)).toBeNull()
  expect(screen.queryByText(/Andreas/)).toBeNull()
}
beforeEach(() => {
  Source.current = undefined; vi.stubGlobal('EventSource', Source)
  vi.spyOn(window, 'scrollTo').mockImplementation(() => undefined)
  vi.spyOn(api, 'tournaments').mockResolvedValue([trip])
  vi.spyOn(api, 'rounds').mockResolvedValue([matchRound])
  vi.spyOn(matchApi, 'table').mockResolvedValue(table)
  vi.spyOn(matchApi, 'list').mockResolvedValue(listing)
  vi.spyOn(matchApi, 'listForPlayer').mockResolvedValue(listing)
})
afterEach(async () => { cleanup(); client?.clear(); await Promise.resolve(); vi.restoreAllMocks(); vi.unstubAllGlobals() })

function read(path: string) { return path.includes('/results/players/') ? vi.mocked(matchApi.listForPlayer) : vi.mocked(matchApi.list) }
function listKey(path: string) { return path.includes('/results/players/') ? matchKeys.listForPlayer(session.user_id, matchRound.id, matchIds.first) : matchKeys.list(session.user_id, matchRound.id) }

for (const path of paths) {
  it(`does not fetch or expose lists before initial table readiness: ${path}`, async () => {
    const pending = deferred<MatchTable>(); vi.mocked(matchApi.table).mockReturnValue(pending.promise)
    mount(path)
    await waitFor(() => expect(client.getQueryData(tournamentKeys.rounds(session.user_id, trip.id))).toBeDefined())
    await waitFor(() => expect(client.getQueryCache().find({ queryKey: listKey(path), exact: true })?.getObserversCount()).toBe(1))
    expect(client.getQueryCache().find({ queryKey: listKey(path), exact: true })?.isActive()).toBe(false)
    expect(read(path)).not.toHaveBeenCalled(); noPrivateDom()
    await act(async () => { pending.resolve(table) }); await populated()
    expect(read(path)).toHaveBeenCalledOnce()
  })

  it.each(['table-first', 'list-first', 'stale-list-first'] as const)(`retains refresh with %s response ordering: ${path}`, async order => {
    mount(path); await populated()
    const nextTable = deferred<MatchTable>(), nextList = deferred<MatchPlayerListing>()
    let signal: AbortSignal | undefined
    vi.mocked(matchApi.table).mockReturnValueOnce(nextTable.promise)
    read(path).mockClear().mockImplementationOnce((_id: string, value?: string | AbortSignal, playerSignal?: AbortSignal) => { signal = typeof value === 'string' ? playerSignal : value; return nextList.promise })
    await emit('visibility'); await waitFor(noPrivateDom)
    const query = client.getQueryCache().find({ queryKey: listKey(path), exact: true })
    expect(query?.getObserversCount()).toBe(1); expect(signal?.aborted).toBe(false)
    expect(read(path)).toHaveBeenCalledOnce()
    if (order === 'table-first') {
      await act(async () => { nextTable.resolve(table) })
      expect(screen.queryByRole('heading', { name: /Andreas.*Bjørn/ })).toBeNull()
      await act(async () => { nextList.resolve(listing) })
    } else {
      await act(async () => { nextList.resolve(listing) }); noPrivateDom()
      if (order === 'stale-list-first') vi.spyOn(Date, 'now').mockReturnValue(Date.now() + 21_000)
      await act(async () => { nextTable.resolve(table) })
    }
    await populated()
    expect(signal?.aborted).toBe(false)
    expect(read(path)).toHaveBeenCalledTimes(order === 'stale-list-first' ? 2 : 1)
  })

  it(`only presents restricted-final data after the parent reopens: ${path}`, async () => {
    const full = { ...matchFixture(), finish: { type: 'draw' as const }, resolved_holes: 18, confirmed: true, half_points: [1, 1] as [number, number] }
    read(path).mockResolvedValue({ ...listing, matches: [full] })
    mount(path); await screen.findByText('Bekreftet resultat')
    const pending = deferred<MatchTable>()
    vi.mocked(matchApi.table).mockReturnValueOnce(pending.promise)
    const restricted = { ...full, holes: full.holes.slice(0, 9), visibility: { mode: 'front_nine' as const }, finish: null, confirmed: null, correction_pending: null, half_points: null, resolved_holes: 0 }
    read(path).mockResolvedValue({ ...listing, matches: [restricted], writable_match_ids: [] })
    await emit('visibility'); await waitFor(noPrivateDom)
    await waitFor(() => expect(client.getQueryData(listKey(path))).toEqual({ ...listing, matches: [restricted], writable_match_ids: [] }))
    noPrivateDom()
    await act(async () => { pending.resolve(table) })
    await screen.findByText('Fullføring og poeng er skjult til finalen frigis.')
    expect(screen.queryByText('Bekreftet resultat')).toBeNull()
    expect(screen.queryByRole('link', { name: 'Før match' })).toBeNull()
  })
}

for (const path of paths.slice(0, 2)) {
for (const dependency of ['rounds', 'table', 'list'] as const) it.each([401, 403, 404])(`fails closed on fresh ${dependency} %s while gated: ${path}`, async status => {
  mount(path); await populated()
  const pending = deferred<MatchTable>()
  vi.mocked(matchApi.table).mockReturnValueOnce(pending.promise)
  const error = new ApiHttpError(status, 'denied', 'Access denied')
  if (dependency === 'rounds') vi.mocked(api.rounds).mockRejectedValueOnce(error)
  if (dependency === 'table') pending.reject(error)
  if (dependency === 'list') read(path).mockRejectedValueOnce(error)
  await emit('visibility')
  await screen.findByText('Access denied'); noPrivateDom()
  expect(client.getQueryData(listKey(path))).toBeUndefined()
  if (dependency !== 'table') { await act(async () => { pending.resolve(table) }); noPrivateDom() }
})

it.each(['success', 'denial'] as const)(`ignores late gated %s after another visibility refresh: ${path}`, async outcome => {
  mount(path); await populated()
  const oldTable = deferred<MatchTable>(), oldList = deferred<MatchPlayerListing>()
  let oldSignal: AbortSignal | undefined
  vi.mocked(matchApi.table).mockReturnValueOnce(oldTable.promise)
  read(path).mockImplementationOnce((_id: string, value?: string | AbortSignal, playerSignal?: AbortSignal) => { oldSignal = typeof value === 'string' ? playerSignal : value; return oldList.promise })
  await emit('visibility'); await waitFor(noPrivateDom)
  await emit('visibility'); await populated()
  expect(oldSignal?.aborted).toBe(true)
  await act(async () => {
    oldTable.resolve(table)
    if (outcome === 'success') oldList.resolve({ ...listing, matches: [] })
    else oldList.reject(new ApiHttpError(403, 'denied', 'Late denial'))
  })
  await populated(); expect(screen.queryByText('Late denial')).toBeNull()
})

it.each(['round', 'account', 'tournament'] as const)(`removes gated ownership on %s change: ${path}`, async change => {
  const view = mount(path); await populated()
  const oldList = deferred<MatchPlayerListing>(), oldTable = deferred<MatchTable>()
  let signal: AbortSignal | undefined
  vi.mocked(matchApi.table).mockReturnValueOnce(oldTable.promise)
  read(path).mockImplementationOnce((_id: string, value?: string | AbortSignal, playerSignal?: AbortSignal) => { signal = typeof value === 'string' ? playerSignal : value; return oldList.promise })
  await emit('visibility'); await waitFor(noPrivateDom)
  if (change === 'round') await act(async () => { client.setQueryData(tournamentKeys.rounds(session.user_id, trip.id), []) })
  else if (change === 'account') await act(async () => { view.replaceAccount() })
  else await act(() => view.router.navigate('/tournaments/replacement-trip/match-results'))
  await waitFor(() => expect(signal?.aborted).toBe(true))
  const oldQuery = client.getQueryCache().find({ queryKey: listKey(path), exact: true })
  if (change !== 'tournament') expect(oldQuery?.getObserversCount() ?? 0).toBe(0)
  await act(async () => { oldTable.resolve(table); oldList.resolve({ ...listing, matches: [] }) })
  expect(screen.queryByText('Ingen matcher er satt opp for dette valget.')).toBeNull()
})

it(`removes retained owners when the table fails with an ordinary error: ${path}`, async () => {
  mount(path); await populated()
  const pending = deferred<MatchPlayerListing>(); let signal: AbortSignal | undefined
  read(path).mockImplementationOnce((_id: string, value?: string | AbortSignal, playerSignal?: AbortSignal) => { signal = typeof value === 'string' ? playerSignal : value; return pending.promise })
  vi.mocked(matchApi.table).mockRejectedValueOnce(new ApiHttpError(503, 'unavailable', 'Table unavailable'))
  await emit('visibility'); await screen.findByText('Table unavailable'); noPrivateDom()
  await waitFor(() => expect(signal?.aborted).toBe(true))
  await act(async () => { pending.resolve(listing) }); noPrivateDom()
})

}

it('keeps standalone MatchRound callers enabled and visible by default', async () => {
  mount('/round-list'); await populated()
  expect(matchApi.table).not.toHaveBeenCalled(); expect(matchApi.list).toHaveBeenCalledOnce()
})
