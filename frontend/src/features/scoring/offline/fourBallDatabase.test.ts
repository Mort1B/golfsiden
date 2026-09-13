// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { queueDatabase, QUEUE_DATABASE, STALE_ERROR } from './database'
import { cardKey, enqueue, queueKey, type FourBallTarget } from './model'
import { decodePending } from './decode'
const target: FourBallTarget = { protocol: 'four_ball_v1', sideId: crypto.randomUUID(), accountId: crypto.randomUUID(),
  roundId: crypto.randomUUID(), tournamentId: crypto.randomUUID(), holeId: crypto.randomUUID(), holeNumber: 1,
  owner: { type: 'player', id: crypto.randomUUID() } }
const other: FourBallTarget = { ...target, owner: { type: 'player', id: crypto.randomUUID() } }
const absent = { type: 'absent' as const }
const pickup = { type: 'no_score' as const }
const applied = { score_id: crypto.randomUUID(), revision: '9007199254740993' }
async function current() { const item = (await queueDatabase.list(target.accountId)).find(item => item.key === queueKey(target)); if (!item) throw new Error('fixture'); return item }
beforeEach(() => vi.stubGlobal('indexedDB', new IDBFactory()))
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals() })
it('loads a pre-upgrade untagged numeric record from the same database without changing its head bytes', async () => {
  const legacy = enqueue(null, { accountId: target.accountId, tournamentId: target.tournamentId, roundId: target.roundId,
    owner: target.owner, holeId: target.holeId, holeNumber: 1 }, 4, absent)
  const serialized = JSON.stringify(legacy.head)
  await queueDatabase.list(target.accountId)
  await new Promise<void>((resolve, reject) => {
    const req = indexedDB.open(QUEUE_DATABASE, 1)
    req.onerror = () => reject(req.error)
    req.onsuccess = () => { const db = req.result; const tx = db.transaction('pending', 'readwrite'); tx.objectStore('pending').put(JSON.parse(JSON.stringify(legacy))); tx.oncomplete = () => { db.close(); resolve() }; tx.onerror = () => reject(tx.error) }
  })
  const loaded = await current()
  expect('protocol' in loaded).toBe(false)
  expect(JSON.stringify(loaded.head)).toBe(serialized)
  expect(decodePending(JSON.parse(JSON.stringify(legacy)))).toEqual(legacy)
})
it('keeps numeric head through pickup edits and uses only acknowledgement identity for successors', async () => {
  await queueDatabase.enqueueFourBall(target, { type: 'numeric', gross_strokes: 4 }, absent)
  const original = await current()
  await queueDatabase.enqueueFourBall(target, pickup, { type: 'present', score_id: crypto.randomUUID(), revision: '999' })
  expect((await current()).head).toEqual(original.head)
  await queueDatabase.acknowledge(original.key, { request_id: original.head.request_id, applied_score: applied })
  const successor = await current()
  expect(successor.protocol === 'four_ball_v1' && successor.head.input).toEqual(pickup)
  expect(successor.head.expected_score).toEqual({ type: 'present', ...applied })
  await queueDatabase.enqueueFourBall(target, { type: 'numeric', gross_strokes: 5 }, absent)
  await queueDatabase.acknowledge(original.key, { request_id: original.head.request_id, applied_score: applied })
  expect((await current()).head).toEqual(successor.head)
  await queueDatabase.acknowledge(successor.key, { request_id: successor.head.request_id, applied_score: { ...applied, revision: '9007199254740994' } })
  expect((await current()).head.expected_score).toEqual({ type: 'present', ...applied, revision: '9007199254740994' })
})
it('acquires both partner leases atomically and excludes either partner in another tab', async () => {
  const keys = [cardKey(target), cardKey(other)], lease = crypto.randomUUID()
  await queueDatabase.acquireConfirmation([...keys, keys[0] ?? ''], lease)
  await expect(queueDatabase.enqueueFourBall(target, pickup, absent)).rejects.toThrow('bekreftes')
  await expect(queueDatabase.enqueueFourBall(other, pickup, absent)).rejects.toThrow('bekreftes')
  await queueDatabase.releaseConfirmation(keys, crypto.randomUUID())
  await expect(queueDatabase.acquireConfirmation(cardKey(other), crypto.randomUUID())).rejects.toThrow('allerede')
  await queueDatabase.releaseConfirmation(keys, lease)
  await queueDatabase.enqueueFourBall(other, pickup, absent)
  await expect(queueDatabase.acquireConfirmation(keys, crypto.randomUUID())).rejects.toThrow('levert')
  // Failed acquisition must not leave a partial lease on the first partner.
  await expect(queueDatabase.enqueueFourBall(target, pickup, absent)).resolves.toBeUndefined()
})
it('requires an exact reviewed generation and no-score remains present on conflict resolution', async () => {
  await queueDatabase.enqueueFourBall(target, pickup, absent)
  const old = await current()
  await queueDatabase.enqueueFourBall(target, { type: 'numeric', gross_strokes: 7 }, absent)
  await expect(queueDatabase.resolve(old.key, old.generation, { type: 'present', ...applied })).rejects.toThrow(STALE_ERROR)
  const latest = await current()
  await queueDatabase.resolve(latest.key, latest.generation, { type: 'present', ...applied })
  expect((await current()).head.expected_score).toEqual({ type: 'present', ...applied })
  expect(await queueDatabase.list(crypto.randomUUID())).toEqual([])
})
