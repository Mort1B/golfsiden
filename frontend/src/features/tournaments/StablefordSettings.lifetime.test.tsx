// @vitest-environment jsdom
import { StrictMode } from 'react'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { StablefordSettings } from './StablefordSettings'
import { round as original, session } from './lifecycle/__tests__/fixtures'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { publishSessionTransition } from '../auth/sessionTransition'
import { authKeys, type AuthSession } from '../../api/auth'
import { tournamentKeys } from '../../api/tournaments'
import { stablefordApi } from '../../api/stableford'
import { ApiHttpError } from '../../api/http'
import type { Round } from '../../api/types'
const round: Round = { ...original, scoring_format: 'individual_stableford' }
const updated = { ...round, handicap_allowance_percent: 50 }
const clients: QueryClient[] = []
afterEach(() => { cleanup(); clients.splice(0).forEach(client => client.clear()); vi.restoreAllMocks() })
function deferred<T>() {
 let resolve: (value: T) => void = () => undefined
 let reject: (reason: unknown) => void = () => undefined
 const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
 return { promise, resolve, reject }
}
function mount() {
 const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } }); clients.push(client)
 client.setQueryData(authKeys.session, session)
 client.setQueryData(tournamentKeys.round(session.user_id, round.id), round)
 client.setQueryData(tournamentKeys.rounds(session.user_id, round.tournament_id), [round])
 const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
 const tree = (identity: AuthSession | null, target: Round | null = round) => <StrictMode><QueryClientProvider client={client}><AuthContext value={{ ...auth, session: identity }}>{target && <StablefordSettings round={target}/>}</AuthContext></QueryClientProvider></StrictMode>
 const view = render(tree(session))
 return { client, change: (identity: AuthSession | null, target: Round | null = round) => {
  publishSessionTransition(client, identity); view.rerender(tree(identity, target))
 } }
}
async function submit() {
 fireEvent.change(screen.getByLabelText('Handicapandel (0–100 %)'), { target: { value: '50' } })
 fireEvent.click(screen.getByRole('button', { name: 'Lagre Stableford-innstillinger' }))
}
for (const transition of ['logout', 'account', 'csrf', 'unmount', 'round'] as const) {
 for (const outcome of ['success', 'stale', 'opened', 'failure'] as const) it(`ignores late ${outcome} after ${transition}`, async () => {
  const pending = deferred<Round>(), save = vi.spyOn(stablefordApi, 'settings').mockReturnValue(pending.promise)
  const { client, change } = mount()
  await submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
  const next = transition === 'logout' ? null : transition === 'account' ? { ...session, user_id: crypto.randomUUID(), csrf_token: 'other' }
   : transition === 'csrf' ? { ...session, csrf_token: 'renewed' } : session
  act(() => change(next, ['logout', 'unmount'].includes(transition) ? null : transition === 'round' ? { ...round, id: crypto.randomUUID() } : round))
  const before = client.getQueriesData({ queryKey: ['private-workspace'] })
  const writes = vi.spyOn(client, 'setQueryData'), refresh = vi.spyOn(client, 'invalidateQueries')
  await act(async () => {
   if (outcome === 'success') pending.resolve(updated)
   else pending.reject(outcome === 'failure' ? new Error('synthetic failure') : new ApiHttpError(409, outcome === 'stale' ? 'round_configuration_stale' : 'round_not_draft', 'synthetic conflict'))
  })
  await waitFor(() => expect(client.getMutationCache().getAll().some(mutation => mutation.state.status === 'pending')).toBe(false))
  expect(writes).not.toHaveBeenCalled(); expect(refresh).not.toHaveBeenCalled()
  expect(client.getQueriesData({ queryKey: ['private-workspace'] })).toEqual(before)
  expect(screen.queryByText('Stableford-innstillingene er lagret.')).toBeNull()
  expect(screen.queryByRole('alert')).toBeNull()
 })
}
it('checks canonical session publication before React has rerendered the old owner', async () => {
 const pending = deferred<Round>(), save = vi.spyOn(stablefordApi, 'settings').mockReturnValue(pending.promise)
 const { client } = mount(); await submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
 act(() => publishSessionTransition(client, null))
 await act(async () => pending.resolve(updated))
 await waitFor(() => expect(client.getMutationCache().getAll().some(mutation => mutation.state.status === 'pending')).toBe(false))
 expect(client.getQueriesData({ queryKey: ['private-workspace'] })).toEqual([])
})
it('a renewed session can save a new draft without an old completion resetting its state', async () => {
 const old = deferred<Round>(), fresh = deferred<Round>()
 const save = vi.spyOn(stablefordApi, 'settings').mockReturnValueOnce(old.promise).mockReturnValueOnce(fresh.promise)
 const { client, change } = mount(); await submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
 act(() => change({ ...session, csrf_token: 'renewed' }))
 const button = () => screen.getByRole('button', { name: 'Lagre Stableford-innstillinger' })
 expect(button().hasAttribute('disabled')).toBe(false)
 fireEvent.change(screen.getByLabelText('Handicapandel (0–100 %)'), { target: { value: '70' } })
 fireEvent.click(button()); await waitFor(() => expect(save).toHaveBeenCalledTimes(2))
 await act(async () => old.resolve(updated))
 expect((screen.getByLabelText('Handicapandel (0–100 %)') as HTMLInputElement).value).toBe('70')
 expect(screen.getByRole('button', { name: 'Lagrer …' }).hasAttribute('disabled')).toBe(true)
 await act(async () => fresh.resolve({ ...round, handicap_allowance_percent: 70 }))
 await waitFor(() => expect(screen.getByRole('status').textContent).toContain('lagret'))
 expect(client.getQueryData<Round>(tournamentKeys.round(session.user_id, round.id))?.handicap_allowance_percent).toBe(70)
 expect(save.mock.calls[1]?.[3]).toBe('renewed')
})
it('does not reset replacement input when an old conflict refresh finishes later', async () => {
 const response = deferred<Round>(), refresh = deferred<void>()
 const save = vi.spyOn(stablefordApi, 'settings').mockReturnValue(response.promise)
 const { client, change } = mount()
 const invalidate = vi.spyOn(client, 'invalidateQueries').mockReturnValue(refresh.promise)
 await submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
 await act(async () => response.reject(new ApiHttpError(409, 'round_configuration_stale', 'synthetic stale')))
 await waitFor(() => expect(invalidate).toHaveBeenCalledTimes(2))
 act(() => change({ ...session, csrf_token: 'renewed' }))
 fireEvent.change(screen.getByLabelText('Handicapandel (0–100 %)'), { target: { value: '70' } })
 await act(async () => refresh.resolve())
 await waitFor(() => expect(client.getMutationCache().getAll().some(m => m.state.status === 'pending')).toBe(false))
 expect((screen.getByLabelText('Handicapandel (0–100 %)') as HTMLInputElement).value).toBe('70')
 expect(screen.queryByRole('alert')).toBeNull()
 expect(invalidate).toHaveBeenCalledTimes(2)
})
it('keeps current-session failures visible and allows a deliberate successful retry', async () => {
 const save = vi.spyOn(stablefordApi, 'settings').mockRejectedValueOnce(new Error('synthetic network error')).mockResolvedValueOnce(updated)
 const { client } = mount(); await submit()
 await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('Prøv igjen'))
 expect((screen.getByLabelText('Handicapandel (0–100 %)') as HTMLInputElement).value).toBe('50')
 fireEvent.click(screen.getByRole('button', { name: 'Lagre Stableford-innstillinger' }))
 await waitFor(() => expect(screen.getByRole('status').textContent).toContain('lagret'))
 expect(save).toHaveBeenCalledTimes(2)
 expect(client.getQueryData<Round>(tournamentKeys.round(session.user_id, round.id))).toEqual(updated)
})
