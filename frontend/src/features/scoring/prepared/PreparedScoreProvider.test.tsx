// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { PreparedScoreProvider } from './PreparedScoreProvider'
import { usePreparedScore } from './context'
import { eligiblePrepared, preparedKeys } from './eligibility'
import { AuthContext, type AuthContextValue } from '../../auth/authContext'
import { authKeys } from '../../../api/auth'
import { ApiHttpError } from '../../../api/http'
import { scoringKeys } from '../../../api/scorecards'
import { tournamentKeys } from '../../../api/tournaments'
import { privateWorkspaceKeys } from '../../../api/privateWorkspace'
import { handleTournamentLiveSignal } from '../../../api/liveInvalidation'
import { stablefordFixture } from '../../../api/stableford/fixtures'
import { round as initialRound, tournament, session as initialSession, completion } from '../../tournaments/lifecycle/__tests__/fixtures'
const session = { ...initialSession, expires_at: '2099-01-01T00:00:00Z' }
const card = stablefordFixture(), round = { ...initialRound, id: card.round_id, status: 'open' as const, scoring_format: 'individual_stableford' as const }
const target = { tournamentId: tournament.id, roundId: round.id, owner: card.owner, holeNumber: 1 }
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
let client: QueryClient
function seed() {
  client.setQueryData(authKeys.session, session)
  client.setQueryData(tournamentKeys.list(session.user_id), [tournament])
  client.setQueryData(tournamentKeys.rounds(session.user_id, tournament.id), [round])
  client.setQueryData(privateWorkspaceKeys.completion(session.user_id, round.id), { ...completion(), round_id: round.id, status: 'open' })
  client.setQueryData(privateWorkspaceKeys.scoreAccess(session.user_id, round.id), { round_id: round.id, writable_owners: [card.owner] })
  client.setQueryData(scoringKeys.scoring(session.user_id, round.id, card.owner), card)
}
function Probe() {
  const state = usePreparedScore()
  return <><output>{state.prepared?.holeNumber ?? 'unavailable'}</output><button onClick={() => state.prepare(target)}>Visit</button>
    <button onClick={() => state.prepare({ ...target, holeNumber: 2 })}>Next</button></>
}
function mount() {
  const tree = (value: AuthContextValue) => <QueryClientProvider client={client}><AuthContext value={value}><PreparedScoreProvider><Probe /></PreparedScoreProvider></AuthContext></QueryClientProvider>
  const view = render(tree(auth))
  return (value: AuthContextValue) => view.rerender(tree(value))
}
beforeEach(() => { client = new QueryClient({ defaultOptions: { queries: { retry: false } } }); seed() })
afterEach(() => { cleanup(); client.clear(); vi.useRealTimers() })
describe('prepared card ownership', () => {
  it('prepares after initial asynchronous login without requiring a second auth read', () => {
    const tree = (value: AuthContextValue) => <QueryClientProvider client={client}><AuthContext value={value}><PreparedScoreProvider><Probe /></PreparedScoreProvider></AuthContext></QueryClientProvider>
    const view = render(tree({ ...auth, session: null }))
    view.rerender(tree(auth))
    fireEvent.click(screen.getByText('Visit')); expect(screen.getByRole('status').textContent).toBe('1')
  })
  it('requires an actual visit; prefetched data alone does not prepare a card', () => {
    mount(); expect(screen.getByRole('status').textContent).toBe('unavailable')
    fireEvent.click(screen.getByText('Visit')); expect(screen.getByRole('status').textContent).toBe('1')
  })
  it('remembers hole moves while completion projection is cleared', async () => {
    mount(); fireEvent.click(screen.getByText('Visit'))
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'error'))
    fireEvent.click(screen.getByText('Next')); expect(screen.getByRole('status').textContent).toBe('2')
  })
  it.each([401, 403, 404])('keeps observed %s revoked through same-turn SSE clearing and cached revisit', async status => {
    mount(); fireEvent.click(screen.getByText('Visit'))
    await act(async () => {
      client.getQueryCache().find({ queryKey: privateWorkspaceKeys.completion(session.user_id, round.id) })?.setState({ status: 'error', error: new ApiHttpError(status, 'denied', 'Denied') })
      await handleTournamentLiveSignal(client, session.user_id, 'error')
    })
    fireEvent.click(screen.getByText('Visit')); expect(screen.getByRole('status').textContent).toBe('unavailable')
  })
  it.each(['list', 'detail', 'completion'])('rejects a fresh locked %s despite stale open metadata elsewhere', source => {
    mount(); fireEvent.click(screen.getByText('Visit'))
    act(() => {
      if (source === 'list') client.setQueryData(tournamentKeys.rounds(session.user_id, tournament.id), [{ ...round, status: 'locked' }])
      else if (source === 'detail') client.setQueryData(tournamentKeys.round(session.user_id, round.id), { ...round, status: 'locked' })
      else client.setQueryData(privateWorkspaceKeys.completion(session.user_id, round.id), { ...completion(), status: 'locked' })
    })
    expect(screen.getByRole('status').textContent).toBe('unavailable')
  })
  it.each(['card', 'access'])('cache eviction of %s invalidates eligibility permanently until fresh loading', source => {
    mount(); fireEvent.click(screen.getByText('Visit'))
    act(() => client.removeQueries({ queryKey: source === 'card' ? scoringKeys.scoring(session.user_id, round.id, card.owner) : privateWorkspaceKeys.scoreAccess(session.user_id, round.id), exact: true }))
    expect(screen.getByRole('status').textContent).toBe('unavailable')
  })
  it('rejects a fresh access response omitting the exact owner', () => {
    mount(); fireEvent.click(screen.getByText('Visit'))
    act(() => client.setQueryData(privateWorkspaceKeys.scoreAccess(session.user_id, round.id), { round_id: round.id, writable_owners: [] }))
    expect(screen.getByRole('status').textContent).toBe('unavailable')
  })
  it.each(['logout', 'account', 'csrf'])('clears eligibility on %s without remounting children', change => {
    const rerender = mount(); fireEvent.click(screen.getByText('Visit'))
    const next = change === 'logout' ? null : change === 'account' ? { ...session, user_id: crypto.randomUUID() } : { ...session, csrf_token: 'replacement' }
    act(() => { client.setQueryData(authKeys.session, next); rerender({ ...auth, session: next }) })
    expect(screen.getByRole('status').textContent).toBe('unavailable')
    fireEvent.click(screen.getByText('Visit')); expect(screen.getByRole('status').textContent).toBe('unavailable')
  })
  it('expires while an offline action is visible and activation cannot use it', () => {
    vi.useFakeTimers(); vi.setSystemTime(new Date('2098-12-31T23:59:59Z'))
    mount(); fireEvent.click(screen.getByText('Visit')); expect(screen.getByRole('status').textContent).toBe('1')
    act(() => vi.advanceTimersByTime(1002))
    expect(screen.getByRole('status').textContent).toBe('unavailable')
    expect(eligiblePrepared(client, session, target)).toBe(false)
  })
  it('rejects absent holes, wrong owners and read-only cards', () => {
    expect(eligiblePrepared(client, session, { ...target, holeNumber: 99 })).toBe(false)
    expect(eligiblePrepared(client, session, { ...target, owner: { type: 'player', id: crypto.randomUUID() } })).toBe(false)
    client.setQueryData(scoringKeys.scoring(session.user_id, round.id, card.owner), { ...card, projection: 'read' })
    expect(eligiblePrepared(client, session, target)).toBe(false)
  })
  it('permits fresh successful loading after a denial but cannot rearm unchanged authority', async () => {
    mount(); fireEvent.click(screen.getByText('Visit'))
    act(() => client.getQueryCache().find({ queryKey: privateWorkspaceKeys.scoreAccess(session.user_id, round.id) })?.setState({ status: 'error', error: new ApiHttpError(403, 'denied', 'Denied') }))
    act(() => { seed(); for (const key of preparedKeys(session.user_id, target)) client.getQueryCache().find({ queryKey: key })?.setState({ dataUpdatedAt: Date.now() + 10 }) })
    fireEvent.click(screen.getByText('Visit')); expect(screen.getByRole('status').textContent).toBe('1')
  })
})
