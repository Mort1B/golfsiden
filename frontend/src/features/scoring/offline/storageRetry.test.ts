// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { QueryClient } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../../../api/client'
import { queueDatabase, STORAGE_ERROR } from './database'
import { QueueRuntime } from './runtime'
import type { QueueTarget } from './model'
const target: QueueTarget = { accountId: crypto.randomUUID(), tournamentId: crypto.randomUUID(), roundId: crypto.randomUUID(),
 owner: { type: 'player', id: crypto.randomUUID() }, holeId: crypto.randomUUID(), holeNumber: 1 }
let online = false
const stops: (() => void)[] = []
beforeEach(() => {
 online = false
 vi.stubGlobal('indexedDB', new IDBFactory()); vi.stubGlobal('BroadcastChannel', undefined)
 vi.spyOn(navigator, 'onLine', 'get').mockImplementation(() => online)
})
afterEach(() => { stops.splice(0).forEach(stop => stop()); vi.restoreAllMocks(); vi.unstubAllGlobals() })
async function setup() {
 await queueDatabase.enqueue(target, 4, { type: 'absent' })
 const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
 const runtime = new QueueRuntime(target.accountId, 'synthetic', client, () => true)
 const stop = runtime.start(); stops.push(stop, () => client.clear())
 await vi.waitFor(() => expect(runtime.current().items).toHaveLength(1))
 return { runtime, stop, initial: runtime.current().items }
}
for (const fault of ['list', 'claim', 'contention'] as const) it(`yields to browser tasks after ${fault} without changing retained intent`, async () => {
 const { runtime, initial } = await setup()
 const save = vi.spyOn(api, 'saveConditionalScore')
 let attempts = 0
 // Even on broken code, end the fault schedule before it can hang the runner.
 const fail = () => { if (++attempts >= 20) online = false; throw new Error(STORAGE_ERROR) }
 vi.spyOn(queueDatabase, 'list').mockImplementation(fault === 'list' ? async () => fail() : async () => initial)
 if (fault !== 'list') vi.spyOn(queueDatabase, 'claim').mockImplementation(async () => {
  if (fault === 'claim') return fail()
  if (++attempts >= 20) online = false
  return null
 })
 online = true
 await runtime.wake()
 await new Promise(resolve => setTimeout(resolve, 0))
 expect(attempts).toBe(1)
 expect(runtime.current().items).toEqual(initial)
 if (fault !== 'contention') expect(runtime.current().error).toBe(STORAGE_ERROR)
 expect(save).not.toHaveBeenCalled()
})

