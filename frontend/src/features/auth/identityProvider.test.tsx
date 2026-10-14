// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys, type AuthSession } from '../../api/auth'
import { waitForIdentity } from '../../api/identityQueue'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import { AuthProvider } from './AuthProvider'
import { useAuth } from './authContext'
import { session } from '../tournaments/lifecycle/__tests__/fixtures'

const response = (value: unknown) => new Response(JSON.stringify(value))
function deferred<T>() {
  let resolve: (value: T) => void = () => { throw new Error('Not initialized') }
  const promise = new Promise<T>(done => { resolve = done })
  return { promise, resolve }
}
let client: QueryClient
function Probe() {
  const auth = useAuth()
  return <>
    <output aria-label="identity">{auth.session?.username ?? 'anonymous'}</output>
    {auth.error && <p role="alert">{auth.error.message}</p>}
    <button onClick={() => void auth.signIn('older', 'password').catch(() => {})}>Older</button>
    <button onClick={() => void auth.signIn('newer', 'password').catch(() => {})}>Newer</button>
    <button onClick={() => void auth.signOut().catch(() => {})}>Logout</button>
    <button onClick={() => void auth.retry()}>Retry</button>
  </>
}
function tree() {
  return render(<QueryClientProvider client={client}><AuthProvider><Probe /></AuthProvider></QueryClientProvider>)
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  client.setQueryData(authKeys.session, null)
})
afterEach(async () => { cleanup(); await waitForIdentity(); client.clear(); vi.unstubAllGlobals() })

it.each(['login', 'logout'])('keeps only the latest deliberate %s after a held first cookie response', async choice => {
  const first = deferred<Response>()
  const second = deferred<Response>()
  const newer = { ...session, username: 'newer', csrf_token: 'newer-csrf' }
  let cookie: AuthSession | null = null
  const fetch = vi.fn(async (path: string, init?: RequestInit) => {
    if (path.endsWith('/login')) {
      const input: unknown = JSON.parse(String(init?.body))
      if (typeof input === 'object' && input !== null && 'username' in input && input.username === 'older') return first.promise
      return second.promise
    }
    if (path.endsWith('/session')) return response(cookie)
    if (path.endsWith('/logout')) {
      expect(init?.headers).toEqual({ 'x-csrf-token': session.csrf_token })
      cookie = null
      return new Response(null, { status: 204 })
    }
    throw new Error('Unexpected request')
  })
  vi.stubGlobal('fetch', fetch)
  tree()
  const privateKey = privateWorkspaceKeys.user(session.user_id)
  client.setQueryData(privateKey, { secret: 'old' })
  fireEvent.click(screen.getByText('Older'))
  await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1))
  fireEvent.click(screen.getByText(choice === 'login' ? 'Newer' : 'Logout'))
  expect(fetch).toHaveBeenCalledTimes(1)
  await act(async () => { cookie = session; first.resolve(response(session)) })
  expect(client.getQueryData(authKeys.session)).toBeNull()
  if (choice === 'login') {
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(2))
    await act(async () => { cookie = newer; second.resolve(response(newer)) })
    await waitFor(() => expect(client.getQueryData(authKeys.session)).toEqual(newer))
    expect(cookie).toEqual(newer)
  } else {
    await waitFor(() => expect(fetch.mock.calls.some(call => call[0].endsWith('/logout'))).toBe(true))
    await waitForIdentity()
    expect(cookie).toBeNull()
    expect(client.getQueryData(authKeys.session)).toBeNull()
  }
  expect(client.getQueryData(privateKey)).toBeUndefined()
})

it('reconciles a cookie-bearing failed login without replaying the write and exposes a failed read for deliberate retry', async () => {
  const recovered = { ...session, username: 'recovered' }
  let offline = true
  const fetch = vi.fn(async (path: string) => {
    if (path.endsWith('/login')) throw new TypeError('lost response')
    if (offline) throw new TypeError('session unavailable')
    return response(recovered)
  })
  vi.stubGlobal('fetch', fetch)
  tree(); fireEvent.click(screen.getByText('Older'))
  await screen.findByRole('alert')
  expect(client.getQueryData(authKeys.session)).toBeNull()
  expect(fetch.mock.calls.filter(call => call[0].endsWith('/login'))).toHaveLength(1)
  offline = false
  fireEvent.click(screen.getByText('Retry'))
  await waitFor(() => expect(client.getQueryData(authKeys.session)).toEqual(recovered))
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull())
  expect(fetch.mock.calls.filter(call => call[0].endsWith('/login'))).toHaveLength(1)
})
