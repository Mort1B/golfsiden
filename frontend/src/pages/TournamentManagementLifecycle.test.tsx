// @vitest-environment jsdom
import { cleanup, render, screen, waitFor, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Routes, Route } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '../api/client'
import { tournamentApi, tournamentKeys } from '../api/tournaments'
import { roundLifecycleApi } from '../api/roundLifecycle'
import { finalRoundVisibilityApi } from '../api/finalRoundVisibility'
import { clearPrivateWorkspace } from '../api/privateWorkspace'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { tournament, round, session, opening } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { TournamentManagementPage } from './TournamentManagementPage'

vi.mock('../features/live/useTournamentLive', () => ({ useTournamentLive: vi.fn() }))
let client: QueryClient
const auth: AuthContextValue = {
  session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn(),
}
function tree(value: AuthContextValue = auth) {
  return <QueryClientProvider client={client}><AuthContext value={value}>
    <MemoryRouter initialEntries={[`/manage/tournaments/${tournament.id}?round=${round.id}`]}>
      <Routes><Route path="/manage/tournaments/:tournamentId" element={<TournamentManagementPage />} /></Routes>
    </MemoryRouter>
  </AuthContext></QueryClientProvider>
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  vi.spyOn(tournamentApi, 'detail').mockResolvedValue(tournament)
  vi.spyOn(tournamentApi, 'mine').mockResolvedValue([{ tournament, role: 'admin', player_id: session.player_id }])
  vi.spyOn(tournamentApi, 'rounds').mockResolvedValue([round])
  vi.spyOn(tournamentApi, 'players').mockResolvedValue({ players: [], handicap_correction: { state: 'editable' } })
  vi.spyOn(api, 'round').mockResolvedValue(round)
  vi.spyOn(roundLifecycleApi, 'validation').mockResolvedValue(opening)
  vi.spyOn(finalRoundVisibilityApi, 'get').mockResolvedValue({ tournament_id: tournament.id, back_nine_hidden: true, visibility_updated_at: tournament.updated_at })
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })

describe('management lifecycle authority gate', () => {
  it.each(['player', 'scorer', 'viewer'] as const)('does not let a global admin with exact %s membership administer rounds', async (role) => {
    vi.mocked(tournamentApi.mine).mockResolvedValue([{ tournament, role, player_id: session.player_id }])
    render(tree())
    await screen.findByRole('heading', { name: 'Ingen tilgang' })
    expect(tournamentApi.rounds).not.toHaveBeenCalled()
    expect(tournamentApi.players).not.toHaveBeenCalled()
    expect(roundLifecycleApi.validation).not.toHaveBeenCalled()
    expect(screen.queryByRole('heading', { name: 'Dette trenger oppfølging' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Åpne runden' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Fullfør turneringen' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Arkiver turneringen' })).toBeNull()
  })
  it('removes lifecycle data and controls after exact admin membership is revoked', async () => {
    render(tree())
    const open = await screen.findByRole('button', { name: 'Åpne runden' })
    await waitFor(() => expect(open.hasAttribute('disabled')).toBe(false))
    vi.mocked(tournamentApi.mine).mockResolvedValue([{ tournament, role: 'viewer', player_id: session.player_id }])
    await act(async () => { await client.invalidateQueries({ queryKey: tournamentKeys.mine(session.user_id) }) })
    await screen.findByRole('heading', { name: 'Ingen tilgang' })
    expect(screen.queryByRole('heading', { name: 'Dette trenger oppfølging' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Åpne runden' })).toBeNull()
    expect(screen.queryByText('Runden er klar til å åpnes.')).toBeNull()
    expect(screen.queryByRole('button', { name: 'Fullfør turneringen' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Arkiver turneringen' })).toBeNull()
  })
  it('does not show the prior account controls during an identity switch', async () => {
    const view = render(tree())
    await screen.findByRole('button', { name: 'Åpne runden' })
    vi.mocked(tournamentApi.mine).mockResolvedValue([])
    act(() => clearPrivateWorkspace(client))
    view.rerender(tree({ ...auth, session: { ...session, user_id: '00000000-0000-0000-0000-000000000099' } }))
    expect(screen.queryByRole('heading', { name: 'Dette trenger oppfølging' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Åpne runden' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Fullfør turneringen' })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Arkiver turneringen' })).toBeNull()
    await screen.findByRole('heading', { name: 'Ingen tilgang' })
  })
  it('renders an empty round list without lifecycle actions', async () => {
    vi.mocked(tournamentApi.rounds).mockResolvedValue([])
    render(tree())
    await screen.findAllByText('Ingen runder er opprettet.')
    expect(screen.queryByRole('button', { name: 'Åpne runden' })).toBeNull()
    expect(roundLifecycleApi.validation).not.toHaveBeenCalled()
  })
})
