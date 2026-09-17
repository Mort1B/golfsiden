// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { AuthContext, type AuthContextValue } from '../../auth/authContext'
import { session } from '../../tournaments/lifecycle/__tests__/fixtures'
import { ScoreQueueProvider } from '../offline/ScoreQueueProvider'
import { queueDatabase, STORAGE_ERROR } from '../offline/database'
import { useScoreDrafts } from './context'
import { useFourBallSync } from '../fourBall/useFourBallSync'
import { useStablefordSync } from '../stableford/useStablefordSync'
import { fourBallFixture } from '../../../api/fourBall/fixtures'
import { stablefordFixture } from '../../../api/stableford/fixtures'
import { ScoringGuardProvider } from '../ScoringGuardProvider'
import { useScoringGuard } from '../scoringGuardContext'
import { useEffect } from 'react'

const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
const client = new QueryClient()
beforeEach(() => { vi.stubGlobal('indexedDB', new IDBFactory()); vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(false) })
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks(); vi.unstubAllGlobals() })
function DraftProbe() { const { drafts } = useScoreDrafts(); return <output data-testid="drafts">{drafts.map(draft => `${draft.kind}:${draft.kind === 'legacy' ? draft.value : draft.value.type}`).join(',')}</output> }
function Inputs() {
  const four = fourBallFixture(), stable = stablefordFixture()
  const hole = four.holes[0], stableHole = stable.holes[0]
  if (!hole || !stableHole) throw new Error('fixture')
  const first = useFourBallSync(four, hole, four.partners[0].player_id, 'tournament')
  const second = useFourBallSync(four, hole, four.partners[1].player_id, 'tournament')
  const sf = useStablefordSync({ ...stable, round_id: 'stable-round' }, stableHole, 'tournament')
  return <><button disabled={!first.storageReady} onClick={() => first.setInput({ type: 'numeric', gross_strokes: 5 })}>First</button>
    <button onClick={() => second.setInput({ type: 'no_score' })}>Second</button><button onClick={() => sf.setInput({ type: 'no_score' })}>Stableford</button></>
}
it('retains both partners and Stableford through unmount and same-account CSRF rotation; hides real account changes', async () => {
  vi.spyOn(queueDatabase, 'enqueueFourBall').mockRejectedValue(new Error(STORAGE_ERROR))
  vi.spyOn(queueDatabase, 'enqueueStableford').mockRejectedValue(new Error(STORAGE_ERROR))
  const tree = (value: AuthContextValue, mounted: boolean) => <QueryClientProvider client={client}><AuthContext value={value}><ScoreQueueProvider><DraftProbe />{mounted && <Inputs />}</ScoreQueueProvider></AuthContext></QueryClientProvider>
  const view = render(tree(auth, true))
  await waitFor(() => expect(screen.getByText('First').hasAttribute('disabled')).toBe(false))
  fireEvent.click(screen.getByText('Second')); fireEvent.click(screen.getByText('First')); fireEvent.click(screen.getByText('Stableford'))
  await waitFor(() => expect(queueDatabase.enqueueStableford).toHaveBeenCalled())
  view.rerender(tree(auth, false))
  expect(screen.getByTestId('drafts').textContent).toBe('four_ball:no_score,four_ball:numeric,stableford:no_score')
  view.rerender(tree({ ...auth, session: { ...session, csrf_token: 'rotated' } }, false))
  expect(screen.getByTestId('drafts').textContent).toContain('stableford:no_score')
  view.rerender(tree({ ...auth, session: { ...session, user_id: 'another-account' } }, false))
  expect(screen.getByTestId('drafts').textContent).toBe('')
  view.rerender(tree(auth, false))
  expect(screen.getByTestId('drafts').textContent).toBe('')
})
function GuardOwner() { const { setBlocked } = useScoringGuard(); useEffect(() => { setBlocked(true); return () => setBlocked(false) }, [setBlocked]); return null }
function GuardProbe() { const { blocked } = useScoringGuard(); return <output>{String(blocked)}</output> }
it('one guard owner unmount cannot release another owner', async () => {
  const tree = (second: boolean) => <ScoringGuardProvider><GuardProbe /><GuardOwner />{second && <GuardOwner />}</ScoringGuardProvider>
  const view = render(tree(true)); expect(screen.getByText('true')).toBeTruthy()
  await act(async () => view.rerender(tree(false)))
  expect(screen.getByText('true')).toBeTruthy()
})
it('late completion from the former account cannot clear the replacement account draft', async () => {
  let release = () => {}
  const pending = new Promise<void>(resolve => { release = resolve })
  vi.spyOn(queueDatabase, 'enqueueFourBall').mockImplementationOnce(() => pending).mockRejectedValue(new Error(STORAGE_ERROR))
  const tree = (value: AuthContextValue) => <QueryClientProvider client={client}><AuthContext value={value}><ScoreQueueProvider><DraftProbe /><Inputs /></ScoreQueueProvider></AuthContext></QueryClientProvider>
  const view = render(tree(auth))
  await waitFor(() => expect(screen.getByText('First').hasAttribute('disabled')).toBe(false))
  fireEvent.click(screen.getByText('First'))
  await waitFor(() => expect(queueDatabase.enqueueFourBall).toHaveBeenCalledOnce())
  view.rerender(tree({ ...auth, session: { ...session, user_id: 'replacement' } }))
  await waitFor(() => expect(screen.getByText('First').hasAttribute('disabled')).toBe(false))
  fireEvent.click(screen.getByText('Second'))
  await waitFor(() => expect(queueDatabase.enqueueFourBall).toHaveBeenCalledTimes(2))
  await act(async () => { release(); await pending })
  expect(screen.getByTestId('drafts').textContent).toBe('four_ball:no_score')
})
