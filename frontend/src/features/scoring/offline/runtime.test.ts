// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { QueryClient } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../../../api/client'
import { ApiHttpError } from '../../../api/http'
import type { ScoringScorecard } from '../../../api/scorecards'
import { queueDatabase } from './database'
import { QueueRuntime } from './runtime'
import type { QueueTarget } from './model'

const target: QueueTarget = { accountId: '00000000-0000-0000-0000-000000000001',
  tournamentId: '00000000-0000-0000-0000-000000000002', roundId: '00000000-0000-0000-0000-000000000003',
  owner: { type: 'player', id: '00000000-0000-0000-0000-000000000004' },
  holeId: '00000000-0000-0000-0000-000000000005', holeNumber: 1 }
const card: ScoringScorecard = { projection: 'scoring', round_id: target.roundId, owner: target.owner, holes: [],
  gross_total: 0, net_total: 0, playing_handicap: 0, holes_scored: 0, number_of_holes: 18,
  complete: false, confirmed: false, confirmed_by: null, confirmed_at: null }
const stops: (() => void)[] = []
function runtime(accountId = target.accountId, csrf = 'current-session') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const value = new QueueRuntime(accountId, csrf, client, () => true)
  const stop = value.start(); stops.push(stop, () => client.clear())
  return { value, stop, client }
}
beforeEach(() => { vi.stubGlobal('indexedDB', new IDBFactory()); vi.stubGlobal('BroadcastChannel', undefined) })
afterEach(() => { stops.splice(0).forEach(stop => stop()); vi.restoreAllMocks(); vi.unstubAllGlobals() })

it('keeps verification distinct after an ack and also when another tab observes queue removal', async () => {
  await queueDatabase.enqueue(target, 4, { type: 'absent' })
  let deliver = () => undefined as void
  const wait = new Promise<void>(resolve => { deliver = resolve })
  vi.spyOn(api, 'saveConditionalScore').mockImplementation(async (_round, request) => {
    await wait
    return { request_id: request.request_id, applied_score: { score_id: crypto.randomUUID(), revision: '1' } }
  })
  let refresh = () => undefined as void
  const refreshWait = new Promise<void>(resolve => { refresh = resolve })
  vi.spyOn(api, 'scorecardScoring').mockImplementation(async () => { await refreshWait; return card })
  const a = runtime(); const b = runtime()
  await vi.waitFor(() => { expect(a.value.current().items).toHaveLength(1); expect(b.value.current().items).toHaveLength(1) })
  deliver()
  await vi.waitFor(() => expect(a.value.current().items).toHaveLength(0))
  void b.value.wake()
  await vi.waitFor(() => expect(b.value.current().items).toHaveLength(0))
  expect(a.value.current().refreshing[0]?.item.desired).toBe(4)
  expect(b.value.current().refreshing[0]?.item.desired).toBe(4)
  refresh()
  await vi.waitFor(() => { expect(a.value.current().refreshing).toHaveLength(0); expect(b.value.current().refreshing).toHaveLength(0) })
})
it('a failed post-ack read remains visible without stopping delivery of other holes', async () => {
  await queueDatabase.enqueue(target, 4, { type: 'absent' })
  await queueDatabase.enqueue({ ...target, holeId: crypto.randomUUID(), holeNumber: 2 }, 5, { type: 'absent' })
  const save = vi.spyOn(api, 'saveConditionalScore').mockImplementation(async (_round, request) => ({ request_id: request.request_id,
    applied_score: { score_id: crypto.randomUUID(), revision: '1' } }))
  vi.spyOn(api, 'scorecardScoring').mockRejectedValue(new Error('offline'))
  const { value } = runtime()
  await vi.waitFor(() => expect(save).toHaveBeenCalledTimes(2))
  await vi.waitFor(() => expect(value.current().refreshing.filter(item => item.failed)).toHaveLength(2))
  expect(await queueDatabase.list(target.accountId)).toHaveLength(0)
})
it('a different account neither displays nor sends retained operations', async () => {
  await queueDatabase.enqueue(target, 4, { type: 'absent' })
  const save = vi.spyOn(api, 'saveConditionalScore')
  const { value } = runtime(crypto.randomUUID())
  await vi.waitFor(() => expect(value.current().loading).toBe(false))
  expect(value.current().items).toHaveLength(0)
  expect(save).not.toHaveBeenCalled()
  expect(await queueDatabase.list(target.accountId)).toHaveLength(1)
})
it('late acknowledgements after session departure do not clear retained operations or hydrate caches', async () => {
  await queueDatabase.enqueue(target, 4, { type: 'absent' })
  let deliver = () => undefined as void
  const wait = new Promise<void>(resolve => { deliver = resolve })
  const save = vi.spyOn(api, 'saveConditionalScore').mockImplementation(async (_round, request) => {
    await wait
    return { request_id: request.request_id, applied_score: { score_id: crypto.randomUUID(), revision: '1' } }
  })
  const read = vi.spyOn(api, 'scorecardScoring')
  const { stop, client } = runtime()
  await vi.waitFor(() => expect(save).toHaveBeenCalledOnce())
  stop(); deliver()
  await vi.waitFor(() => expect(client.getQueryCache().getAll()).toHaveLength(0))
  expect(await queueDatabase.list(target.accountId)).toHaveLength(1)
  expect(read).not.toHaveBeenCalled()
})
it('discarding blocked data retires verification when current authority rejects the card', async () => {
  await queueDatabase.enqueue(target, 4, { type: 'absent' })
  vi.spyOn(api, 'saveConditionalScore').mockRejectedValue(new ApiHttpError(403, 'forbidden', 'denied'))
  vi.spyOn(api, 'scorecardScoring').mockRejectedValue(new ApiHttpError(403, 'forbidden', 'denied'))
  const { value } = runtime()
  await vi.waitFor(() => expect(value.current().items[0]?.phase).toBe('blocked'))
  const item = value.current().items[0]
  if (!item) throw new Error('Missing blocked score')
  await queueDatabase.resolve(item.key, item.generation, null)
  await value.changed()
  await vi.waitFor(() => expect(value.current().refreshing).toHaveLength(0))
  expect(value.current().items).toHaveLength(0)
})

it('updates the delivering tab with a newer generation from another tab before the request completes', async () => {
  await queueDatabase.enqueue(target, 4, { type: 'absent' })
  let deliver = () => undefined as void
  const wait = new Promise<void>(resolve => { deliver = resolve })
  const save = vi.spyOn(api, 'saveConditionalScore').mockImplementation(async (_round, request) => {
    await wait
    return { request_id: request.request_id, applied_score: { score_id: crypto.randomUUID(), revision: '1' } }
  })
  vi.spyOn(api, 'scorecardScoring').mockResolvedValue(card)
  const { value } = runtime()
  await vi.waitFor(() => expect(save).toHaveBeenCalledOnce())
  await queueDatabase.enqueue(target, 6, { type: 'absent' })
  await value.wake()
  expect(value.current().items[0]?.desired).toBe(6)
  deliver()
  await vi.waitFor(() => expect(save).toHaveBeenCalledTimes(2))
})
