// @vitest-environment jsdom
import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider, useSearchParams } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { matchApi, matchKeys, type MatchCard, type MatchListing, type MatchPlayerListing } from '../api/matchPlay'
import { ApiHttpError } from '../api/http'
import { matchFixture, matchIds } from '../api/matchPlay/fixtures'
import { MatchRound } from '../features/matchPlay/MatchRound'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { publishSessionTransition } from '../features/auth/sessionTransition'
import { session } from '../features/tournaments/lifecycle/__tests__/fixtures'
const first = '00000000-0000-0000-0000-00000000000a', third = '00000000-0000-0000-0000-00000000000c'
const firstCard: MatchCard = { ...matchFixture(), opponents: [{ ...matchFixture().opponents[0], player_id: first }, matchFixture().opponents[1]] }
const otherCard: MatchCard = { ...matchFixture(), match_id: matchIds.user, opponents: [{ player_id: third, display_name: 'Caroline', playing_handicap: 0 }, { player_id: '00000000-0000-0000-0000-00000000000d', display_name: 'Daniel', playing_handicap: 0 }] }
const full: MatchListing = { round_id: matchIds.round, matches: [firstCard, otherCard], writable_match_ids: [matchIds.match] }
function selected(player: string): MatchPlayerListing {
  const matches = full.matches.filter(m => m.opponents.some(p => p.player_id === player))
  return { ...full, player_id: player, matches, writable_match_ids: full.writable_match_ids.filter(id => matches.some(m => m.match_id === id)) }
}
let client: QueryClient
function View() { const [search] = useSearchParams(); return <MatchRound roundId={matchIds.round} playerId={search.get('player') ?? undefined} /> }
function mount(path = '/') {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: 20_000 } } })
  const router = createMemoryRouter([{ path: '/', element: <View /> }], { initialEntries: [path] })
  const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
  publishSessionTransition(client, session)
  render(<QueryClientProvider client={client}><AuthContext value={auth}><RouterProvider router={router} /></AuthContext></QueryClientProvider>)
  return router
}
async function settled() { await waitFor(() => expect(client.isFetching()).toBe(0)) }
beforeEach(() => {
  vi.spyOn(matchApi, 'list').mockResolvedValue(full)
  vi.spyOn(matchApi, 'listForPlayer').mockImplementation(async (_round, player) => selected(player))
})
afterEach(() => { cleanup(); client?.clear(); vi.restoreAllMocks() })
it('isolates player variants from the management list and reuses fresh exact keys on return', async () => {
  const router = mount()
  await screen.findByRole('heading', { name: 'Caroline mot Daniel' }); await settled()
  expect(document.querySelectorAll('.match-list article')).toHaveLength(2)
  await act(() => router.navigate(`/?player=${first}`)); await settled()
  expect(document.querySelectorAll('.match-list article')).toHaveLength(1)
  expect(screen.queryByRole('heading', { name: 'Caroline mot Daniel' })).toBeNull()
  expect(client.getQueryData(matchKeys.list(session.user_id, matchIds.round))).toEqual(full)
  expect(client.getQueryData(matchKeys.listForPlayer(session.user_id, matchIds.round, first))).toEqual(selected(first))
  // Management's exact unfiltered key still contains both pairs without a fetch.
  expect(await client.fetchQuery({ queryKey: matchKeys.list(session.user_id, matchIds.round), queryFn: () => matchApi.list(matchIds.round) })).toEqual(full)
  await act(() => router.navigate(`/?player=${third}`)); await screen.findByRole('heading', { name: 'Caroline mot Daniel' }); await settled()
  expect(screen.queryByRole('link', { name: 'Før match' })).toBeNull()
  await act(() => router.navigate('/')); await settled()
  expect(document.querySelectorAll('.match-list article')).toHaveLength(2)
  await act(() => router.navigate(`/?player=${first}`)); await settled()
  expect(matchApi.list).toHaveBeenCalledOnce(); expect(matchApi.listForPlayer).toHaveBeenCalledTimes(2)
})
it.each(['', 'invalid', first.toUpperCase(), first.replaceAll('-', '')])('preserves legacy URL filtering for %s', async player => {
  mount(`/?player=${encodeURIComponent(player)}`); await settled()
  if (!player) expect(document.querySelectorAll('.match-list article')).toHaveLength(2)
  else await screen.findByText('Ingen matcher er satt opp for dette valget.')
  expect(matchApi.list).toHaveBeenCalledOnce(); expect(matchApi.listForPlayer).not.toHaveBeenCalled()
})
it('keeps an absent valid player empty through the filtered endpoint', async () => {
  mount(`/?player=${matchIds.tournament}`)
  await screen.findByText('Ingen matcher er satt opp for dette valget.')
  expect(matchApi.list).not.toHaveBeenCalled(); expect(matchApi.listForPlayer).toHaveBeenCalledOnce()
})
it.each(['success', 'denial'] as const)('does not publish a previous player late %s after switching', async outcome => {
  let resolve: (value: MatchPlayerListing) => void = () => undefined, reject: (error: Error) => void = () => undefined, signal: AbortSignal | undefined
  const pending = new Promise<MatchPlayerListing>((yes, no) => { resolve = yes; reject = no })
  vi.mocked(matchApi.listForPlayer).mockImplementationOnce((_round, _player, value) => { signal = value; return pending })
  const router = mount(`/?player=${first}`)
  await waitFor(() => expect(matchApi.listForPlayer).toHaveBeenCalledOnce())
  await act(() => router.navigate(`/?player=${third}`))
  await screen.findByRole('heading', { name: 'Caroline mot Daniel' }); await settled()
  expect(signal?.aborted).toBe(true)
  await act(async () => { if (outcome === 'success') resolve(selected(first)); else reject(new ApiHttpError(403, 'denied', 'Old player denied')) })
  expect(screen.queryByText('Old player denied')).toBeNull()
  expect(screen.queryByRole('heading', { name: /Andreas/ })).toBeNull()
  expect(client.getQueryData(matchKeys.listForPlayer(session.user_id, matchIds.round, third))).toEqual(selected(third))
})
