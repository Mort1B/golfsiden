import { QueryClient } from '@tanstack/react-query'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { matchApi, matchKeys } from '../matchPlay'
import { loadPrivateResult } from '../privateResults'
import { tournamentKeys } from '../tournaments'
import { matchIds } from './fixtures'

const scope = { userId: matchIds.user, tournamentId: matchIds.tournament, roundId: matchIds.round }
const listing = { round_id: matchIds.round, matches: [], writable_match_ids: [] }
const table = { tournament_id: matchIds.tournament, entries: [] }
const endpoints = [
  { name: 'list', key: matchKeys.list(matchIds.user, matchIds.round), value: listing,
    load: (signal?: AbortSignal): Promise<unknown> => matchApi.list(matchIds.round, signal) },
  { name: 'player list', key: matchKeys.listForPlayer(matchIds.user, matchIds.round, matchIds.first), value: { ...listing, player_id: matchIds.first },
    load: (signal?: AbortSignal): Promise<unknown> => matchApi.listForPlayer(matchIds.round, matchIds.first, signal) },
  { name: 'table', key: matchKeys.table(matchIds.user, matchIds.tournament), value: table,
    load: (signal?: AbortSignal): Promise<unknown> => matchApi.table(matchIds.tournament, signal) },
]
const response = (value: unknown, status = 200) => new Response(JSON.stringify(value), { status })
const failure = (status: number) => response({ error: { code: 'fixture_error', message: `Read failed ${status}` } }, status)
afterEach(() => { vi.unstubAllGlobals() })

for (const endpoint of endpoints) describe(`match ${endpoint.name} cancellation`, () => {
  it('forwards the exact signal so a pending HTTP request aborts', async () => {
    const controller = new AbortController()
    const fetch = vi.fn((_path: string, init?: RequestInit) => new Promise<Response>((_resolve, reject) => {
      expect(init?.signal).toBe(controller.signal)
      init?.signal?.addEventListener('abort', () => reject(init.signal?.reason), { once: true })
    }))
    vi.stubGlobal('fetch', fetch)
    const read = endpoint.load(controller.signal)
    controller.abort()
    await expect(read).rejects.toMatchObject({ name: 'AbortError' })
    expect(fetch).toHaveBeenCalledOnce()
  })

  it('keeps callers without a signal compatible', async () => {
    const fetch = vi.fn(async (_path: string, init?: RequestInit) => {
      expect(init?.signal).toBeUndefined()
      return response(endpoint.value)
    })
    vi.stubGlobal('fetch', fetch)
    await expect(endpoint.load()).resolves.toEqual(endpoint.value)
  })

  for (const oldStatus of [200, 401, 403, 404]) it(`ignores superseded HTTP ${oldStatus} after a replacement succeeds`, async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    let release: (value: Response) => void = () => undefined
    const held = new Promise<Response>(resolve => { release = resolve })
    let signal: AbortSignal | null | undefined
    vi.stubGlobal('fetch', vi.fn((_path: string, init?: RequestInit) => { signal = init?.signal; return held }))
    let obsolete: Promise<unknown> = Promise.resolve()
    const oldQuery = client.fetchQuery({ queryKey: endpoint.key, queryFn: context => {
      obsolete = loadPrivateResult(client, scope, endpoint.key, context.signal, () => endpoint.load(context.signal))
      return obsolete
    } }).catch(() => undefined)
    await client.cancelQueries({ queryKey: endpoint.key, exact: true })
    expect(signal?.aborted).toBe(true)
    vi.stubGlobal('fetch', vi.fn(async () => response(endpoint.value)))
    await client.fetchQuery({ queryKey: endpoint.key, queryFn: context =>
      loadPrivateResult(client, scope, endpoint.key, context.signal, () => endpoint.load(context.signal)) })
    // A deliberately noncooperative transport exercises the independent generation guard.
    release(oldStatus === 200 ? response(endpoint.value) : failure(oldStatus))
    await expect(obsolete).rejects.toThrow()
    await oldQuery
    expect(client.getQueryData(endpoint.key)).toEqual(endpoint.value)
    expect(client.getQueryState(endpoint.key)?.status).toBe('success')
    client.clear()
  })

  for (const status of [401, 403, 404]) it(`applies a fresh HTTP ${status} denial after cancellation`, async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    client.setQueryData(tournamentKeys.rounds(matchIds.user, matchIds.tournament), [{ id: matchIds.round }])
    const sibling = endpoint.name === 'list' ? matchKeys.table(matchIds.user, matchIds.tournament) : matchKeys.list(matchIds.user, matchIds.round)
    const other = matchKeys.table('another-account', matchIds.tournament)
    client.setQueryData(sibling, { retained: true }); client.setQueryData(other, { retained: true })
    vi.stubGlobal('fetch', vi.fn((_path: string, init?: RequestInit) => new Promise<Response>((_resolve, reject) => {
      init?.signal?.addEventListener('abort', () => reject(init.signal?.reason), { once: true })
    })))
    const old = client.fetchQuery({ queryKey: endpoint.key, queryFn: context =>
      loadPrivateResult(client, scope, endpoint.key, context.signal, () => endpoint.load(context.signal)) }).catch(() => undefined)
    await client.cancelQueries({ queryKey: endpoint.key, exact: true }); await old
    vi.stubGlobal('fetch', vi.fn(async () => failure(status)))
    await expect(client.fetchQuery({ queryKey: endpoint.key, queryFn: context =>
      loadPrivateResult(client, scope, endpoint.key, context.signal, () => endpoint.load(context.signal)) })).rejects.toThrow()
    expect(client.getQueryData(endpoint.key)).toBeUndefined()
    expect(client.getQueryState(endpoint.key)?.error).toMatchObject({ status })
    expect(client.getQueryData(sibling)).toBeUndefined()
    expect(client.getQueryData(other)).toEqual({ retained: true })
    client.clear()
  })

  it('preserves ordinary uncancelled errors', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => failure(503)))
    await expect(endpoint.load(new AbortController().signal)).rejects.toMatchObject({ status: 503, message: 'Read failed 503' })
  })
})
