// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'
import { fourBallApi } from '../../../api/fourBall'
import { fourBallFixture } from '../../../api/fourBall/fixtures'
import { scoringKeys } from '../../../api/scorecards'
import type { FourBallScoringCard } from '../../../api/fourBall'
import { AuthContext, type AuthContextValue } from '../../auth/authContext'
import { round as draft, session } from '../../tournaments/lifecycle/__tests__/fixtures'
import { ScoreQueueProvider } from '../offline/ScoreQueueProvider'
import { queueDatabase } from '../offline/database'
import { useFourBallConfirmation } from './useFourBallConfirmation'

const round = { ...draft, status: 'open' as const, scoring_format: 'four_ball_stroke_play' as const }
const card: FourBallScoringCard = { ...fourBallFixture(true), round_id: round.id }
const owner = card.owner
const baseAuth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
function mount() {
  let auth = baseAuth
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const onConfirmed = vi.fn()
  const input = () => ({ round, owner, card, csrfToken: auth.session?.csrf_token ?? null, tournamentId: round.tournament_id,
    onConfirmed, onTerminal: vi.fn(), acknowledgeBlanks: true })
  const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}><AuthContext value={auth}><ScoreQueueProvider>{children}</ScoreQueueProvider></AuthContext></QueryClientProvider>
  const hook = renderHook(() => useFourBallConfirmation(input()), { wrapper })
  return { ...hook, onConfirmed, client, replaceSession: () => { auth = { ...baseAuth, session: { ...session, csrf_token: 'replacement-session' } }; hook.rerender() } }
}
beforeEach(() => { vi.stubGlobal('indexedDB', new IDBFactory()); vi.spyOn(fourBallApi, 'scoring').mockResolvedValue(card) })
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.unstubAllGlobals() })

it('rejects a confirmation after a suspended tab outlives its cross-tab lease', async () => {
  let resolve: (value: FourBallScoringCard) => void = () => undefined
  vi.mocked(fourBallApi.scoring).mockImplementation(() => new Promise(done => { resolve = done }))
  const confirm = vi.spyOn(fourBallApi, 'confirm')
  const hook = mount()
  await waitFor(() => expect(hook.result.current.blocked).toBe(false))
  act(() => { hook.result.current.confirm() })
  await waitFor(() => expect(fourBallApi.scoring).toHaveBeenCalledOnce())
  const future = Date.now() + 21_000
  vi.spyOn(Date, 'now').mockReturnValue(future)
  await act(async () => { resolve(card) })
  await waitFor(() => expect(hook.result.current.errorMessage).toContain('tok for lang tid'))
  expect(confirm).not.toHaveBeenCalled()
})
it('freshly checks completeness and synchronously prevents duplicate confirmation', async () => {
  vi.mocked(fourBallApi.scoring).mockResolvedValue({ ...card, complete: false, holes_scored: 17 })
  const confirm = vi.spyOn(fourBallApi, 'confirm')
  const hook = mount()
  await waitFor(() => expect(hook.result.current.blocked).toBe(false))
  act(() => { hook.result.current.confirm(); hook.result.current.confirm() })
  await waitFor(() => expect(hook.result.current.errorMessage).toContain('komplett'))
  expect(fourBallApi.scoring).toHaveBeenCalledOnce()
  expect(confirm).not.toHaveBeenCalled()
})
it('retains same-account pending edits and blocks confirmation even on an apparently complete card', async () => {
  await queueDatabase.enqueueFourBall({ protocol: 'four_ball_v1', sideId: owner.id, accountId: session.user_id, roundId: round.id, tournamentId: round.tournament_id, owner: { type: 'player', id: card.partners[1].player_id },
    holeId: crypto.randomUUID(), holeNumber: 1 }, { type: 'numeric', gross_strokes: 5 }, { type: 'absent' })
  vi.spyOn(fourBallApi, 'save').mockRejectedValue(new Error('offline'))
  const confirm = vi.spyOn(fourBallApi, 'confirm')
  const hook = mount()
  await waitFor(async () => expect(await queueDatabase.list(session.user_id)).toHaveLength(1))
  act(() => { hook.result.current.confirm() })
  expect(hook.result.current.blocked).toBe(true)
  expect(confirm).not.toHaveBeenCalled()
})
it('ignores a late confirmation after replacement of the same accounts session', async () => {
  let resolve: (value: FourBallScoringCard) => void = () => undefined
  const confirm = vi.spyOn(fourBallApi, 'confirm').mockImplementation(() => new Promise(done => { resolve = done }))
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

it('blocks team confirmation while a delivered partner-B input awaits its authoritative side refresh', async () => {
  const hole = card.holes[0]; if (!hole) throw new Error('fixture')
  await queueDatabase.enqueueFourBall({ protocol: 'four_ball_v1', sideId: owner.id, accountId: session.user_id, roundId: round.id,
    tournamentId: round.tournament_id, owner: { type: 'player', id: card.partners[1].player_id }, holeId: hole.hole_id, holeNumber: 1 },
  { type: 'no_score' }, { type: 'absent' })
  vi.spyOn(fourBallApi, 'save').mockImplementation(async (_round, operation) => ({ request_id: operation.request_id, applied_score: { score_id: crypto.randomUUID(), revision: '1' } }))
  vi.mocked(fourBallApi.scoring).mockImplementation(() => new Promise(() => undefined))
  const confirm = vi.spyOn(fourBallApi, 'confirm')
  const hook = mount()
  await waitFor(() => expect(fourBallApi.scoring).toHaveBeenCalled())
  expect(await queueDatabase.list(session.user_id)).toEqual([])
  expect(hook.result.current.blocked).toBe(true)
  act(() => hook.result.current.confirm())
  expect(confirm).not.toHaveBeenCalled()
})