function successfulDelivery() {
 vi.spyOn(api, 'scorecardScoring').mockResolvedValue({ projection: 'scoring', round_id: target.roundId, owner: target.owner,
  holes: [], gross_total: 0, net_total: 0, playing_handicap: 0, holes_scored: 0, number_of_holes: 18,
  complete: false, confirmed: false, confirmed_by: null, confirmed_at: null })
 return vi.spyOn(api, 'saveConditionalScore').mockImplementation(async (_round, request) => ({ request_id: request.request_id,
  applied_score: { score_id: crypto.randomUUID(), revision: '1' } }))
}
for (const fault of ['post-claim-read', 'acknowledge', 'failure-write', 'post-ack-read'] as const) it(`retains immutable requests and yields on ${fault}, then recovers`, async () => {
 const { runtime, initial } = await setup()
 const original = initial[0]; if (!original) throw new Error('missing queue')
 const save = successfulDelivery()
 const list = queueDatabase.list.bind(queueDatabase)
 let reads = 0
 if (fault.endsWith('read')) vi.spyOn(queueDatabase, 'list').mockImplementation(async account => {
  if (++reads === (fault === 'post-claim-read' ? 2 : 3)) throw new Error(STORAGE_ERROR)
  return list(account)
 })
 if (fault === 'acknowledge') vi.spyOn(queueDatabase, 'acknowledge').mockRejectedValueOnce(new Error(STORAGE_ERROR))
 if (fault === 'failure-write') {
  save.mockRejectedValueOnce(new Error('Synthetic network failure'))
  vi.spyOn(queueDatabase, 'fail').mockRejectedValueOnce(new Error(STORAGE_ERROR))
 }
 online = true
 await runtime.wake()
 expect(runtime.current().error).toBe(STORAGE_ERROR)
 await new Promise(resolve => setTimeout(resolve, 0))
 expect(save).toHaveBeenCalledTimes(fault === 'post-claim-read' ? 0 : 1)
 const retained = await list(target.accountId)
 if (fault === 'post-ack-read') expect(retained).toEqual([])
 else expect(retained[0]?.head).toEqual(original.head)
 // Expiry permits safe replay of the SAME request; no test rewrites queue data.
 const afterLease = Date.now() + 20_001
 vi.spyOn(Date, 'now').mockReturnValue(afterLease)
 await runtime.wake()
 await vi.waitFor(() => expect(runtime.current().items).toEqual([]))
 expect(runtime.current().error).toBeNull()
 for (const call of save.mock.calls) expect(call[1]).toEqual(original.head)
})
it('an overlapping successful reload cannot turn a failed claim into immediate progress', async () => {
 const { runtime, initial } = await setup()
 vi.spyOn(queueDatabase, 'list').mockResolvedValue(initial)
 let attempts = 0
 vi.spyOn(queueDatabase, 'claim').mockImplementation(async () => {
  if (++attempts >= 20) online = false
  await runtime.wake()
  throw new Error(STORAGE_ERROR)
 })
 online = true; await runtime.wake()
 await new Promise(resolve => setTimeout(resolve, 0))
 expect(attempts).toBe(1)
 expect(runtime.current().items).toEqual(initial)
 expect(runtime.current().error).toBe(STORAGE_ERROR)
})
for (const recovery of ['manual', 'timer', 'return', 'online'] as const) it(`recovers a failed read through ${recovery} without losing the original request`, async () => {
 const { runtime, initial } = await setup()
 const save = successfulDelivery()
 const list = queueDatabase.list.bind(queueDatabase)
 let failed = true, attempts = 0
 vi.spyOn(queueDatabase, 'list').mockImplementation(async account => {
  if (failed) { if (++attempts >= 20) online = false; throw new Error(STORAGE_ERROR) }
  return list(account)
 })
 online = true; await runtime.wake()
 await new Promise(resolve => setTimeout(resolve, 0))
 expect(attempts).toBe(1); expect(runtime.current().error).toBe(STORAGE_ERROR)
 failed = false
 if (recovery === 'manual') await runtime.wake()
 if (recovery === 'return') window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true }))
 if (recovery === 'online') window.dispatchEvent(new Event('online'))
 await vi.waitFor(() => expect(runtime.current().items).toEqual([]), { timeout: 3000 })
 expect(save).toHaveBeenCalledOnce()
 expect(save.mock.calls[0]?.[1]).toEqual(initial[0]?.head)
 expect(runtime.current().error).toBeNull()
})
it('stopping after a storage failure prevents return/timer work or dispatch', async () => {
 const { runtime, stop, initial } = await setup()
 const save = successfulDelivery()
 const read = vi.spyOn(queueDatabase, 'list').mockRejectedValueOnce(new Error(STORAGE_ERROR))
 online = true; await runtime.wake(); stop()
 const before = read.mock.calls.length
 window.dispatchEvent(new Event('online')); await runtime.wake()
 await new Promise(resolve => setTimeout(resolve, 0))
 expect(read).toHaveBeenCalledTimes(before)
 expect(save).not.toHaveBeenCalled()
 expect(runtime.current().items).toEqual(initial)
})
it('keeps a sustained outage bounded by the existing polling interval', async () => {
 const { runtime, initial } = await setup()
 let attempts = 0
 const read = vi.spyOn(queueDatabase, 'list').mockImplementation(async () => {
  if (++attempts >= 20) online = false
  throw new Error(STORAGE_ERROR)
 })
 online = true; await runtime.wake()
 await new Promise(resolve => setTimeout(resolve, 2200))
 expect(read).toHaveBeenCalledTimes(2)
 expect(runtime.current().items).toEqual(initial)
 expect(runtime.current().error).toBe(STORAGE_ERROR)
})
