// @vitest-environment jsdom
import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../api/client'
import { ApiHttpError } from '../api/http'
import { scoringKeys, type ReadScorecard } from '../api/scorecards'
import { leaderboardKeys } from '../api/leaderboards'
import { tournamentKeys } from '../api/tournaments'
import type { RoundLeaderboard } from '../api/types'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { tournament, round as draft, session } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { DirectScorecardPage } from './DirectScorecardPage'
import { PlayerHistoryPage } from './PlayerHistoryPage'
import { MatchPage } from './MatchPage'
import { matchApi, matchKeys } from '../api/matchPlay'
import { matchFixture, matchIds } from '../api/matchPlay/fixtures'
import { tieBoard, tieRounds, tieTournamentId } from '../api/leaderboards/__tests__/tieBreakFixtures'
import { LeaderboardPage } from './LeaderboardPage'
vi.mock('../features/live/useTournamentLive', () => ({ useTournamentLive: () => false }))
const round = { ...draft, status: 'open' as const }
const owner = { type: 'player' as const, id: session.player_id ?? '' }
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
const board: RoundLeaderboard = { tournament_id: tournament.id, round_id: round.id, status: 'open', scoring_format: 'individual_stroke_play', metric: 'gross', number_of_holes: 18, visible_hole_count: 18, visibility: { mode: 'full' }, entries: [{ position: 1, tied: false, owner, owner_name: 'Private Player', members: [], holes_scored: 1, number_of_holes: 18, complete: false, confirmed: false, playing_handicap: 0, gross_total: 7, net_total: 7, par_played: 4, score_to_par: 3 }] }
const card: ReadScorecard = { projection: 'read', round_id: round.id, owner, number_of_holes: 18, visible_hole_count: 18, visibility: { mode: 'full' }, complete: false, confirmed: false, confirmed_at: null, playing_handicap: 0, gross_total: 7, net_total: 7, holes_scored: 1, holes: [{ hole_id: 'hole', hole_number: 1, par: 4, stroke_index: 1, handicap_strokes: 0, net_strokes: 7, score: { id: 'score', gross_strokes: 7 } }] }
const url = `/tournaments/${tournament.id}/rounds/${round.id}/scorecards/player/${owner.id}?metric=gross&view=summary`
function mount(path = url) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } })
  const router = createMemoryRouter([{ path: '/rounds/:roundId/matches/:matchId', element: <MatchPage /> }, { path: '/tournaments/:tournamentId/results/players/:playerId', element: <PlayerHistoryPage /> }, { path: '/tournaments/:tournamentId/rounds/:roundId/scorecards/:ownerType/:ownerId', element: <DirectScorecardPage /> }, { path: '/leaderboard', element: <LeaderboardPage /> }, { path: '/away', element: <p>Away</p> }], { initialEntries: [path] })
  render(<QueryClientProvider client={client}><AuthContext value={auth}><RouterProvider router={router} /></AuthContext></QueryClientProvider>)
  return { client, router }
}
beforeEach(() => {
  vi.spyOn(window, 'scrollTo').mockImplementation(() => undefined)
  vi.spyOn(api, 'rounds').mockResolvedValue([round]); vi.spyOn(api, 'tournaments').mockResolvedValue([tournament])
  vi.spyOn(api, 'roundLeaderboard').mockResolvedValue(board); vi.spyOn(api, 'scorecardRead').mockResolvedValue(card)
})
afterEach(() => { cleanup(); vi.restoreAllMocks() })
it.each([401, 403, 404])('erases cached card and dependent names after %s and never revives them on retry/remount', async status => {
  const { client, router } = mount(); await screen.findByText('Private Player')
  vi.mocked(api.scorecardRead).mockRejectedValue(new ApiHttpError(status, 'denied', 'Access denied'))
  await act(() => client.invalidateQueries({ queryKey: scoringKeys.read(session.user_id, round.id, owner) }))
  await waitFor(() => expect(screen.queryByText('Private Player')).toBeNull())
  expect(client.getQueryData(scoringKeys.read(session.user_id, round.id, owner))).toBeUndefined()
  vi.mocked(api.scorecardRead).mockRejectedValue(new ApiHttpError(500, 'unavailable', 'Temporary failure'))
  await act(() => router.navigate('/away')); await act(() => router.navigate(url))
  await act(() => client.invalidateQueries())
  expect(screen.queryByText('Private Player')).toBeNull()
  vi.mocked(api.scorecardRead).mockResolvedValue(card)
  await act(() => client.invalidateQueries()); await screen.findByText('Private Player')
})
it('keeps authorized data on an ordinary 500 and offline failure', async () => {
  const { client } = mount(); await screen.findByText('Private Player')
  for (const error of [new ApiHttpError(500, 'unavailable', 'Temporary failure'), new TypeError('Failed to fetch')]) {
    vi.mocked(api.scorecardRead).mockRejectedValue(error)
    await act(() => client.invalidateQueries({ queryKey: scoringKeys.read(session.user_id, round.id, owner) }))
    expect(screen.getByText('Private Player')).toBeTruthy()
  }
})
it('erases cached round results and sibling metric on a rounds dependency denial', async () => {
  const { client } = mount(`/leaderboard?tournament=${tournament.id}&scope=round&round=${round.id}&metric=gross`)
  await screen.findByText('Private Player')
  client.setQueryData(leaderboardKeys.round(session.user_id, round.id, 'net'), { ...board, metric: 'net' })
  vi.mocked(api.rounds).mockRejectedValue(new ApiHttpError(403, 'denied', 'Access denied'))
  await act(() => client.invalidateQueries({ queryKey: tournamentKeys.rounds(session.user_id, tournament.id) }))
  await waitFor(() => expect(screen.queryByText('Private Player')).toBeNull())
  expect(client.getQueryData(leaderboardKeys.round(session.user_id, round.id, 'net'))).toBeUndefined()
})
it.each(['rounds', 'result'])('hides history and its scorecard links after denied %s', async dependency => {
  vi.mocked(api.rounds).mockResolvedValue(tieRounds)
  vi.spyOn(api, 'tournamentLeaderboard').mockResolvedValue(tieBoard())
  const player = tieBoard().entries[0]; if (!player) throw new Error('Missing player')
  const { client } = mount(`/tournaments/${tieTournamentId}/results/players/${player.player_id}?metric=gross`)
  await screen.findByText(player.display_name)
  const error = new ApiHttpError(403, 'denied', 'Access denied')
  if (dependency === 'rounds') vi.mocked(api.rounds).mockRejectedValue(error)
  else vi.mocked(api.tournamentLeaderboard).mockRejectedValue(error)
  await act(() => client.invalidateQueries())
  await waitFor(() => expect(screen.queryByText(player.display_name)).toBeNull())
  expect(document.querySelector('a[href*="scorecards"]')).toBeNull()
  vi.mocked(api.tournamentLeaderboard).mockRejectedValue(new TypeError('Offline'))
  await act(() => client.invalidateQueries())
  expect(screen.queryByText(player.display_name)).toBeNull()
})
it.each(['round', 'card'])('hides a read-only match after denied %s through pending retry', async dependency => {
  const match = matchFixture(), matchRound = { ...round, id: matchIds.round, tournament_id: matchIds.tournament, scoring_format: 'singles_match_play' as const }
  vi.spyOn(api, 'round').mockResolvedValue(matchRound); vi.spyOn(matchApi, 'read').mockResolvedValue(match)
  const { client } = mount(`/rounds/${matchIds.round}/matches/${matchIds.match}`)
  await screen.findByRole('heading', { name: /Andreas.*Bjørn/ })
  const error = new ApiHttpError(403, 'denied', 'Access denied')
  if (dependency === 'round') vi.mocked(api.round).mockRejectedValue(error)
  else vi.mocked(matchApi.read).mockRejectedValue(error)
  await act(() => client.invalidateQueries())
  await waitFor(() => expect(screen.queryByRole('heading', { name: /Andreas.*Bjørn/ })).toBeNull())
  expect(client.getQueryData(matchKeys.read(session.user_id, matchIds.round, matchIds.match))).toBeUndefined()
  let release: () => void = () => undefined
  const pending = new Promise<void>(resolve => { release = resolve })
  vi.mocked(api.round).mockImplementation(async () => { await pending; return matchRound })
  vi.mocked(matchApi.read).mockImplementation(async () => { await pending; return match })
  const refresh = client.invalidateQueries()
  expect(screen.queryByRole('heading', { name: /Andreas.*Bjørn/ })).toBeNull()
  await act(async () => { release(); await refresh })
  await screen.findByRole('heading', { name: /Andreas.*Bjørn/ })
})
