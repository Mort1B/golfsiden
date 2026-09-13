// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { tournamentApi, tournamentKeys } from '../../api/tournaments'
import { leaderboardKeys } from '../../api/leaderboards'
import { ApiHttpError } from '../../api/http'
import type { Tournament } from '../../api/types'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { round, session, tournament } from './lifecycle/__tests__/fixtures'
import { CountedRoundsEditor } from './CountedRoundsEditor'

let client: QueryClient
let server: Tournament
let auth: AuthContextValue
function Harness({ refreshing = false, roundsError = null }: { refreshing?: boolean; roundsError?: Error | null }) {
  const detail = useQuery({ queryKey: tournamentKeys.detail(session.user_id, tournament.id), queryFn: () => tournamentApi.detail(tournament.id) })
  if (!detail.data) return <p>Henter turnering</p>
  return <CountedRoundsEditor tournament={detail.data} rounds={[{ ...round, status: 'draft' }]}
    roundsPending={false} roundsError={roundsError} onRetryRounds={vi.fn()} authorityRefreshing={refreshing || detail.isFetching} />
}
function tree(refreshing = false, roundsError: Error | null = null) {
  return <QueryClientProvider client={client}><AuthContext value={auth}><Harness refreshing={refreshing} roundsError={roundsError} /></AuthContext></QueryClientProvider>
}
async function choose() {
  const field = await screen.findByLabelText('Ved lik totalscore sammenlagt')
  await waitFor(() => expect(field.hasAttribute('disabled')).toBe(false))
  fireEvent.change(field, { target: { value: 'final_round_score' } })
  return field
}
async function save() { await choose(); fireEvent.click(screen.getByRole('button', { name: 'Lagre valg' })) }
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  server = { ...tournament, status: 'draft' }
  auth = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
  vi.spyOn(tournamentApi, 'detail').mockImplementation(async () => server)
  vi.spyOn(tournamentApi, 'updateCountedRounds').mockImplementation(async (_id, input) => {
    server = { ...server, tie_break_policy: input.tie_break_policy ?? server.tie_break_policy, updated_at: '2026-09-06T11:00:00Z' }
    return server
  })
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })

