// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, expect, it, vi } from 'vitest'
import { authKeys } from '../api/auth'
import { ApiHttpError } from '../api/http'
import { api } from '../api/client'
import { tournamentApi, tournamentKeys } from '../api/tournaments'
import { finalRoundVisibilityApi } from '../api/finalRoundVisibility'
import { awardsApi } from '../api/roundAwards'
import { view } from '../features/roundAwards/fixtures'
import { AuthContext } from '../features/auth/authContext'
import { ScoringGuardProvider } from '../features/scoring/ScoringGuardProvider'
import { useScoringGuard } from '../features/scoring/scoringGuardContext'
import { tournament, round, session, completion } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { TournamentManagementPage } from './TournamentManagementPage'
vi.mock('../features/live/useTournamentLive', () => ({ useTournamentLive: vi.fn() }))
afterEach(() => { cleanup(); vi.restoreAllMocks() })
function GuardStatus() { return <p>Guard {String(useScoringGuard().blocked)}</p> }
it.each([403, 404])('keeps the signed-in account through admin loss during award dispatch (%s), then abandons and leaves without another write', async status => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  client.setQueryData(authKeys.session, session)
  const completed = { ...round, status: 'completed' as const }
  vi.spyOn(tournamentApi, 'detail').mockResolvedValue(tournament)
  vi.spyOn(tournamentApi, 'mine').mockResolvedValue([{ tournament, role: 'admin', player_id: session.player_id }])
  vi.spyOn(tournamentApi, 'rounds').mockResolvedValue([completed])
  vi.spyOn(tournamentApi, 'players').mockResolvedValue({ players: [], handicap_correction: { state: 'editable' } })
  vi.spyOn(api, 'round').mockResolvedValue(completed)
  vi.spyOn(api, 'completionValidation').mockResolvedValue(completion('completed'))
  vi.spyOn(finalRoundVisibilityApi, 'get').mockResolvedValue({ tournament_id: tournament.id, back_nine_hidden: true, visibility_updated_at: tournament.updated_at })
  vi.spyOn(awardsApi, 'read').mockResolvedValue({ ...view, round_id: round.id, tournament_id: tournament.id })
  let deny: ((error: Error) => void) | undefined
  vi.spyOn(awardsApi, 'write').mockImplementation(() => new Promise((_resolve, reject) => { deny = reject }))
  const router = createMemoryRouter([{ path: '/manage/tournaments/:tournamentId', element: <TournamentManagementPage /> }, { path: '/tournaments', element: <p>Safe exit</p> }], { initialEntries: [`/manage/tournaments/${tournament.id}?round=${round.id}`] })
  render(<QueryClientProvider client={client}><AuthContext value={{ session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }}><ScoringGuardProvider><GuardStatus /><RouterProvider router={router} /></ScoringGuardProvider></AuthContext></QueryClientProvider>)
  fireEvent.change(await screen.findByLabelText('Vinner av Closest to pin'), { target: { value: view.eligible_players[0]?.player_id } })
  fireEvent.change(screen.getByLabelText('Begrunnelse for Closest to pin'), { target: { value: 'Målt' } })
  const save = screen.getByRole('button', { name: 'Lagre Closest to pin' })
  await waitFor(() => expect(save).toHaveProperty('disabled', false)); fireEvent.click(save)
  await waitFor(() => expect(awardsApi.write).toHaveBeenCalledTimes(1))
  vi.mocked(tournamentApi.mine).mockResolvedValue([{ tournament, role: 'viewer', player_id: session.player_id }])
  await act(async () => { await client.invalidateQueries({ queryKey: tournamentKeys.mine(session.user_id) }) })
  await screen.findByRole('heading', { name: 'Ingen tilgang' })
  await act(async () => deny?.(new ApiHttpError(status, 'forbidden', 'Access denied')))
  await screen.findByText(/Utfallet av den opprinnelige innsendingen er fortsatt uavklart/)
  expect(client.getQueryData(authKeys.session)).toEqual(session)
  await act(async () => router.navigate('/tournaments'))
  expect(router.state.location.pathname).toContain('/manage/tournaments/')
  expect(screen.getByText('Guard true')).toBeTruthy()
  const guardedUnload = new Event('beforeunload', { cancelable: true }); window.dispatchEvent(guardedUnload); expect(guardedUnload.defaultPrevented).toBe(true)
  expect(screen.getByText(/angrer ikke noe serveren allerede har godtatt/)).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: 'Oppgi lokal gjenoppretting og forlat siden' }))
  await screen.findByText('Safe exit'); await screen.findByText('Guard false')
  const releasedUnload = new Event('beforeunload', { cancelable: true }); window.dispatchEvent(releasedUnload); expect(releasedUnload.defaultPrevented).toBe(false)
  expect(awardsApi.write).toHaveBeenCalledTimes(1)
  expect(client.getQueryData(authKeys.session)).toEqual(session)
  client.clear()
})
