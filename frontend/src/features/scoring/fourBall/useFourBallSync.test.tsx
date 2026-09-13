// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'
import { AuthContext, type AuthContextValue } from '../../auth/authContext'
import { session } from '../../tournaments/lifecycle/__tests__/fixtures'
import { fourBallFixture } from '../../../api/fourBall/fixtures'
import { ScoreQueueProvider } from '../offline/ScoreQueueProvider'
import { queueDatabase, STORAGE_ERROR } from '../offline/database'
import { useScoreQueue } from '../offline/context'
import { useFourBallSync } from './useFourBallSync'
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
beforeEach(() => vi.stubGlobal('indexedDB', new IDBFactory()))
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.unstubAllGlobals() })
it('keeps unsaved pickup guarded through total storage loss and releases the guard on deliberate discard', async () => {
  const card = fourBallFixture(), hole = card.holes[0]
  if (!hole) throw new Error('fixture')
  const client = new QueryClient()
  const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}><AuthContext value={auth}><ScoreQueueProvider>{children}</ScoreQueueProvider></AuthContext></QueryClientProvider>
  const hook = renderHook(() => ({ sync: useFourBallSync(card, hole, card.partners[1].player_id, crypto.randomUUID()), queue: useScoreQueue() }), { wrapper })
  await waitFor(() => expect(hook.result.current.sync.storageReady).toBe(true))
  vi.spyOn(queueDatabase, 'enqueueFourBall').mockRejectedValue(new Error(STORAGE_ERROR))
  act(() => hook.result.current.sync.setInput({ type: 'no_score' }))
  await waitFor(() => expect(hook.result.current.sync.error).toBe(STORAGE_ERROR))
  vi.spyOn(queueDatabase, 'list').mockRejectedValue(new Error(STORAGE_ERROR))
  await act(async () => hook.result.current.queue.runtime.wake())
  expect(hook.result.current.sync.storageReady).toBe(false)
  expect(hook.result.current.sync.navigationLocked).toBe(true)
  expect(hook.result.current.sync.desired).toEqual({ type: 'no_score' })
  act(() => hook.result.current.sync.discard())
  expect(hook.result.current.sync.navigationLocked).toBe(false)
  expect(hook.result.current.sync.desired).toBeNull()
  client.clear()
})
