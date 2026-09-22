// @vitest-environment jsdom
import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../api/client'
import { ApiHttpError } from '../api/http'
import { matchApi, matchKeys, type MatchTable, type MatchListing } from '../api/matchPlay'
import { matchFixture, matchIds } from '../api/matchPlay/fixtures'
import * as invalidation from '../api/liveInvalidation'
import { privateWorkspaceKeys } from '../api/privateWorkspace'
import { tournament, round, session } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { publishSessionTransition } from '../features/auth/sessionTransition'
import type { AuthSession } from '../api/auth'
import { MatchResultsPage } from './MatchResultsPage'
import { PlayerHistoryPage } from './PlayerHistoryPage'
import { LeaderboardPage } from './LeaderboardPage'

// The hook, shared subscription, invalidation and query lifecycle are real.
class Source extends EventTarget {
  static instances: Source[] = []
  readyState = 0
  close = vi.fn(() => { this.readyState = 2 })
  constructor(readonly url: string) { super(); Source.instances.push(this) }
  emit(signal: string) {
    if (signal === 'open') this.readyState = 1
    if (signal === 'error') this.readyState = 0
    this.dispatchEvent(new Event(signal))
  }
}
const routes = ['direct', 'filtered', 'history', 'global'] as const
type Entry = typeof routes[number]
const trip = { ...tournament, id: matchIds.tournament, counted_rounds: null }
const nextTrip = { ...trip, id: 'next-trip', name: 'Neste turnering' }
const matchRound = { ...round, id: matchIds.round, tournament_id: trip.id, status: 'open' as const, scoring_format: 'singles_match_play' as const }
const table = (id = trip.id, name = 'Authorized player'): MatchTable => ({ tournament_id: id, entries: [{ player_id: matchIds.first, display_name: name, position: null, half_points: 0, played: 0, wins: 0, draws: 0, losses: 0 }] })
function path(entry: Entry, id = trip.id) {
  if (entry === 'global') return `/leaderboard?tournament=${id}&scope=tournament&metric=net`
  if (entry === 'history') return `/tournaments/${id}/results/players/${matchIds.first}?metric=net`
  return `/tournaments/${id}/match-results${entry === 'filtered' ? `?player=${matchIds.first}` : ''}`
}
function deferred<T>() {
  let resolve: (value: T) => void = () => undefined
  const promise = new Promise<T>(done => { resolve = done })
  return { promise, resolve }
}
let client: QueryClient
function mount(entry: Entry) {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } })
  const router = createMemoryRouter([
    { path: '/tournaments/:tournamentId/match-results', element: <MatchResultsPage /> },
    { path: '/tournaments/:tournamentId/results/players/:playerId', element: <PlayerHistoryPage /> },
    { path: '/leaderboard', element: <LeaderboardPage /> },
  ], { initialEntries: [path(entry)] })
  const tree = (value: AuthSession | null) => {
    const auth: AuthContextValue = { session: value, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
    return <QueryClientProvider client={client}><AuthContext value={auth}>{value ? <RouterProvider router={router} /> : <p>Signed out</p>}</AuthContext></QueryClientProvider>
  }
  publishSessionTransition(client, session)
  const view = render(tree(session))
  return { router, changeAccount(value: AuthSession | null) { publishSessionTransition(client, value); view.rerender(tree(value)) } }
}
async function currentSource() {
  await waitFor(() => expect(Source.instances.length).toBeGreaterThan(0))
  const source = Source.instances.at(-1)
  if (!source) throw new Error('Missing source')
  return source
}
async function settled() { await waitFor(() => expect(client.isFetching()).toBe(0)) }
async function populated() { await screen.findByText('– · Authorized player'); await screen.findByRole('heading', { name: /Andreas.*Bjørn/ }); await settled() }
beforeEach(() => {
  Source.instances = []
  vi.stubGlobal('EventSource', Source)
  vi.spyOn(window, 'scrollTo').mockImplementation(() => undefined)
  vi.spyOn(invalidation, 'handleTournamentLiveSignal')
  vi.spyOn(api, 'tournaments').mockResolvedValue([trip, nextTrip])
  vi.spyOn(api, 'rounds').mockImplementation(async id => [{ ...matchRound, tournament_id: id }])
  vi.spyOn(matchApi, 'table').mockImplementation(async id => table(id))
  vi.spyOn(matchApi, 'list').mockResolvedValue({ round_id: matchRound.id, matches: [matchFixture()], writable_match_ids: [] })
})
afterEach(async () => { cleanup(); client?.clear(); await Promise.resolve(); vi.restoreAllMocks(); vi.unstubAllGlobals() })

