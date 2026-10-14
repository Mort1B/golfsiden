import { afterEach, expect, it, vi } from 'vitest'
import { api } from './client'
import { profileApi } from './profileRequests'
import { session } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { requestDecoded } from './http'
import { observeIdentity, waitForIdentity } from './identityQueue'

function deferred<T>() {
  let resolve: (value: T) => void = () => { throw new Error('Not initialized') }
  const promise = new Promise<T>(done => { resolve = done })
  return { promise, resolve }
}
const response = (body: unknown) => new Response(JSON.stringify(body), { status: 200 })
afterEach(async () => { await waitForIdentity(); vi.unstubAllGlobals(); vi.restoreAllMocks() })

it('dispatches the newer login only after the first cookie response is consumed', async () => {
  const first = deferred<Response>()
  const fetch = vi.fn().mockReturnValueOnce(first.promise).mockResolvedValue(response({ ...session, csrf_token: 'newer' }))
  vi.stubGlobal('fetch', fetch)
  const old = api.login('older', 'password')
  await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(1))
  const newer = api.login('newer', 'password')
  await Promise.resolve(); await Promise.resolve()
  expect(fetch).toHaveBeenCalledTimes(1)
  first.resolve(response(session))
  await expect(old).resolves.toEqual(session)
  await expect(newer).resolves.toMatchObject({ csrf_token: 'newer' })
})

it('releases queued login after an earlier malformed cookie-bearing reply', async () => {
  const first = deferred<Response>()
  const fetch = vi.fn().mockReturnValueOnce(first.promise).mockResolvedValue(response(session))
  vi.stubGlobal('fetch', fetch)
  const old = api.login('older', 'password').catch(error => error)
  await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(1))
  const newer = api.login('newer', 'password')
  first.resolve(response({ invalid: true }))
  expect(await old).toBeInstanceOf(Error)
  await expect(newer).resolves.toEqual(session)
})

it('resolves queued logout against the cookie created by an earlier request', async () => {
  const first = deferred<Response>()
  const fetch = vi.fn().mockReturnValueOnce(first.promise).mockImplementation(async (path: string) =>
    path.endsWith('/session') ? response(session) : new Response(null, { status: 204 }))
  vi.stubGlobal('fetch', fetch)
  const login = api.login('older', 'password')
  await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(1))
  const logout = api.logout('obsolete-csrf')
  first.resolve(response(session))
  await login; await logout
  expect(fetch).toHaveBeenLastCalledWith('/api/auth/logout', expect.objectContaining({
    headers: { 'x-csrf-token': session.csrf_token },
  }))
})

it('does not dispatch a queued password change using a renewed or different session', async () => {
  const first = deferred<Response>()
  const fetch = vi.fn().mockReturnValueOnce(first.promise).mockResolvedValue(response({ ...session, csrf_token: 'renewed' }))
  vi.stubGlobal('fetch', fetch)
  const login = api.login('newer', 'password')
  await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(1))
  const password = profileApi.password(1, 'new password', 'old password', session.csrf_token).catch(error => error)
  first.resolve(response({ ...session, csrf_token: 'renewed' }))
  await login
  expect(await password).toBeInstanceOf(Error)
  expect(fetch.mock.calls.some(call => call[0] === '/api/me/profile/password')).toBe(false)
})

it('rejects stale private requests initiated during an identity wait and releases newly owned reads', async () => {
  const first = deferred<Response>()
  const fetch = vi.fn().mockReturnValueOnce(first.promise).mockResolvedValue(response(session))
  vi.stubGlobal('fetch', fetch)
  const stop = observeIdentity({ started: () => {}, settled: async () => {} })
  try {
    const login = api.login('newer', 'password')
    await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(1))
    const staleRead = requestDecoded('/private-old-owner', value => value).catch(error => error)
    const publicRead = requestDecoded('/public-capability', value => value, { credentials: 'omit' })
    await publicRead
    expect(fetch.mock.calls.some(call => call[0] === '/public-capability')).toBe(true)
    expect(fetch.mock.calls.some(call => call[0] === '/private-old-owner')).toBe(false)
    first.resolve(response(session)); await login; await waitForIdentity()
    expect(await staleRead).toBeInstanceOf(Error)
    expect(fetch.mock.calls.some(call => call[0] === '/private-old-owner')).toBe(false)
    await requestDecoded('/private-new-owner', value => value)
    expect(fetch.mock.calls.some(call => call[0] === '/private-new-owner')).toBe(true)
  } finally { stop() }
})

it('rejects an old private response whose body arrives after a newer identity intent', async () => {
  const body = deferred<unknown>()
  const fetch = vi.fn().mockResolvedValueOnce({ ok: true, json: () => body.promise }).mockResolvedValue(response(session))
  vi.stubGlobal('fetch', fetch)
  const staleRead = requestDecoded('/private-old-owner', value => value).catch(error => error)
  await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(1))
  const login = api.login('newer', 'password')
  body.resolve({ private: 'old' })
  expect(await staleRead).toBeInstanceOf(Error)
  await login
})

it('rejects an old-owner read started while a failed writer is reconciling its cookie', async () => {
  const entered = deferred<void>()
  const release = deferred<void>()
  const fetch = vi.fn().mockRejectedValueOnce(new TypeError('lost cookie-bearing reply')).mockResolvedValue(response(session))
  vi.stubGlobal('fetch', fetch)
  const stop = observeIdentity({ started: () => {}, settled: async () => {
    entered.resolve(); await release.promise
  } })
  try {
    await expect(api.login('newer', 'password')).rejects.toThrow('lost cookie-bearing reply')
    await entered.promise
    const oldOwner = requestDecoded('/private-old-owner', value => value).catch(error => error)
    expect(fetch).toHaveBeenCalledTimes(1)
    release.resolve(); await waitForIdentity()
    expect(await oldOwner).toBeInstanceOf(Error)
    expect(fetch.mock.calls.some(call => call[0] === '/private-old-owner')).toBe(false)
    await requestDecoded('/private-new-owner', value => value)
    expect(fetch.mock.calls.some(call => call[0] === '/private-new-owner')).toBe(true)
  } finally { release.resolve(); stop() }
})
