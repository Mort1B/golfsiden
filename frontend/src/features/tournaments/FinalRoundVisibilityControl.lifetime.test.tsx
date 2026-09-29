// @vitest-environment jsdom
import { StrictMode } from 'react'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { FinalRoundVisibilityControl } from './FinalRoundVisibilityControl'
import { round, session, tournament } from './lifecycle/__tests__/fixtures'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { publishSessionTransition } from '../auth/sessionTransition'
import { authKeys, type AuthSession } from '../../api/auth'
import { finalRoundVisibilityApi as api, finalRoundVisibilityKeys as keys, type FinalRoundVisibility } from '../../api/finalRoundVisibility'
import { ApiHttpError } from '../../api/http'

const hidden: FinalRoundVisibility = { tournament_id: tournament.id, back_nine_hidden: true, visibility_updated_at: tournament.updated_at }
const released = { ...hidden, back_nine_hidden: false, visibility_updated_at: '2026-09-07T11:00:00Z' }
const clients: QueryClient[] = []
afterEach(() => { cleanup(); clients.splice(0).forEach(client => client.clear()); vi.restoreAllMocks() })
function deferred<T>() {
  let resolve: (value: T) => void = () => undefined
  let reject: (reason: unknown) => void = () => undefined
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
function mount() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity }, mutations: { retry: false } } })
  clients.push(client)
  client.setQueryData(authKeys.session, session)
  client.setQueryData(keys.detail(session.user_id, tournament.id), hidden)
  const get = vi.spyOn(api, 'get').mockImplementation(async id => ({ ...hidden, tournament_id: id }))
  const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
  const tree = (identity: AuthSession | null, target = tournament, mounted = true, finalRound = round) => (
    <StrictMode><QueryClientProvider client={client}><AuthContext value={{ ...auth, session: identity }}>
      {mounted && <FinalRoundVisibilityControl tournament={target} finalRound={finalRound} />}
    </AuthContext></QueryClientProvider></StrictMode>
  )
  const view = render(tree(session))
  return { client, get, change: (identity: AuthSession | null, target = tournament, mounted = true, finalRound = round) => {
    publishSessionTransition(client, identity)
    view.rerender(tree(identity, target, mounted, finalRound))
  } }
}
async function submit() {
  fireEvent.click(await screen.findByRole('switch', { name: 'Frigi hull 10–18' }))
}
async function settled(client: QueryClient) {
  await waitFor(() => expect(client.getMutationCache().getAll().some(m => m.state.status === 'pending')).toBe(false))
}
for (const transition of ['logout', 'account', 'csrf', 'unmount', 'tournament', 'round'] as const) {
  for (const outcome of ['success', 'stale', 'failure'] as const) it(`ignores late ${outcome} after ${transition}`, async () => {
    const pending = deferred<FinalRoundVisibility>()
    const save = vi.spyOn(api, 'update').mockReturnValue(pending.promise)
    const { client, get, change } = mount()
    await submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
    const next = transition === 'logout' ? null : transition === 'account' ? { ...session, user_id: crypto.randomUUID(), csrf_token: 'other' }
      : transition === 'csrf' ? { ...session, csrf_token: 'renewed' } : session
    await act(async () => change(next, transition === 'tournament' ? { ...tournament, id: crypto.randomUUID() } : tournament,
      !['logout', 'unmount'].includes(transition), transition === 'round' ? { ...round, id: crypto.randomUUID() } : round))
    const before = client.getQueriesData({ queryKey: ['private-workspace'] })
    const writes = vi.spyOn(client, 'setQueryData'), refresh = vi.spyOn(client, 'invalidateQueries')
    const reads = get.mock.calls.length
    await act(async () => {
      if (outcome === 'success') pending.resolve(released)
      else pending.reject(outcome === 'stale' ? new ApiHttpError(409, 'final_round_visibility_stale', 'synthetic conflict') : new Error('synthetic failure'))
    })
    await settled(client)
    expect(writes).not.toHaveBeenCalled(); expect(refresh).not.toHaveBeenCalled()
    expect(get).toHaveBeenCalledTimes(reads)
    expect(client.getQueriesData({ queryKey: ['private-workspace'] })).toEqual(before)
    expect(screen.queryByText(/Serverstatusen er bekreftet/)).toBeNull()
    expect(screen.queryByRole('alert')).toBeNull()
  })
}
for (const replacement of [null, { ...session, csrf_token: 'renewed' }]) it(`rejects late writes and submission after canonical ${replacement ? 'renewal' : 'logout'} before React rerenders`, async () => {
  const pending = deferred<FinalRoundVisibility>(), save = vi.spyOn(api, 'update').mockReturnValue(pending.promise)
  const { client } = mount()
  await submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
  // Isolate canonical publication from QueryObserver recreation after removal;
  // the transition matrix above separately covers actual private-cache clearing.
  act(() => client.setQueryData(authKeys.session, replacement))
  await act(async () => pending.resolve(released))
  await settled(client)
  expect(client.getQueryData(keys.detail(session.user_id, tournament.id))).toEqual(hidden)
  await submit()
  expect(save).toHaveBeenCalledOnce()
})
it('keeps a renewed session operation pending when the old response arrives', async () => {
  const old = deferred<FinalRoundVisibility>(), fresh = deferred<FinalRoundVisibility>()
  const save = vi.spyOn(api, 'update').mockReturnValueOnce(old.promise).mockReturnValueOnce(fresh.promise)
  const { client, change, get } = mount()
  await submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
  act(() => change({ ...session, csrf_token: 'renewed' }))
  expect(screen.getByRole('switch').hasAttribute('disabled')).toBe(false)
  await submit(); await waitFor(() => expect(save).toHaveBeenCalledTimes(2))
  await act(async () => old.resolve(released))
  expect(screen.getByRole('switch').hasAttribute('disabled')).toBe(true)
  expect(screen.queryByText(/Serverstatusen er bekreftet/)).toBeNull()
  get.mockResolvedValue(released)
  await act(async () => fresh.resolve(released))
  await settled(client)
  expect(screen.getByRole('switch')).toHaveProperty('checked', true)
  expect(screen.getByText('Hull 10–18 er frigitt. Serverstatusen er bekreftet.')).toBeTruthy()
  expect(save.mock.calls[1]?.[2]).toBe('renewed')
  expect(client.getQueryData(keys.detail(session.user_id, tournament.id))).toEqual(released)
})
it('preserves current-session failure, deliberate retry and visibility invalidation', async () => {
  const save = vi.spyOn(api, 'update').mockRejectedValueOnce(new Error('synthetic network error')).mockResolvedValueOnce(released)
  const { client, get } = mount()
  const invalidate = vi.spyOn(client, 'invalidateQueries')
  await submit()
  expect(await screen.findByRole('alert')).toHaveProperty('textContent', expect.stringContaining('synthetic network error'))
  get.mockResolvedValue(released)
  fireEvent.click(screen.getByRole('button', { name: 'Prøv lagring igjen' }))
  await screen.findByText('Hull 10–18 er frigitt. Serverstatusen er bekreftet.')
  expect(save).toHaveBeenCalledTimes(2)
  expect(invalidate).toHaveBeenCalledOnce()
  expect(client.getQueryData(keys.detail(session.user_id, tournament.id))).toEqual(released)
})
it('refreshes stale current-session state and allows a new versioned save on a locked final', async () => {
  const save = vi.spyOn(api, 'update').mockRejectedValueOnce(new ApiHttpError(409, 'final_round_visibility_stale', 'stale')).mockResolvedValueOnce(hidden)
  const { client, get, change } = mount()
  act(() => change(session, tournament, true, { ...round, status: 'locked' }))
  get.mockResolvedValue(released)
  await submit()
  await waitFor(() => expect(client.getQueryData(keys.detail(session.user_id, tournament.id))).toEqual(released))
  expect(await screen.findByRole('alert')).toHaveProperty('textContent', expect.stringContaining('endret et annet sted'))
  get.mockResolvedValue(hidden)
  await submit(); await settled(client)
  expect(save.mock.calls[1]?.[1]).toEqual({ back_nine_hidden: true, expected_visibility_updated_at: released.visibility_updated_at })
})
