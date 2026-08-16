import { QueryClient, QueryObserver } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import type { AuthSession } from '../../api/auth'
import { authKeys } from '../../api/auth'
import { tournamentKeys } from '../../api/tournaments'
import { invalidateLiveQueries } from '../../api/liveInvalidation'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import { publishSessionTransition, resolveSessionTransition } from './sessionTransition'

const session = (userId: string): AuthSession => ({
  user_id: userId,
  username: userId,
  display_name: userId,
  role: 'viewer',
  player_id: null,
  expires_at: '2026-08-16T18:00:00Z',
  csrf_token: `csrf-${userId}`,
})

describe('AuthProvider private cache transitions', () => {
  it('clears predecessor data before a restored session can start its first protected query', async () => {
    const queryClient = new QueryClient()
    queryClient.setQueryData(tournamentKeys.detail('old-user', 'old-tour'), { name: 'Old' })

    const restored = await queryClient.fetchQuery({
      queryKey: authKeys.session,
      queryFn: () => resolveSessionTransition(queryClient, async () => session('restored-user')),
    })
    expect(restored).not.toBeNull()
    if (!restored) throw new Error('Expected a restored session')
    const protectedResult = await queryClient.fetchQuery({
      queryKey: tournamentKeys.detail(restored.user_id, 'restored-tour'),
      queryFn: async () => ({ name: 'Restored' }),
    })

    expect(protectedResult).toEqual({ name: 'Restored' })
    expect(queryClient.getQueryData(tournamentKeys.detail('old-user', 'old-tour'))).toBeUndefined()
    expect(queryClient.getQueryState(tournamentKeys.detail(restored.user_id, 'restored-tour'))?.status)
      .toBe('success')
  })

  it('removes old-user and in-flight current-user keys before publishing a changed session', () => {
    const queryClient = new QueryClient()
    queryClient.setQueryData(authKeys.session, session('old-user'))
    queryClient.setQueryData(tournamentKeys.list('old-user'), [{ id: 'old-tour' }])
    queryClient.setQueryData(tournamentKeys.round('new-user', 'stale-round'), { id: 'stale-round' })

    publishSessionTransition(queryClient, session('new-user'))

    expect(queryClient.getQueriesData({ queryKey: ['private-workspace'] })).toEqual([])
    expect(queryClient.getQueryData<AuthSession>(authKeys.session)?.user_id).toBe('new-user')
  })

  it('keeps an observed protected query mounted and cached across same-user auth and SSE refreshes', async () => {
    const queryClient = new QueryClient()
    const current = session('same-user')
    const protectedKey = privateWorkspaceKeys.completion(current.user_id, 'current-round')
    queryClient.setQueryData(authKeys.session, current)
    queryClient.setQueryData(protectedKey, { name: 'Current' })
    let authFetches = 0
    let protectedFetches = 0
    const authObserver = new QueryObserver(queryClient, {
      queryKey: authKeys.session,
      queryFn: async () => {
        authFetches += 1
        return session('same-user')
      },
      staleTime: Number.POSITIVE_INFINITY,
    })
    const observer = new QueryObserver(queryClient, {
      queryKey: protectedKey,
      queryFn: async () => {
        protectedFetches += 1
        return { name: 'Current' }
      },
      staleTime: Number.POSITIVE_INFINITY,
    })
    const unsubscribeAuth = authObserver.subscribe(() => undefined)
    const unsubscribe = observer.subscribe(() => undefined)

    const refreshed = await resolveSessionTransition(queryClient, async () => session('same-user'))
    queryClient.setQueryData(authKeys.session, refreshed)
    await invalidateLiveQueries(queryClient)

    expect(queryClient.getQueryData(protectedKey)).toEqual({ name: 'Current' })
    expect(queryClient.getQueryCache().find({ queryKey: protectedKey })?.getObserversCount()).toBe(1)
    expect(observer.getCurrentResult().status).toBe('success')
    expect(authFetches).toBe(0)
    expect(protectedFetches).toBe(1)
    unsubscribeAuth()
    unsubscribe()
  })

  it('clears private data before publishing a null session', () => {
    const queryClient = new QueryClient()
    queryClient.setQueryData(authKeys.session, session('signed-in-user'))
    queryClient.setQueryData(tournamentKeys.list('signed-in-user'), [{ id: 'private-tour' }])

    publishSessionTransition(queryClient, null)

    expect(queryClient.getQueriesData({ queryKey: ['private-workspace'] })).toEqual([])
    expect(queryClient.getQueryData(authKeys.session)).toBeNull()
  })
})