for (const entry of routes) {
  it.each(['early', 'settled'] as const)(`${entry} keeps one owner through %s open, refresh, clearing and remount`, async timing => {
    const held = deferred<MatchTable>()
    if (timing === 'early') vi.mocked(matchApi.table).mockReturnValueOnce(held.promise)
    mount(entry)
    const source = await currentSource()
    if (timing === 'settled') await populated()
    vi.mocked(invalidation.handleTournamentLiveSignal).mockClear()
    await act(async () => { source.emit('open'); held.resolve(table()) })
    await populated()
    expect(invalidation.handleTournamentLiveSignal).toHaveBeenCalledExactlyOnceWith(client, session.user_id, 'open')
    vi.mocked(matchApi.table).mockClear(); vi.mocked(matchApi.list).mockClear()
    vi.mocked(invalidation.handleTournamentLiveSignal).mockClear()
    const matchTable = deferred<MatchTable>(), matchList = deferred<MatchListing>()
    vi.mocked(matchApi.table).mockReturnValueOnce(matchTable.promise)
    vi.mocked(matchApi.list).mockReturnValueOnce(matchList.promise)
    await act(async () => { source.emit('match') })
    expect(matchApi.table).toHaveBeenCalledTimes(1); expect(matchApi.list).toHaveBeenCalledTimes(1)
    expect(invalidation.handleTournamentLiveSignal).toHaveBeenCalledExactlyOnceWith(client, session.user_id, 'match')
    await act(async () => { matchTable.resolve(table()); matchList.resolve({ round_id: matchRound.id, matches: [matchFixture()], writable_match_ids: [] }) }); await populated()
    const refreshing = deferred<MatchTable>()
    vi.mocked(matchApi.table).mockReturnValueOnce(refreshing.promise)
    await act(async () => {
      source.emit('visibility')
      expect(client.getQueryData(matchKeys.table(session.user_id, trip.id))).toBeUndefined()
    })
    await waitFor(() => expect(screen.queryByRole('heading', { name: /Andreas.*Bjørn/ })).toBeNull())
    await act(async () => { refreshing.resolve(table()) }); await populated()
    await act(async () => { source.emit('error') })
    expect(client.getQueryData(matchKeys.table(session.user_id, trip.id))).toBeUndefined()
    expect(client.getQueryData(matchKeys.list(session.user_id, matchRound.id))).toBeUndefined()
    await act(async () => { source.emit('open') }); await populated()
    expect(Source.instances).toHaveLength(1); expect(source.close).not.toHaveBeenCalled()
  })

  it(`${entry} retains live recovery through error and empty child states`, async () => {
    vi.mocked(matchApi.table).mockRejectedValueOnce(new ApiHttpError(503, 'unavailable', 'Results unavailable'))
    mount(entry); const source = await currentSource()
    await screen.findByText('Results unavailable'); await settled()
    vi.mocked(matchApi.table).mockResolvedValueOnce({ ...table(), entries: [] })
    vi.mocked(matchApi.list).mockResolvedValueOnce({ round_id: matchRound.id, matches: [], writable_match_ids: [] })
    await act(async () => { source.emit('open') }); await settled()
    await screen.findByText('Ingen spillere er registrert.')
    await act(async () => { source.emit('match') }); await populated()
    expect(Source.instances).toHaveLength(1)
  })

  it.each([401, 403, 404])(`${entry} erases sibling projections on a fresh event-driven %s denial`, async status => {
    mount(entry); const source = await currentSource(); await populated()
    vi.mocked(matchApi.table).mockRejectedValue(new ApiHttpError(status, 'denied', 'Access denied'))
    await act(async () => { source.emit('match') }); await settled()
    await waitFor(() => expect(screen.queryByText('– · Authorized player')).toBeNull())
    expect(client.getQueryData(matchKeys.table(session.user_id, trip.id))).toBeUndefined()
    expect(client.getQueryData(matchKeys.list(session.user_id, matchRound.id))).toBeUndefined()
    expect(Source.instances).toHaveLength(1)
  })

  it(`${entry} transfers ownership on tournament change and logout/new account`, async () => {
    const view = mount(entry); const first = await currentSource(); await populated()
    await act(() => view.router.navigate(path(entry, nextTrip.id))); await populated()
    const second = await currentSource(); expect(second).not.toBe(first)
    expect(first.close).toHaveBeenCalledOnce()
    vi.mocked(invalidation.handleTournamentLiveSignal).mockClear()
    await act(async () => { first.emit('match'); second.emit('match') }); await settled()
    expect(invalidation.handleTournamentLiveSignal).toHaveBeenCalledExactlyOnceWith(client, session.user_id, 'match')
    await act(async () => { view.changeAccount(null) })
    expect(screen.getByText('Signed out')).toBeTruthy(); expect(second.close).toHaveBeenCalledOnce()
    expect(client.getQueryCache().findAll({ queryKey: privateWorkspaceKeys.user(session.user_id) })).toHaveLength(0)
    const nextSession = { ...session, user_id: 'replacement-user' }
    vi.mocked(matchApi.table).mockImplementation(async id => table(id, 'Replacement player'))
    await act(async () => { view.changeAccount(nextSession) }); await screen.findByText('– · Replacement player'); await settled()
    const third = await currentSource(); expect(third).not.toBe(second)
    vi.mocked(invalidation.handleTournamentLiveSignal).mockClear()
    await act(async () => { first.emit('visibility'); second.emit('error'); third.emit('match') }); await settled()
    expect(invalidation.handleTournamentLiveSignal).toHaveBeenCalledExactlyOnceWith(client, nextSession.user_id, 'match')
    expect(screen.queryByText('– · Authorized player')).toBeNull()
  })
}
