// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { queueDatabase, QUEUE_DATABASE, STALE_ERROR } from './database'
import { cardKey, enqueue, queueKey, type StablefordTarget } from './model'
import { decodePending } from './decode'
const target: StablefordTarget = { protocol: 'stableford_v1', accountId: crypto.randomUUID(),
  roundId: crypto.randomUUID(), tournamentId: crypto.randomUUID(), holeId: crypto.randomUUID(), holeNumber: 1,
  owner: { type: 'player', id: crypto.randomUUID() } }
const other: StablefordTarget = { ...target, owner: { type: 'player', id: crypto.randomUUID() } }
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
  await queueDatabase.enqueueStableford(target, { type: 'numeric', gross_strokes: 4 }, absent)
  const original = await current()
  await queueDatabase.enqueueStableford(target, pickup, { type: 'present', score_id: crypto.randomUUID(), revision: '999' })
  expect((await current()).head).toEqual(original.head)
  await queueDatabase.acknowledge(original.key, { request_id: original.head.request_id, applied_score: applied })
  const successor = await current()
  expect(successor.protocol === 'stableford_v1' && successor.head.input).toEqual(pickup)
  expect(successor.head.expected_score).toEqual({ type: 'present', ...applied })
  await queueDatabase.enqueueStableford(target, { type: 'numeric', gross_strokes: 5 }, absent)
  await queueDatabase.acknowledge(original.key, { request_id: original.head.request_id, applied_score: applied })
  expect((await current()).head).toEqual(successor.head)
  await queueDatabase.acknowledge(successor.key, { request_id: successor.head.request_id, applied_score: { ...applied, revision: '9007199254740994' } })
  expect((await current()).head.expected_score).toEqual({ type: 'present', ...applied, revision: '9007199254740994' })
})
it('leases exactly one player card across tabs while another player remains independent', async () => {
  const key = cardKey(target), lease = crypto.randomUUID()
  await queueDatabase.acquireConfirmation(key, lease)
  await expect(queueDatabase.enqueueStableford(target, pickup, absent)).rejects.toThrow('bekreftes')
  await expect(queueDatabase.enqueueStableford(other, pickup, absent)).resolves.toBeUndefined()
  await queueDatabase.releaseConfirmation(key, crypto.randomUUID())
  await expect(queueDatabase.acquireConfirmation(key, crypto.randomUUID())).rejects.toThrow('allerede')
  await queueDatabase.releaseConfirmation(key, lease)
  await queueDatabase.enqueueStableford(target, pickup, absent)
  await expect(queueDatabase.acquireConfirmation(key, crypto.randomUUID())).rejects.toThrow('levert')
})
it('requires an exact reviewed generation and no-score remains present on conflict resolution', async () => {
  await queueDatabase.enqueueStableford(target, pickup, absent)
  const old = await current()
  await queueDatabase.enqueueStableford(target, { type: 'numeric', gross_strokes: 7 }, absent)
  await expect(queueDatabase.resolve(old.key, old.generation, { type: 'present', ...applied })).rejects.toThrow(STALE_ERROR)
  const latest = await current()
  await queueDatabase.resolve(latest.key, latest.generation, { type: 'present', ...applied })
  expect((await current()).head.expected_score).toEqual({ type: 'present', ...applied })
  expect(await queueDatabase.list(crypto.randomUUID())).toEqual([])
})
