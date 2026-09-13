import { QueryClient } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { authKeys, type AuthSession } from '../../api/auth'
import { api } from '../../api/client'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import { publishSessionTransition, resolveSessionTransition } from '../auth/sessionTransition'
import { session } from '../tournaments/lifecycle/__tests__/fixtures'
import { refreshRecoverySession } from './refreshRecoverySession'
afterEach(() => vi.restoreAllMocks())
it('preserves unrelated valid sessions and clears only an invalidated identity', async () => {
  const client = new QueryClient(); client.setQueryData(authKeys.session, session)
  const key = privateWorkspaceKeys.user(session.user_id); client.setQueryData(key, 'private')
  vi.spyOn(api, 'session').mockResolvedValueOnce(session).mockResolvedValueOnce(null)
  await refreshRecoverySession(client, session)
  expect(client.getQueryData(key)).toBe('private')
  await refreshRecoverySession(client, session)
  expect(client.getQueryData(authKeys.session)).toBeNull(); expect(client.getQueryData(key)).toBeUndefined()
})
it('does not overwrite a concurrent login while waiting for refreshed auth', async () => {
  const client = new QueryClient(); client.setQueryData(authKeys.session, session)
  let finish: (value: AuthSession | null) => void = () => { throw new Error('missing') }
  vi.spyOn(api, 'session').mockImplementation(() => new Promise(resolve => { finish = resolve }))
  const refresh = refreshRecoverySession(client, session)
  await vi.waitFor(() => expect(api.session).toHaveBeenCalled())
  const changed = { ...session, csrf_token: 'new-login' }; client.setQueryData(authKeys.session, changed)
  finish(null); await refresh
  expect(client.getQueryData(authKeys.session)).toEqual(changed)
})
it('cancelled pre-reset auth cannot publish or clear a newer session private data when its HTTP response arrives late', async () => {
  const client = new QueryClient(); client.setQueryData(authKeys.session, session)
  let finish: (value: AuthSession | null) => void = () => { throw new Error('missing') }
  const old = client.fetchQuery({ queryKey: authKeys.session, queryFn: ({ signal }) => resolveSessionTransition(client, () => new Promise(resolve => { finish = resolve }), signal) }).catch(() => undefined)
  await client.cancelQueries({ queryKey: authKeys.session })
  const next = { ...session, user_id: 'another', csrf_token: 'new-session' }
  client.setQueryData(authKeys.session, next)
  const key = privateWorkspaceKeys.user(next.user_id); client.setQueryData(key, 'new-private')
  finish(session); await old; await Promise.resolve()
  expect(client.getQueryData(authKeys.session)).toEqual(next); expect(client.getQueryData(key)).toBe('new-private')
})

it('an auth refetch started after recovery cancellation cannot replace a newer explicit login', async () => {
  const client = new QueryClient(); client.setQueryData(authKeys.session, session)
  await client.cancelQueries({ queryKey: authKeys.session })
  let finish: (value: AuthSession | null) => void = () => { throw new Error('missing') }
  const pending = client.fetchQuery({ queryKey: authKeys.session, queryFn: ({ signal }) => resolveSessionTransition(client, () => new Promise(resolve => { finish = resolve }), signal) })
  const next = { ...session, user_id: 'new-login', csrf_token: 'new-login-token' }
  publishSessionTransition(client, next)
  const key = privateWorkspaceKeys.user(next.user_id); client.setQueryData(key, 'new-private')
  finish(null); await pending
  expect(client.getQueryData(authKeys.session)).toEqual(next)
  expect(client.getQueryData(key)).toBe('new-private')
})
