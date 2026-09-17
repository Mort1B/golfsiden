// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../api/client'
import { ApiHttpError } from '../api/http'
import { matchApi, type MatchTable } from '../api/matchPlay'
import { tournament, round, session } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { LeaderboardPage } from './LeaderboardPage'
import { MatchResultsPage } from './MatchResultsPage'
vi.mock('../features/live/useTournamentLive', () => ({ useTournamentLive: () => false }))
const matchTrip = { ...tournament, id: 'match-trip', name: 'Matchturnering', counted_rounds: null }
const matchRound = { ...round, id: 'match-round', tournament_id: matchTrip.id, scoring_format: 'singles_match_play' as const }
const table: MatchTable = { tournament_id: matchTrip.id, entries: ['Alice', 'Bob'].map((display_name, index) => ({ player_id: `player-${index}`, display_name, position: null, half_points: 0, played: 0, wins: 0, draws: 0, losses: 0 })) }
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
let client: QueryClient
function mount(path = '/leaderboard?tournament=match-trip&player=player-0') {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } })
  const router = createMemoryRouter([{ path: '/leaderboard', element: <LeaderboardPage /> }, { path: '/tournaments/:tournamentId/match-results', element: <MatchResultsPage /> }], { initialEntries: [path] })
  render(<QueryClientProvider client={client}><AuthContext value={auth}><RouterProvider router={router} /></AuthContext></QueryClientProvider>)
  return router
}
beforeEach(() => {
  vi.spyOn(api, 'tournaments').mockResolvedValue([matchTrip, tournament])
  vi.spyOn(api, 'rounds').mockImplementation(async id => id === matchTrip.id ? [matchRound] : [round])
  vi.spyOn(api, 'roundLeaderboard').mockResolvedValue({ tournament_id: tournament.id, round_id: round.id, status: 'draft', scoring_format: 'individual_stroke_play', metric: 'net', number_of_holes: 18, visible_hole_count: 18, visibility: { mode: 'full' }, entries: [] })
  vi.spyOn(api, 'tournamentLeaderboard')
  vi.spyOn(matchApi, 'table').mockResolvedValue(table)
  vi.spyOn(matchApi, 'list').mockResolvedValue({ round_id: matchRound.id, matches: [], writable_match_ids: [] })
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })
it('switches out of match-only results, canonicalizes stroke selection and restores player scope through history', async () => {
  const router = mount()
  const selector = await screen.findByRole('combobox', { name: 'Turnering' })
  await screen.findByText('– · Alice')
  expect(screen.queryByText('– · Bob')).toBeNull()
  expect(screen.queryByRole('group', { name: 'Beregning' })).toBeNull()
  expect(api.roundLeaderboard).not.toHaveBeenCalled()
  expect(api.tournamentLeaderboard).not.toHaveBeenCalled()
  fireEvent.change(selector, { target: { value: tournament.id } })
  await screen.findByText('Runden er fortsatt en kladd. Resultater vises når runden åpnes.')
  expect(router.state.location.search).toBe(`?tournament=${tournament.id}&scope=round&round=${round.id}&metric=net`)
  expect(api.roundLeaderboard).toHaveBeenCalledWith(round.id, tournament.id, 'net')
  await act(() => router.navigate(-1))
  await screen.findByText('– · Alice')
  expect(screen.queryByText('– · Bob')).toBeNull()
  expect(router.state.location.search).toBe('?tournament=match-trip&player=player-0')
  await act(() => router.navigate(1))
  await waitFor(() => expect(router.state.location.search).toBe(`?tournament=${tournament.id}&scope=round&round=${round.id}&metric=net`))
  fireEvent.change(screen.getByRole('combobox', { name: 'Turnering' }), { target: { value: matchTrip.id } })
  await screen.findByText('– · Bob')
  expect(router.state.location.search).toBe('?tournament=match-trip&scope=round&metric=net')
  expect(api.roundLeaderboard).toHaveBeenCalledTimes(1)
  expect(api.tournamentLeaderboard).not.toHaveBeenCalled()
})
it.each(['loading', 'error', 'empty'] as const)('keeps switching available with %s match results', async state => {
  if (state === 'loading') vi.mocked(matchApi.table).mockReturnValue(new Promise(() => undefined))
  if (state === 'error') vi.mocked(matchApi.table).mockRejectedValue(new ApiHttpError(500, 'unavailable', 'Results unavailable'))
  if (state === 'empty') vi.mocked(matchApi.table).mockResolvedValue({ ...table, entries: [] })
  mount('/leaderboard?tournament=match-trip&scope=tournament&metric=gross')
  const selector = await screen.findByRole('combobox', { name: 'Turnering' })
  if (state === 'error') await screen.findByText('Results unavailable')
  if (state === 'empty') await screen.findByText('Ingen spillere er registrert.')
  expect(api.tournamentLeaderboard).not.toHaveBeenCalled()
  expect(api.roundLeaderboard).not.toHaveBeenCalled()
  expect(selector.hasAttribute('disabled')).toBe(false)
})
it('leaves the dedicated mixed match-results route and its overall link intact', async () => {
  vi.mocked(api.rounds).mockResolvedValue([matchRound, { ...round, tournament_id: matchTrip.id }])
  mount('/tournaments/match-trip/match-results?player=player-0')
  const link = await screen.findByRole('link', { name: 'Sammenlagt brutto/netto' })
  expect(link.getAttribute('href')).toBe('/leaderboard?tournament=match-trip&scope=tournament&metric=net')
  expect(screen.queryByRole('combobox', { name: 'Turnering' })).toBeNull()
  expect(screen.queryByText('– · Bob')).toBeNull()
})
