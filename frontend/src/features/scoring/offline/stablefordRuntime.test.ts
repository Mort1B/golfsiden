// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { QueryClient } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../../../api/client'
import { stablefordApi } from '../../../api/stableford'
import { stablefordFixture } from '../../../api/stableford/fixtures'
import { scoringKeys } from '../../../api/scorecards'
import { ApiHttpError } from '../../../api/http'
import { queueDatabase } from './database'
import { QueueRuntime } from './runtime'
import type { StablefordTarget } from './model'
const card = stablefordFixture()
const target: StablefordTarget = { protocol: 'stableford_v1', accountId: crypto.randomUUID(),
  roundId: card.round_id, tournamentId: crypto.randomUUID(), owner: { type: 'player', id: card.owner.id },
  holeId: card.holes[0]?.hole_id ?? '', holeNumber: 1 }
let stop = () => undefined as void
beforeEach(() => { vi.stubGlobal('indexedDB', new IDBFactory()); vi.stubGlobal('BroadcastChannel', undefined) })
afterEach(() => { stop(); vi.restoreAllMocks(); vi.unstubAllGlobals() })
it('delivers the Stableford protocol and refreshes the player card before clearing verification', async () => {
  await queueDatabase.enqueueStableford(target, { type: 'no_score' }, { type: 'absent' })
  const oldSave = vi.spyOn(api, 'saveConditionalScore')
  const save = vi.spyOn(stablefordApi, 'save').mockImplementation(async (_round, operation) => ({ request_id: operation.request_id,
    applied_score: { score_id: crypto.randomUUID(), revision: '1' } }))
  let finish = () => undefined as void
  const wait = new Promise<void>(resolve => { finish = resolve })
  const read = vi.spyOn(stablefordApi, 'scoring').mockImplementation(async () => { await wait; return card })
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const runtime = new QueueRuntime(target.accountId, 'current', client, () => true); stop = runtime.start()
  await vi.waitFor(() => expect(read).toHaveBeenCalled())
  expect(save.mock.calls[0]?.[1].input).toEqual({ type: 'no_score' })
  expect(oldSave).not.toHaveBeenCalled()
  expect(read.mock.calls[0]?.slice(0, 2)).toEqual([card.round_id, card.owner.id])
  expect(runtime.current().refreshing).toHaveLength(1)
  finish()
  await vi.waitFor(() => expect(runtime.current().refreshing).toHaveLength(0))
  expect(client.getQueryData(scoringKeys.scoring(target.accountId, card.round_id, card.owner))).toEqual(card)
  client.clear()
})
it('removes the player scoring query when a retained player operation is rejected after locking', async () => {
  await queueDatabase.enqueueStableford(target, { type: 'numeric', gross_strokes: 4 }, { type: 'absent' })
  vi.spyOn(stablefordApi, 'save').mockRejectedValue(new ApiHttpError(409, 'round_not_editable', 'locked'))
  const client = new QueryClient()
  const key = scoringKeys.scoring(target.accountId, card.round_id, card.owner)
  client.setQueryData(key, card)
  const runtime = new QueueRuntime(target.accountId, 'current', client, () => true); stop = runtime.start()
  await vi.waitFor(() => expect(runtime.current().items[0]?.phase).toBe('blocked'))
  expect(client.getQueryData(key)).toBeUndefined()
  expect(await queueDatabase.list(target.accountId)).toHaveLength(1)
  client.clear()
})
it('promptly re-verifies after a live refetch cancels delivery verification without trusting cached data', async () => {
  await queueDatabase.enqueueStableford(target, { type: 'no_score' }, { type: 'absent' })
  vi.spyOn(stablefordApi, 'save').mockImplementation(async (_round, operation) => ({ request_id: operation.request_id,
    applied_score: { score_id: crypto.randomUUID(), revision: '1' } }))
  let release = () => undefined as void
  const wait = new Promise<void>(resolve => { release = resolve })
  const read = vi.spyOn(stablefordApi, 'scoring').mockImplementationOnce(async () => { await wait; return card }).mockResolvedValue(card)
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const key = scoringKeys.scoring(target.accountId, card.round_id, card.owner)
  const runtime = new QueueRuntime(target.accountId, 'current', client, () => true)
  stop = runtime.start()
  try {
    await vi.waitFor(() => expect(read).toHaveBeenCalledTimes(1))
    await client.cancelQueries({ queryKey: key, exact: true })
    release()
    expect(runtime.current().refreshing).toHaveLength(1)
    await vi.waitFor(() => expect(read).toHaveBeenCalledTimes(2), { timeout: 3500 })
    await vi.waitFor(() => expect(runtime.current().refreshing).toHaveLength(0))
    expect(client.getQueryData(key)).toEqual(card)
  } finally { release(); stop(); client.clear() }
})
it('retains verification after failed canonical refresh until a successful read', async () => {
  await queueDatabase.enqueueStableford(target, { type: 'no_score' }, { type: 'absent' })
  vi.spyOn(stablefordApi, 'save').mockImplementation(async (_round, operation) => ({ request_id: operation.request_id,
    applied_score: { score_id: crypto.randomUUID(), revision: '1' } }))
  const read = vi.spyOn(stablefordApi, 'scoring').mockRejectedValue(new Error('offline'))
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const runtime = new QueueRuntime(target.accountId, 'current', client, () => true); stop = runtime.start()
  await vi.waitFor(() => expect(runtime.current().refreshing[0]?.failed).toBe(true))
  expect(runtime.current().items).toHaveLength(0)
  read.mockResolvedValue(card)
  const future = Date.now() + 16_000; vi.spyOn(Date, 'now').mockReturnValue(future)
  await runtime.wake()
  expect(runtime.current().refreshing).toHaveLength(0)
  client.clear()
})