it('saves the declared rule with CSRF/version and refreshes both metric caches', async () => {
  const gross = leaderboardKeys.tournament(session.user_id, tournament.id, 'gross')
  const net = leaderboardKeys.tournament(session.user_id, tournament.id, 'net')
  client.setQueryData(gross, {}); client.setQueryData(net, {})
  render(tree()); await save()
  await screen.findByText(/^Lagret:.*Siste runde, deretter delt plass/)
  expect(tournamentApi.updateCountedRounds).toHaveBeenCalledWith(tournament.id, {
    counted_rounds: 1, mandatory_round_id: null, tie_break_policy: 'final_round_score', expected_tournament_updated_at: tournament.updated_at,
  }, session.csrf_token)
  expect(client.getQueryState(gross)?.isInvalidated).toBe(true)
  expect(client.getQueryState(net)?.isInvalidated).toBe(true)
  expect(screen.getByText('Lik totalscore: Siste runde, deretter delt plass.')).toBeTruthy()
})
it('keeps a failed choice for explicit retry after authoritative reconciliation', async () => {
  vi.mocked(tournamentApi.updateCountedRounds).mockRejectedValueOnce(new Error('Server utilgjengelig'))
  render(tree()); await save()
  await screen.findByText('Server utilgjengelig')
  const retry = screen.getByRole('button', { name: 'Prøv lagring igjen' })
  await waitFor(() => expect(retry.hasAttribute('disabled')).toBe(false))
  fireEvent.click(retry)
  await screen.findByText(/^Lagret:.*Siste runde/)
})
it('discards stale drafts and shows the refreshed server policy', async () => {
  vi.mocked(tournamentApi.updateCountedRounds).mockRejectedValue(new ApiHttpError(409, 'tournament_configuration_stale', 'stale'))
  render(tree()); await save()
  await screen.findByText(/Turneringen ble endret et annet sted/)
  await waitFor(() => expect(screen.getByLabelText('Ved lik totalscore sammenlagt')).toHaveProperty('value', 'shared_positions'))
  expect(screen.getByRole('button', { name: 'Prøv lagring igjen' }).hasAttribute('disabled')).toBe(true)
})
it('disables cached draft controls during authority refresh and round-read failure', async () => {
  const view = render(tree()); await choose()
  view.rerender(tree(true))
  expect(screen.getByLabelText('Ved lik totalscore sammenlagt').hasAttribute('disabled')).toBe(true)
  view.rerender(tree(false, new Error('offline')))
  expect(screen.getByLabelText('Ved lik totalscore sammenlagt').hasAttribute('disabled')).toBe(true)
  fireEvent.submit(screen.getByRole('button', { name: 'Lagre valg' }).closest('form') ?? document.body)
  expect(tournamentApi.updateCountedRounds).not.toHaveBeenCalled()
})
it('retains the declared policy visibly after start and removes mutation controls', async () => {
  server = { ...server, status: 'active', tie_break_policy: 'final_round_score' }
  render(tree())
  await screen.findByText('Lik totalscore: Siste runde, deretter delt plass.')
  expect(screen.queryByLabelText('Ved lik totalscore sammenlagt')).toBeNull()
  await screen.findByText(/Valget er låst fordi turneringen er startet/)
})
it('blocks after failed reconciliation until explicit refresh succeeds', async () => {
  vi.mocked(tournamentApi.updateCountedRounds).mockImplementation(async () => {
    vi.mocked(tournamentApi.detail).mockRejectedValue(new Error('offline'))
    throw new Error('Uklart svar')
  })
  render(tree()); await save()
  await screen.findByText(/Kunne ikke oppdatere innstillingene/)
  expect(screen.getByLabelText('Ved lik totalscore sammenlagt').hasAttribute('disabled')).toBe(true)
  vi.mocked(tournamentApi.detail).mockResolvedValue(server)
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater innstillingene' }))
  await waitFor(() => expect(screen.getByLabelText('Ved lik totalscore sammenlagt').hasAttribute('disabled')).toBe(false))
})
it('prevents duplicate submits and ignores late responses after unmount', async () => {
  let finish: ((value: Tournament) => void) | undefined
  vi.mocked(tournamentApi.updateCountedRounds).mockImplementation(() => new Promise(resolve => { finish = resolve }))
  const view = render(tree()); await choose()
  const form = screen.getByRole('button', { name: 'Lagre valg' }).closest('form')
  if (!form) throw new Error('Missing form')
  fireEvent.submit(form); fireEvent.submit(form)
  await waitFor(() => expect(tournamentApi.updateCountedRounds).toHaveBeenCalledTimes(1))
  view.unmount(); client.clear()
  await act(async () => { finish?.({ ...server, tie_break_policy: 'final_round_score' }) })
  expect(client.getQueryCache().getAll()).toHaveLength(0)
})
it('remounts a usable form on same-account session replacement and ignores the previous receipt', async () => {
  let finish: ((value: Tournament) => void) | undefined
  vi.mocked(tournamentApi.updateCountedRounds).mockImplementation(() => new Promise(resolve => { finish = resolve }))
  const view = render(tree()); await save()
  await waitFor(() => expect(finish).toBeDefined())
  auth = { ...auth, session: { ...session, csrf_token: 'replacement-session' } }
  view.rerender(tree())
  await act(async () => { finish?.({ ...server, tie_break_policy: 'final_round_score' }) })
  await choose()
  expect(screen.queryByText(/^Lagret:/)).toBeNull()
  expect(screen.getByRole('button', { name: 'Lagre valg' }).hasAttribute('disabled')).toBe(false)
})
