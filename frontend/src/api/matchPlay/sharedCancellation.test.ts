import { QueryClient, QueryObserver } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { matchApi, matchKeys } from '../matchPlay'
import { loadPrivateResult } from '../privateResults'
import { matchIds } from './fixtures'
const value = { round_id: matchIds.round, matches: [], writable_match_ids: [] }
const scope = { userId: matchIds.user, roundId: matchIds.round }
const key = matchKeys.list(matchIds.user, matchIds.round)
afterEach(() => vi.unstubAllGlobals())

function setup() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } })
  let release: (response: Response) => void = () => undefined
  let signal: AbortSignal | null | undefined
  const fetch = vi.fn((_path: string, init?: RequestInit) => new Promise<Response>((resolve, reject) => {
    release = resolve; signal = init?.signal
    signal?.addEventListener('abort', () => reject(signal?.reason), { once: true })
  }))
  vi.stubGlobal('fetch', fetch)
  const results = new QueryObserver(client, { queryKey: key, queryFn: ({ signal }) =>
    loadPrivateResult(client, scope, key, signal, () => matchApi.list(matchIds.round, signal)) })
  const management = new QueryObserver(client, { queryKey: key, queryFn: () => matchApi.list(matchIds.round) })
  return { client, results, management, fetch, signal: () => signal, release: () => release(new Response(JSON.stringify(value))) }
}

it('keeps a protected-origin request alive while management still observes the shared key', async () => {
  const s = setup(), stopResults = s.results.subscribe(() => undefined), stopManagement = s.management.subscribe(() => undefined)
  stopResults()
  expect(s.signal()?.aborted).toBe(false)
  s.release()
  await vi.waitFor(() => expect(s.management.getCurrentResult().data).toEqual(value))
  expect(s.fetch).toHaveBeenCalledOnce()
  stopManagement(); s.client.clear()
})

it('aborts a protected-origin request when its final management observer leaves', async () => {
  const s = setup(), stopResults = s.results.subscribe(() => undefined), stopManagement = s.management.subscribe(() => undefined)
  stopResults()
  expect(s.signal()?.aborted).toBe(false)
  stopManagement()
  expect(s.signal()?.aborted).toBe(true)
  await Promise.resolve()
  expect(s.client.getQueryData(key)).toBeUndefined()
  s.client.clear()
})

it('does not retrofit cancellation into a management-origin read reused by protected results', async () => {
  const s = setup(), stopManagement = s.management.subscribe(() => undefined), stopResults = s.results.subscribe(() => undefined)
  stopManagement()
  expect(s.signal()).toBeUndefined()
  s.release()
  await vi.waitFor(() => expect(s.results.getCurrentResult().data).toEqual(value))
  expect(s.fetch).toHaveBeenCalledOnce()
  stopResults(); s.client.clear()
})
