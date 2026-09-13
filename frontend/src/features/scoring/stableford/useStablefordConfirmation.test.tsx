// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'
import { stablefordApi as api } from '../../../api/stableford'
import type { StablefordScoringCard as ScoringScorecard } from '../../../api/stableford'
import { stablefordFixture } from '../../../api/stableford/fixtures'
import { scoringKeys } from '../../../api/scorecards'
import { AuthContext, type AuthContextValue } from '../../auth/authContext'
import { round as draft, session } from '../../tournaments/lifecycle/__tests__/fixtures'
import { ScoreQueueProvider } from '../offline/ScoreQueueProvider'
import { queueDatabase } from '../offline/database'
import { useStablefordConfirmation } from './useStablefordConfirmation'

const round = { ...draft, status: 'open' as const }
const owner = { type: 'player' as const, id: session.player_id ?? '' }
const card = { ...stablefordFixture('pickup'), round_id: round.id, owner }
const baseAuth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
function mount() {
  let auth = baseAuth
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const onConfirmed = vi.fn()
  const input = () => ({ round, owner, card, csrfToken: auth.session?.csrf_token ?? null, tournamentId: round.tournament_id,
    onConfirmed, onTerminal: vi.fn() })
  const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}><AuthContext value={auth}><ScoreQueueProvider>{children}</ScoreQueueProvider></AuthContext></QueryClientProvider>
  const hook = renderHook(() => useStablefordConfirmation(input()), { wrapper })
  return { ...hook, onConfirmed, client, replaceSession: () => { auth = { ...baseAuth, session: { ...session, csrf_token: 'replacement-session' } }; hook.rerender() } }
}
beforeEach(() => { vi.stubGlobal('indexedDB', new IDBFactory()); vi.spyOn(api, 'scoring').mockResolvedValue(card) })
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.unstubAllGlobals() })

it('rejects a confirmation after a suspended tab outlives its cross-tab lease', async () => {
  let resolve: (value: ScoringScorecard) => void = () => undefined
  vi.mocked(api.scoring).mockImplementation(() => new Promise(done => { resolve = done }))
  const confirm = vi.spyOn(api, 'confirm')
  const hook = mount()
  await waitFor(() => expect(hook.result.current.blocked).toBe(false))
  act(() => { hook.result.current.confirm() })
  await waitFor(() => expect(api.scoring).toHaveBeenCalledOnce())
  const future = Date.now() + 21_000
  vi.spyOn(Date, 'now').mockReturnValue(future)
  await act(async () => { resolve(card) })
  await waitFor(() => expect(hook.result.current.errorMessage).toContain('tok for lang tid'))
  expect(confirm).not.toHaveBeenCalled()
})
it('freshly checks completeness and synchronously prevents duplicate confirmation', async () => {
  vi.mocked(api.scoring).mockResolvedValue({ ...card, complete: false, holes_scored: 17 })
  const confirm = vi.spyOn(api, 'confirm')
  const hook = mount()
  await waitFor(() => expect(hook.result.current.blocked).toBe(false))
  act(() => { hook.result.current.confirm(); hook.result.current.confirm() })
  await waitFor(() => expect(hook.result.current.errorMessage).toContain('komplett'))
  expect(api.scoring).toHaveBeenCalledOnce()
  expect(confirm).not.toHaveBeenCalled()
})
it('retains same-account pending edits and blocks confirmation even on an apparently complete card', async () => {
  await queueDatabase.enqueueStableford({ protocol: 'stableford_v1', accountId: session.user_id, roundId: round.id, tournamentId: round.tournament_id, owner,
    holeId: crypto.randomUUID(), holeNumber: 1 }, { type: 'numeric', gross_strokes: 5 }, { type: 'absent' })
  vi.spyOn(api, 'save').mockRejectedValue(new Error('offline'))
  const confirm = vi.spyOn(api, 'confirm')
  const hook = mount()
  await waitFor(async () => expect(await queueDatabase.list(session.user_id)).toHaveLength(1))
  act(() => { hook.result.current.confirm() })
  expect(hook.result.current.blocked).toBe(true)
  expect(confirm).not.toHaveBeenCalled()
})
it('ignores a late confirmation after replacement of the same accounts session', async () => {
  let resolve: (value: ScoringScorecard) => void = () => undefined
  const confirm = vi.spyOn(api, 'confirm').mockImplementation(() => new Promise(done => { resolve = done }))
  const hook = mount()
  await waitFor(() => expect(hook.result.current.blocked).toBe(false))
  act(() => hook.result.current.confirm())
  await waitFor(() => expect(confirm).toHaveBeenCalledOnce())
  act(() => hook.replaceSession())
  await act(async () => { resolve({ ...card, confirmed: true, confirmed_at: '2026-09-13T10:00:00Z' }) })
  expect(hook.onConfirmed).not.toHaveBeenCalled()
  expect(hook.result.current.confirming).toBe(false)
  expect(hook.client.getQueryData(scoringKeys.scoring(session.user_id, round.id, owner))).toEqual(card)
})
