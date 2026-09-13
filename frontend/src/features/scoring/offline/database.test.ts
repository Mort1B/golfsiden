// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { queueDatabase, STALE_ERROR } from './database'
import { cardKey, queueKey, type QueueTarget } from './model'

export const target: QueueTarget = { accountId: '00000000-0000-0000-0000-000000000001',
  tournamentId: '00000000-0000-0000-0000-000000000002', roundId: '00000000-0000-0000-0000-000000000003',
  owner: { type: 'player', id: '00000000-0000-0000-0000-000000000004' },
  holeId: '00000000-0000-0000-0000-000000000005', holeNumber: 1 }
const absent = { type: 'absent' as const }
const applied = { score_id: '00000000-0000-0000-0000-000000000006', revision: '9007199254740993' }
async function current() {
  const item = (await queueDatabase.list(target.accountId))[0]
  if (!item) throw new Error('Missing queued score')
  return item
}
beforeEach(() => { vi.stubGlobal('indexedDB', new IDBFactory()) })

describe('durable score transactions', () => {
  it('persists multiple holes and isolates account reads', async () => {
    await queueDatabase.enqueue(target, 4, absent)
    await queueDatabase.enqueue({ ...target, holeId: crypto.randomUUID(), holeNumber: 2 }, 5, absent)
    expect(await queueDatabase.list(target.accountId)).toHaveLength(2)
    expect(await queueDatabase.list(crypto.randomUUID())).toEqual([])
    expect((await current()).head.expected_score).toEqual(absent)
  })
  it('keeps a queued head immutable and bases the latest successor on its acknowledgement', async () => {
    await queueDatabase.enqueue(target, 4, absent)
    const original = await current()
    const claimed = await queueDatabase.claim(original.key, crypto.randomUUID())
    expect(claimed).not.toBeNull()
    await queueDatabase.enqueue(target, 6, { type: 'present', ...applied, revision: '999' })
    await queueDatabase.enqueue(target, 7, absent)
    expect((await current()).head).toEqual(original.head)
    await queueDatabase.acknowledge(original.key, { request_id: original.head.request_id, applied_score: applied })
    const successor = await current()
    expect(successor.desired).toBe(7)
    expect(successor.head.gross_strokes).toBe(7)
    expect(successor.head.request_id).not.toBe(original.head.request_id)
    expect(successor.head.expected_score).toEqual({ type: 'present', ...applied })
    await queueDatabase.acknowledge(original.key, { request_id: original.head.request_id, applied_score: applied })
    expect(await current()).toEqual(successor)
  })
  it('allows only one tab to claim, then reuses the same immutable operation after timeout', async () => {
    await queueDatabase.enqueue(target, 4, absent)
    const results = await Promise.all([queueDatabase.claim(queueKey(target), crypto.randomUUID()), queueDatabase.claim(queueKey(target), crypto.randomUUID())])
    expect(results.filter(Boolean)).toHaveLength(1)
    const item = await current()
    vi.spyOn(Date, 'now').mockReturnValue(Date.now() + 21_000)
    const retry = await queueDatabase.claim(item.key, crypto.randomUUID())
    expect(retry?.head).toEqual(item.head)
    vi.restoreAllMocks()
  })
  it('will not discard an active delivery or a newer local generation', async () => {
    await queueDatabase.enqueue(target, 4, absent)
    const first = await current()
    await queueDatabase.enqueue(target, 5, absent)
    await expect(queueDatabase.resolve(first.key, first.generation, null)).rejects.toThrow(STALE_ERROR)
    const newer = await current()
    await queueDatabase.claim(newer.key, crypto.randomUUID())
    await expect(queueDatabase.resolve(newer.key, newer.generation, null)).rejects.toThrow(STALE_ERROR)
    expect((await current()).desired).toBe(5)
  })
  it('requires an explicit new operation for reviewed conflicts and rejects stale conflict decisions', async () => {
    await queueDatabase.enqueue(target, 4, absent)
    const first = await current(); const lease = crypto.randomUUID()
    await queueDatabase.claim(first.key, lease)
    await queueDatabase.fail(first.key, first.head.request_id, lease, 'conflict')
    await queueDatabase.resolve(first.key, first.generation, { type: 'present', ...applied })
    const resolved = await current()
    expect(resolved.head.request_id).not.toBe(first.head.request_id)
    expect(resolved.head.expected_score).toEqual({ type: 'present', ...applied })
    await expect(queueDatabase.resolve(first.key, first.generation, null)).rejects.toThrow(STALE_ERROR)
    await queueDatabase.resolve(resolved.key, resolved.generation, null)
    expect(await queueDatabase.list(target.accountId)).toEqual([])
  })
  it('serializes confirmation against queued edits and other tabs', async () => {
    const key = cardKey(target); const id = crypto.randomUUID()
    const until = await queueDatabase.acquireConfirmation(key, id)
    expect(until).toBeGreaterThan(Date.now())
    await expect(queueDatabase.enqueue(target, 4, absent)).rejects.toThrow('bekreftes')
    await expect(queueDatabase.acquireConfirmation(key, crypto.randomUUID())).rejects.toThrow('allerede')
    await queueDatabase.releaseConfirmation(key, crypto.randomUUID())
    await expect(queueDatabase.enqueue(target, 4, absent)).rejects.toThrow('bekreftes')
    await queueDatabase.releaseConfirmation(key, id)
    await queueDatabase.enqueue(target, 4, absent)
    await expect(queueDatabase.acquireConfirmation(key, id)).rejects.toThrow('levert')
  })
  it('expires abandoned confirmation leases using wall clock', async () => {
    const key = cardKey(target)
    const until = await queueDatabase.acquireConfirmation(key, crypto.randomUUID())
    vi.spyOn(Date, 'now').mockReturnValue(until + 1)
    await queueDatabase.enqueue(target, 4, absent)
    expect((await current()).desired).toBe(4)
    vi.restoreAllMocks()
  })
  it('reports storage failure without claiming any persisted edit', async () => {
    vi.stubGlobal('indexedDB', undefined)
    await expect(queueDatabase.enqueue(target, 4, absent)).rejects.toThrow('ikke trygt lagret')
  })
})
