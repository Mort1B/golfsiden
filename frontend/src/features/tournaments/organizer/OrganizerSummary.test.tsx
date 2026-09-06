// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider, onlineManager } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { handleTournamentLiveSignal } from '../../../api/liveInvalidation'
import { api } from '../../../api/client'
import { privateWorkspaceKeys } from '../../../api/privateWorkspace'
import { roundLifecycleApi, roundLifecycleKeys } from '../../../api/roundLifecycle'
import type { Round } from '../../../api/types'
import { AuthContext, type AuthContextValue } from '../../auth/authContext'
import { completion, opening, round, session, tournament } from '../lifecycle/__tests__/fixtures'
import { OrganizerSummary } from './OrganizerSummary'

let client: QueryClient
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
const refresh = vi.fn()
function tree(rounds: Round[] = [round], blocked = false) {
  return <QueryClientProvider client={client}><AuthContext value={auth}><MemoryRouter>
    <OrganizerSummary tournament={tournament} rounds={rounds} pending={false} error={null} authorityRefreshing={blocked} onRefresh={refresh} />
  </MemoryRouter></AuthContext></QueryClientProvider>
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  vi.spyOn(roundLifecycleApi, 'validation').mockImplementation(async id => ({ ...opening, round_id: id }))
  vi.spyOn(api, 'completionValidation').mockResolvedValue(completion())
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks(); refresh.mockClear() })

it('orders actionable rounds and does not request readiness for locked rounds', async () => {
  render(tree([{ ...round, id: 'third', round_number: 3 }, { ...round, id: 'locked', round_number: 2, status: 'locked' }, round]))
  await screen.findByText('Runde 1 er klar til å åpnes.')
  await screen.findByText('Runde 3 er klar til å åpnes.')
  expect(screen.getAllByRole('heading', { level: 3 }).map(item => item.textContent)).toEqual([`Runde 1: ${round.name}`, `Runde 3: ${round.name}`])
  expect(roundLifecycleApi.validation).toHaveBeenCalledTimes(2)
  expect(api.completionValidation).not.toHaveBeenCalled()
})

it('removes stale readiness during refetch and failure, then retries successfully', async () => {
  render(tree()); await screen.findByText('Runde 1 er klar til å åpnes.')
  let fail: (error: Error) => void = () => { throw new Error('not started') }
  vi.mocked(roundLifecycleApi.validation).mockImplementation(() => new Promise((_resolve, reject) => { fail = reject }))
  act(() => { void client.invalidateQueries({ queryKey: roundLifecycleKeys.validation(session.user_id, round.id) }) })
  await screen.findByText(/Kontrollerer Runde 1/)
  expect(screen.queryByText('Runde 1 er klar til å åpnes.')).toBeNull()
  await act(async () => fail(new Error('offline')))
  await screen.findByRole('alert')
  expect(screen.queryByRole('link', { name: 'Se åpning' })).toBeNull()
  vi.mocked(roundLifecycleApi.validation).mockResolvedValue(opening)
  fireEvent.click(screen.getByRole('button', { name: 'Prøv igjen for runde 1' }))
  await screen.findByText('Runde 1 er klar til å åpnes.')
  expect(refresh).toHaveBeenCalled()
})

it('hides hints while authority refreshes without remounting reads', async () => {
  const view = render(tree()); await screen.findByText('Runde 1 er klar til å åpnes.')
  view.rerender(tree([round], true))
  expect(screen.queryByRole('link', { name: 'Se åpning' })).toBeNull()
  await screen.findByText('Kontrollerer tilgang og rundestatus …')
  view.rerender(tree())
  await screen.findByText('Runde 1 er klar til å åpnes.')
  expect(roundLifecycleApi.validation).toHaveBeenCalledTimes(1)
})

it.each(['restricted', 'mismatched'] as const)('fails closed for %s completion data', async mode => {
  vi.mocked(api.completionValidation).mockResolvedValue(mode === 'restricted'
    ? { ...completion(), visibility: { mode: 'front_nine' } } : completion('completed'))
  render(tree([{ ...round, status: 'open' }]))
  await screen.findByRole('alert')
  expect(screen.queryByRole('link')).toBeNull()
  expect(screen.queryByText(/klar til å fullføres/)).toBeNull()
})

it('uses canonical completion invalidation after a scorecard confirmation', async () => {
  const unconfirmed = { ...completion(), ready_to_complete: false, owners: completion().owners.map(item => ({ ...item, confirmed: false })) }
  vi.mocked(api.completionValidation).mockResolvedValue(unconfirmed)
  render(tree([{ ...round, status: 'open' }]))
  await screen.findByText('Runde 1: 1 scorekort må bekreftes.')
  vi.mocked(api.completionValidation).mockResolvedValue(completion())
  await act(async () => { await client.invalidateQueries({ queryKey: privateWorkspaceKeys.completion(session.user_id, round.id), exact: true }) })
  await screen.findByText('Runde 1 er klar til å fullføres.')
})

it('shows calm empty and all-locked states', async () => {
  const view = render(tree([])); await screen.findByText('Ingen runder å følge opp ennå.')
  view.rerender(tree([{ ...round, status: 'locked' }]))
  await screen.findByText(/Alle rundene er låst/)
  await waitFor(() => expect(roundLifecycleApi.validation).not.toHaveBeenCalled())
})


it('clears completion hints on live disconnect and rejects restricted reconnect data', async () => {
  render(tree([{ ...round, status: 'open' }]))
  await screen.findByText('Runde 1 er klar til å fullføres.')
  await act(async () => { await handleTournamentLiveSignal(client, session.user_id, 'error') })
  await screen.findByText(/Kontrollerer Runde 1/)
  expect(screen.queryByRole('link')).toBeNull()
  vi.mocked(api.completionValidation).mockResolvedValue({ ...completion(), visibility: { mode: 'front_nine' } })
  await act(async () => { await handleTournamentLiveSignal(client, session.user_id, 'open') })
  await screen.findByRole('alert')
  expect(screen.queryByRole('link')).toBeNull()
})


it('hides cached readiness when the request is paused offline', async () => {
  render(tree()); await screen.findByText('Runde 1 er klar til å åpnes.')
  try {
    await act(async () => {
      onlineManager.setOnline(false)
      void client.invalidateQueries({ queryKey: roundLifecycleKeys.validation(session.user_id, round.id) })
    })
    await screen.findByRole('alert')
    expect(screen.queryByRole('link', { name: 'Se åpning' })).toBeNull()
  } finally {
    await act(async () => { onlineManager.setOnline(true) })
  }
})
