// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'
import { api } from '../../api/client'
import { scoringKeys, type ScoringScorecard } from '../../api/scorecards'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { round as draft, session } from '../tournaments/lifecycle/__tests__/fixtures'
import { ScoreQueueProvider } from './offline/ScoreQueueProvider'
import { queueDatabase } from './offline/database'
import { useScorecardConfirmation } from './useScorecardConfirmation'

const round = { ...draft, status: 'open' as const }
const owner = { type: 'player' as const, id: session.player_id ?? '' }
const card: ScoringScorecard = { projection: 'scoring', round_id: round.id, owner, holes: [], gross_total: 72,
  net_total: 72, playing_handicap: 0, holes_scored: 18, number_of_holes: 18,
  complete: true, confirmed: false, confirmed_at: null, confirmed_by: null }
const baseAuth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
function mount() {
  let auth = baseAuth
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const onConfirmed = vi.fn()
  const input = () => ({ round, owner, card, csrfToken: auth.session?.csrf_token ?? null, tournamentId: round.tournament_id,
    onConfirmed, onTerminal: vi.fn() })
  const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}><AuthContext value={auth}><ScoreQueueProvider>{children}</ScoreQueueProvider></AuthContext></QueryClientProvider>
  const hook = renderHook(() => useScorecardConfirmation(input()), { wrapper })
  return { ...hook, onConfirmed, client, replaceSession: () => { auth = { ...baseAuth, session: { ...session, csrf_token: 'replacement-session' } }; hook.rerender() } }
}
beforeEach(() => { vi.stubGlobal('indexedDB', new IDBFactory()); vi.spyOn(api, 'scorecardScoring').mockResolvedValue(card) })
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.unstubAllGlobals() })

it('rejects a confirmation after a suspended tab outlives its cross-tab lease', async () => {
  let resolve: (value: ScoringScorecard) => void = () => undefined
  vi.mocked(api.scorecardScoring).mockImplementation(() => new Promise(done => { resolve = done }))
  const confirm = vi.spyOn(api, 'confirmScorecard')
  const hook = mount()
  await waitFor(() => expect(hook.result.current.blocked).toBe(false))
  act(() => { hook.result.current.confirm() })
  await waitFor(() => expect(api.scorecardScoring).toHaveBeenCalledOnce())
  const future = Date.now() + 21_000
  vi.spyOn(Date, 'now').mockReturnValue(future)
  await act(async () => { resolve(card) })
  await waitFor(() => expect(hook.result.current.errorMessage).toContain('tok for lang tid'))
  expect(confirm).not.toHaveBeenCalled()
})
it('freshly checks completeness and synchronously prevents duplicate confirmation', async () => {
  vi.mocked(api.scorecardScoring).mockResolvedValue({ ...card, complete: false, holes_scored: 17 })
  const confirm = vi.spyOn(api, 'confirmScorecard')
  const hook = mount()
  await waitFor(() => expect(hook.result.current.blocked).toBe(false))
  act(() => { hook.result.current.confirm(); hook.result.current.confirm() })
  await waitFor(() => expect(hook.result.current.errorMessage).toContain('komplett'))
  expect(api.scorecardScoring).toHaveBeenCalledOnce()
  expect(confirm).not.toHaveBeenCalled()
})
it('retains same-account pending edits and blocks confirmation even on an apparently complete card', async () => {
  await queueDatabase.enqueue({ accountId: session.user_id, roundId: round.id, tournamentId: round.tournament_id, owner,
    holeId: crypto.randomUUID(), holeNumber: 1 }, 5, { type: 'absent' })
  vi.spyOn(api, 'saveConditionalScore').mockRejectedValue(new Error('offline'))
  const confirm = vi.spyOn(api, 'confirmScorecard')
  const hook = mount()
  await waitFor(async () => expect(await queueDatabase.list(session.user_id)).toHaveLength(1))
  act(() => { hook.result.current.confirm() })
  expect(hook.result.current.blocked).toBe(true)
  expect(confirm).not.toHaveBeenCalled()
})
it('ignores a late confirmation after replacement of the same accounts session', async () => {
  let resolve: (value: ScoringScorecard) => void = () => undefined
  const confirm = vi.spyOn(api, 'confirmScorecard').mockImplementation(() => new Promise(done => { resolve = done }))
  const hook = mount()
  await waitFor(() => expect(hook.result.current.blocked).toBe(false))
  act(() => hook.result.current.confirm())
  await waitFor(() => expect(confirm).toHaveBeenCalledOnce())
  act(() => hook.replaceSession())
  await act(async () => { resolve({ ...card, confirmed: true, confirmed_by: session.user_id, confirmed_at: '2026-09-13T10:00:00Z' }) })
  expect(hook.onConfirmed).not.toHaveBeenCalled()
  expect(hook.result.current.confirming).toBe(false)
  expect(hook.client.getQueryData(scoringKeys.scoring(session.user_id, round.id, owner))).toEqual(card)
})
