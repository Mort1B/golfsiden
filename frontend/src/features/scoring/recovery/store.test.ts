// @vitest-environment jsdom
import { IDBFactory, IDBObjectStore } from 'fake-indexeddb'
import { QueryClient } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { waitFor } from '@testing-library/react'
import { ScoreDraftStore } from './store'
import { queueDatabase, STORAGE_ERROR } from '../offline/database'
import { QueueRuntime } from '../offline/runtime'
import { queueKey, type FourBallTarget } from '../offline/model'

const target: FourBallTarget = { protocol: 'four_ball_v1', sideId: crypto.randomUUID(), accountId: crypto.randomUUID(),
  tournamentId: crypto.randomUUID(), roundId: crypto.randomUUID(), owner: { type: 'player', id: crypto.randomUUID() }, holeId: crypto.randomUUID(), holeNumber: 1 }
const expected = { type: 'present' as const, score_id: crypto.randomUUID(), revision: '1' }
let stop = () => {}, runtime: QueueRuntime
beforeEach(() => {
  vi.stubGlobal('indexedDB', new IDBFactory())
  vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(false)
  runtime = new QueueRuntime(target.accountId, 'csrf', new QueryClient(), () => true)
  stop = runtime.start()
})
afterEach(() => { stop(); vi.restoreAllMocks(); vi.unstubAllGlobals() })
it('keeps each partner independent and stores offline recovery as review-only with the original expectation', async () => {
  const store = new ScoreDraftStore(target.accountId)
  const first = { ...target, owner: { type: 'player' as const, id: crypto.randomUUID() } }
  const failure = vi.spyOn(queueDatabase, 'enqueueFourBall').mockRejectedValue(new Error(STORAGE_ERROR))
  store.set({ kind: 'four_ball', slot: 2, target, value: { type: 'no_score' } }, expected, runtime)
  store.set({ kind: 'four_ball', slot: 2, target: first, value: { type: 'numeric', gross_strokes: 5 } }, { type: 'absent' }, runtime)
  await waitFor(() => expect(store.current().every(draft => !draft.saving)).toBe(true))
  store.set({ kind: 'four_ball', slot: 2, target, value: { type: 'numeric', gross_strokes: 8 } }, { ...expected, revision: '9' }, runtime)
  await waitFor(() => expect(store.current().every(draft => !draft.saving)).toBe(true))
  expect(store.current().find(draft => draft.key === queueKey(target))?.expected).toEqual(expected)
  store.suspend(); store.discard(queueKey(first))
  expect(store.current()).toHaveLength(1)
  failure.mockRestore()
  store.retry(queueKey(target), runtime)
  await waitFor(() => expect(store.current()).toHaveLength(0))
  const rows = await queueDatabase.list(target.accountId)
  expect(rows).toHaveLength(1)
  expect(rows[0]?.phase).toBe('conflict')
  expect(rows[0]?.head.expected_score).toEqual(expected)
  expect(rows[0]?.desired).toEqual({ type: 'numeric', gross_strokes: 8 })
})
it('aborts an in-flight device transaction when authority disappears before commit', async () => {
  const store = new ScoreDraftStore(target.accountId)
  await queueDatabase.list(target.accountId)
  const put = IDBObjectStore.prototype.put
  vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementation(function(this: IDBObjectStore, value: unknown, key?: IDBValidKey) {
    const request = key === undefined ? put.call(this, value) : put.call(this, value, key)
    if (this.name === 'pending') store.suspend()
    return request
  })
  store.set({ kind: 'four_ball', slot: 2, target, value: { type: 'no_score' } }, expected, runtime)
  await waitFor(() => expect(store.current()[0]?.saving).toBe(false))
  expect(store.current()[0]?.recovery).toBe(true)
  expect(await queueDatabase.list(target.accountId)).toHaveLength(0)
})
it.each([false, true])('preserves competing immutable heads on retry (recovery=%s)', async recovery => {
  const store = new ScoreDraftStore(target.accountId)
  const failure = vi.spyOn(queueDatabase, 'enqueueFourBall').mockRejectedValue(new Error(STORAGE_ERROR))
  store.set({ kind: 'four_ball', slot: 2, target, value: { type: 'no_score' } }, expected, runtime)
  await waitFor(() => expect(store.current()[0]?.saving).toBe(false))
  failure.mockRestore(); if (recovery) store.suspend()
  await queueDatabase.enqueueFourBall(target, { type: 'numeric', gross_strokes: 4 }, expected)
  const before = await queueDatabase.list(target.accountId)
  store.retry(queueKey(target), runtime)
  await waitFor(() => expect(store.current()[0]?.saving).toBe(false))
  expect(store.current()[0]?.error).toBeTruthy()
  expect(await queueDatabase.list(target.accountId)).toEqual(before)
})
it('a delayed old write cannot clear the newer local sequence', async () => {
  const store = new ScoreDraftStore(target.accountId)
  let release = () => {}
  const deferred = new Promise<void>(resolve => { release = resolve })
  const original = queueDatabase.enqueueFourBall
  vi.spyOn(queueDatabase, 'enqueueFourBall').mockImplementationOnce(async (...args) => { await deferred; return original(...args) })
  store.set({ kind: 'four_ball', slot: 1, target, value: { type: 'numeric', gross_strokes: 5 } }, expected, runtime)
  await waitFor(() => expect(queueDatabase.enqueueFourBall).toHaveBeenCalledOnce())
  store.set({ kind: 'four_ball', slot: 1, target, value: { type: 'numeric', gross_strokes: 6 } }, { ...expected, revision: '20' }, runtime)
  release()
  await waitFor(() => expect(store.current()).toHaveLength(0))
  const row = (await queueDatabase.list(target.accountId))[0]
  expect(row?.head.expected_score).toEqual(expected)
  expect(row?.protocol === 'four_ball_v1' && row.head.input).toEqual({ type: 'numeric', gross_strokes: 5 })
  expect(row?.desired).toEqual({ type: 'numeric', gross_strokes: 6 })
})
it('failed input never adopts a newly observed other-tab head on a later keystroke', async () => {
  const store = new ScoreDraftStore(target.accountId)
  const failure = vi.spyOn(queueDatabase, 'enqueueFourBall').mockRejectedValue(new Error(STORAGE_ERROR))
  store.set({ kind: 'four_ball', slot: 1, target, value: { type: 'numeric', gross_strokes: 5 } }, expected, runtime)
  await waitFor(() => expect(store.current()[0]?.saving).toBe(false))
  failure.mockRestore()
  await queueDatabase.enqueueFourBall(target, { type: 'numeric', gross_strokes: 7 }, expected)
  await runtime.changed()
  const head = (await queueDatabase.list(target.accountId))[0]?.head
  store.set({ kind: 'four_ball', slot: 1, target, value: { type: 'numeric', gross_strokes: 6 } }, { ...expected, revision: '99' }, runtime)
  await waitFor(() => expect(store.current()[0]?.saving).toBe(false))
  expect(store.current()[0]?.predecessor).toBeNull()
  expect(store.current()[0]?.expected).toEqual(expected)
  expect((await queueDatabase.list(target.accountId))[0]?.head).toEqual(head)
  expect((await queueDatabase.list(target.accountId))[0]?.desired).toEqual({ type: 'numeric', gross_strokes: 7 })
})
it('retry after an observed head disappears preserves the original expectation instead of rebasing', async () => {
  await queueDatabase.enqueueFourBall(target, { type: 'numeric', gross_strokes: 4 }, expected)
  await runtime.changed()
  const before = (await queueDatabase.list(target.accountId))[0]
  if (!before) throw new Error('fixture')
  const store = new ScoreDraftStore(target.accountId)
  const failure = vi.spyOn(queueDatabase, 'enqueueFourBall').mockRejectedValue(new Error(STORAGE_ERROR))
  store.set({ kind: 'four_ball', slot: 1, target, value: { type: 'no_score' } }, expected, runtime)
  await waitFor(() => expect(store.current()[0]?.saving).toBe(false))
  failure.mockRestore()
  await queueDatabase.resolve(before.key, before.generation, null)
  store.retry(queueKey(target), runtime)
  await waitFor(() => expect(store.current()).toHaveLength(0))
  expect((await queueDatabase.list(target.accountId))[0]?.head.expected_score).toEqual(expected)
})
it('retains recovery input while a confirmation lease prevents device persistence', async () => {
  const store = new ScoreDraftStore(target.accountId)
  const key = `${target.accountId}:${target.roundId}:player:${target.owner.id}`
  const lease = crypto.randomUUID()
  await queueDatabase.acquireConfirmation(key, lease)
  store.set({ kind: 'four_ball', slot: 2, target, value: { type: 'no_score' } }, expected, runtime)
  await waitFor(() => expect(store.current()[0]?.saving).toBe(false))
  store.suspend(); store.retry(queueKey(target), runtime)
  await waitFor(() => expect(store.current()[0]?.saving).toBe(false))
  expect(store.current()[0]?.error).toContain('bekreftes')
  expect(await queueDatabase.list(target.accountId)).toHaveLength(0)
  await queueDatabase.releaseConfirmation(key, lease)
  store.retry(queueKey(target), runtime)
  await waitFor(() => expect(store.current()).toHaveLength(0))
  expect((await queueDatabase.list(target.accountId))[0]?.phase).toBe('conflict')
})
