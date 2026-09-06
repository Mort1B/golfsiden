// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../../api/client'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import { handleTournamentLiveSignal } from '../../api/liveInvalidation'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { session } from '../tournaments/lifecycle/__tests__/fixtures'
import { FlightProgressPanel } from './FlightProgressPanel'
import { pairings, progress } from './flightProgressFixtures'
import type { RoundCompletionValidation } from '../../api/scorecards'

let client: QueryClient
const refresh = vi.fn()
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
const key = privateWorkspaceKeys.completion(session.user_id, 'round')
function mount(roster = pairings()) {
  return render(<QueryClientProvider client={client}><AuthContext value={auth}>
    <FlightProgressPanel pairings={roster} onRefreshRound={refresh} />
  </AuthContext></QueryClientProvider>)
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  vi.spyOn(api, 'completionValidation').mockResolvedValue(progress())
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks(); refresh.mockClear() })

it('loads the canonical projection and renders full card and flight counts', async () => {
  mount()
  await screen.findByText('20/36 hullregistreringer fordelt på 2 scorekort')
  expect(screen.getByText('1/2 fullført · 1/2 bekreftet')).toBeTruthy()
  expect(api.completionValidation).toHaveBeenCalledWith('round', 'individual_stroke_play')
  expect(client.getQueryData(key)).toEqual(progress())
})
it('does not request progress for a draft', () => {
  mount({ ...pairings(), status: 'draft' })
  expect(screen.getByText('Fremdrift blir tilgjengelig når runden åpnes.')).toBeTruthy()
  expect(api.completionValidation).not.toHaveBeenCalled()
})
it('keeps loading, empty and failed refresh states explicit and retries both snapshots', async () => {
  vi.mocked(api.completionValidation).mockImplementation(() => new Promise(() => {}))
  mount()
  expect(screen.getByRole('status')).toBeTruthy()
  await act(async () => { client.setQueryData(key, { ...progress(), owners: [] }) })
  await screen.findByText('Ingen scorekort med flighttilknytning er tilgjengelige.')
  vi.mocked(api.completionValidation).mockRejectedValue(new Error('Ingen tilgang'))
  await act(async () => { await client.invalidateQueries({ queryKey: key }) })
  await screen.findByRole('alert')
  expect(screen.queryByText('Ingen scorekort med flighttilknytning er tilgjengelige.')).toBeNull()
  vi.mocked(api.completionValidation).mockResolvedValue(progress())
  fireEvent.click(screen.getByRole('button', { name: 'Prøv igjen' }))
  expect(refresh).toHaveBeenCalled()
  await screen.findByText('20/36 hullregistreringer fordelt på 2 scorekort')
})
it('clears released progress immediately on visibility/reconnect/error, then renders only projected counts', async () => {
  mount()
  await screen.findByText('1/2 fullført · 1/2 bekreftet')
  let resolve: ((data: RoundCompletionValidation) => void) | undefined
  vi.mocked(api.completionValidation).mockImplementation(() => new Promise((done) => { resolve = done }))
  await act(async () => { void handleTournamentLiveSignal(client, session.user_id, 'visibility') })
  expect(client.getQueryData(key)).toBeUndefined()
  await waitFor(() => expect(screen.queryByText('1/2 fullført · 1/2 bekreftet')).toBeNull())
  expect(screen.getByRole('status')).toBeTruthy()
  await waitFor(() => expect(resolve).toBeDefined())
  const hidden: RoundCompletionValidation = { ...progress(), visibility: { mode: 'front_nine' }, ready_to_complete: null, ready_to_lock: null,
    owners: progress().owners.map((owner) => ({ ...owner, holes_scored: 9, required_holes: 9, complete: null, confirmed: null })) }
  await act(async () => { resolve?.(hidden) })
  expect(await screen.findAllByText('18/18 synlige hullregistreringer fordelt på 2 scorekort')).toHaveLength(2)
  expect(screen.queryByText(/fullført|Bekreftet/)).toBeNull()
  await act(async () => { await handleTournamentLiveSignal(client, session.user_id, 'error') })
  expect(client.getQueryData(key)).toBeUndefined()
  await waitFor(() => expect(screen.queryByText(/18\/18/)).toBeNull())
  vi.mocked(api.completionValidation).mockResolvedValue(progress())
  await act(async () => { await handleTournamentLiveSignal(client, session.user_id, 'open') })
  await screen.findByText('1/2 fullført · 1/2 bekreftet')
})
it('suppresses incompatible or missing historical mappings', async () => {
  mount({ ...pairings(), flights: [] })
  expect((await screen.findByRole('alert')).textContent).toContain('Eldre runder')
  expect(screen.queryByText(/20\/36/)).toBeNull()
})
