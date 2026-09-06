// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../../api/client'
import { pairingApi, pairingKeys, type RoundPairings } from '../../api/pairings'
import { RoundPage } from '../../pages/RoundPage'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { round, session, tournament } from '../tournaments/lifecycle/__tests__/fixtures'

vi.mock('../live/useTournamentLive', () => ({ useTournamentLive: vi.fn() }))
let client: QueryClient
let data: RoundPairings
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
const group = { id: '00000000-0000-0000-0000-000000000005', name: 'Flight med langt navn',
  starting_hole: 10, tee_time: '09:30:00', created_at: round.created_at, updated_at: round.updated_at,
  members: [{ player_id: session.player_id ?? '', display_name: 'Spiller med langt navn', display_order: 1 }] }

function mount(id = round.id, value = auth) {
  return render(<QueryClientProvider client={client}><AuthContext value={value}>
    <MemoryRouter initialEntries={[`/rounds/${id}`]}><Routes><Route path="/rounds/:roundId" element={<RoundPage />} /></Routes></MemoryRouter>
  </AuthContext></QueryClientProvider>)
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  data = { round_id: round.id, tournament_id: tournament.id, status: round.status, scoring_format: round.scoring_format,
    updated_at: round.updated_at, active_entrants: [], inactive_entrants: [], flights: [group], teams: [], legacy_individual_groups: [] }
  vi.spyOn(api, 'round').mockResolvedValue(round)
  vi.spyOn(api, 'myTournaments').mockResolvedValue([])
  vi.spyOn(pairingApi, 'get').mockImplementation(async () => data)
  vi.spyOn(api, 'teams')
  vi.spyOn(api, 'completionValidation')
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })

it('shows flight schedules on an individual draft, no empty teams or scoring authority', async () => {
  mount()
  await screen.findByText(group.name)
  expect(screen.getByText('09:30')).toBeTruthy()
  expect(screen.getByText('Start hull 10')).toBeTruthy()
  expect(screen.queryByRole('heading', { name: 'Lag' })).toBeNull()
  expect(screen.queryByRole('link', { name: 'Åpne scorekort' })).toBeNull()
  expect(screen.queryByRole('link', { name: 'Administrer runden' })).toBeNull()
  expect(screen.getByText('Bane ikke satt opp')).toBeTruthy()
  expect(screen.getByRole('link', { name: 'Se rundens resultater' }).getAttribute('href')).toBe(`/leaderboard?tournament=${tournament.id}&scope=round&round=${round.id}&metric=net`)
  expect(pairingApi.get).toHaveBeenCalledWith(round.id, tournament.id)
  expect(api.teams).not.toHaveBeenCalled()
  expect(api.completionValidation).not.toHaveBeenCalled()
})

it.each(['team_scramble', 'two_player_foursomes'] as const)('separates %s teams from flights without reusing team schedules', async (scoring_format) => {
  vi.mocked(api.round).mockResolvedValue({ ...round, scoring_format })
  data = { ...data, scoring_format, teams: [{ ...group, name: 'Scorelaget', tee_time: '22:22:00', starting_hole: 36 }] }
  mount()
  await screen.findByText('Scorelaget')
  expect(screen.getByRole('heading', { name: 'Lag' })).toBeTruthy()
  expect(screen.queryByText(/22:22|Start hull 36/)).toBeNull()
})

it.each(['open', 'completed', 'locked'] as const)('links %s rounds to exact scorecard summary without choosing an owner', async (status) => {
  vi.mocked(api.round).mockResolvedValue({ ...round, status })
  data.status = status
  mount()
  const link = await screen.findByRole('link', { name: 'Åpne scorekort' })
  expect(link.getAttribute('href')).toBe(`/score?tournament=${tournament.id}&round=${round.id}&view=summary`)
})

it('shows deliberate empty groups and missing schedule/member states', async () => {
  data.flights = [{ ...group, starting_hole: null, tee_time: null, members: [] }]
  mount()
  await screen.findByText('Starttid ikke satt')
  expect(screen.getByText('Starthull ikke satt')).toBeTruthy()
  expect(screen.getByText('Ingen spillere er satt opp.')).toBeTruthy()
  await act(async () => { data = { ...data, flights: [] }; await client.invalidateQueries({ queryKey: pairingKeys.detail(session.user_id, round.id) }) })
  await screen.findByText('Ingen flighter er satt opp.')
})

it('labels legacy individual groups separately', async () => {
  data.legacy_individual_groups = [{ ...group, name: 'Gammel gruppe' }]
  mount()
  await screen.findByRole('heading', { name: 'Eldre individuelle grupper' })
  expect(screen.queryByRole('heading', { name: 'Lag' })).toBeNull()
})

it('hides retained member data on failed refresh and retries', async () => {
  mount()
  await screen.findByText(group.name)
  vi.mocked(pairingApi.get).mockRejectedValue(new Error('Ingen tilgang'))
  await act(async () => { await client.invalidateQueries({ queryKey: pairingKeys.detail(session.user_id, round.id) }) })
  expect((await screen.findByRole('alert')).textContent).toContain('Ingen tilgang')
  expect(screen.queryByText(group.name)).toBeNull()
  vi.mocked(pairingApi.get).mockResolvedValue(data)
  fireEvent.click(screen.getByRole('button', { name: 'Prøv igjen' }))
  await screen.findByText(group.name)
})

it('rejects mixed lifecycle snapshots and recovers on refresh', async () => {
  data.status = 'open'
  mount()
  await screen.findByRole('alert')
  expect(screen.queryByText(group.name)).toBeNull()
  expect(screen.queryByRole('link', { name: 'Åpne scorekort' })).toBeNull()
  vi.mocked(api.round).mockResolvedValue({ ...round, status: 'open' })
  fireEvent.click(screen.getByRole('button', { name: 'Prøv igjen' }))
  await screen.findByText(group.name)
})

it('shows loading while pairings are pending and does not invent an empty roster', async () => {
  vi.mocked(pairingApi.get).mockImplementation(() => new Promise(() => {}))
  mount()
  await waitFor(() => expect(pairingApi.get).toHaveBeenCalled())
  expect(screen.getByRole('status').textContent).toBe('Laster …')
  expect(screen.queryByText('Ingen flighter er satt opp.')).toBeNull()
})

it('rejects malformed round routes without reads', () => {
  mount('invalid')
  expect(screen.getByRole('alert').textContent).toContain('Ugyldig runde')
  expect(api.round).not.toHaveBeenCalled()
  expect(pairingApi.get).not.toHaveBeenCalled()
})

it('shows the exact round management link only with exact tournament-admin membership', async () => {
  vi.mocked(api.myTournaments).mockResolvedValue([{ tournament, role: 'admin', player_id: session.player_id }])
  mount()
  const link = await screen.findByRole('link', { name: 'Administrer runden' })
  expect(link.getAttribute('href')).toBe(`/manage/tournaments/${tournament.id}?round=${round.id}#lifecycle`)
})

it('does not reuse another account’s cached pairings or management authority', async () => {
  client.setQueryData(pairingKeys.detail(session.user_id, round.id), data)
  vi.mocked(pairingApi.get).mockImplementation(() => new Promise(() => {}))
  mount(round.id, { ...auth, session: { ...session, user_id: '00000000-0000-0000-0000-000000000099' } })
  await waitFor(() => expect(pairingApi.get).toHaveBeenCalled())
  expect(screen.queryByText(group.name)).toBeNull()
  expect(screen.queryByRole('link', { name: 'Administrer runden' })).toBeNull()
})
