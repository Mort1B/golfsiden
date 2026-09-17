import type { ExpectedScore, ScoreAcknowledgement } from '../../../api/scorecards/conditional'
import { enqueueStableford, type StablefordTarget, acknowledge, cardKey, enqueue, hasLease, LEASE_MS, resolveOperation, enqueueFourBall, type FourBallTarget, queueKey, type ConfirmationLease, type PendingScore, type QueuePhase, type QueueTarget } from './model'
import type { FourBallInput } from '../../../api/fourBall'
import { decodeLease, decodePending } from './decode'

export const QUEUE_DATABASE = 'golf-pending-scores-v1'
export const STORAGE_ERROR = 'Kunne ikke lagre på denne enheten. Endringen er ikke trygt lagret. Prøv igjen eller forkast.'
export const STALE_ERROR = 'Endringen ble oppdatert i en annen fane. Se gjennom den på nytt.'

function request<T>(input: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => { input.onsuccess = () => resolve(input.result); input.onerror = () => reject(new Error(STORAGE_ERROR)) })
}
function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === 'undefined') { reject(new Error(STORAGE_ERROR)); return }
    const req = indexedDB.open(QUEUE_DATABASE, 1)
    req.onupgradeneeded = () => {
      const pending = req.result.createObjectStore('pending', { keyPath: 'key' })
      pending.createIndex('account', 'accountId'); pending.createIndex('card', 'cardKey')
      req.result.createObjectStore('confirmations', { keyPath: 'key' })
    }
    req.onsuccess = () => { req.result.onversionchange = () => req.result.close(); resolve(req.result) }
    req.onerror = () => reject(new Error(STORAGE_ERROR))
    req.onblocked = () => reject(new Error(STORAGE_ERROR))
  })
}
export interface PersistenceGuard {
  expectedHead: string | null
  written: (requestId: string) => void
  allowed: () => boolean
  subscribe: (listener: () => void) => () => void
}
async function transaction<T>(action: (pending: IDBObjectStore, confirmations: IDBObjectStore) => Promise<T>, guard?: PersistenceGuard): Promise<T> {
  const db = await open().catch(() => { throw new Error(STORAGE_ERROR) })
  try {
    if (guard && !guard.allowed()) throw new Error(STORAGE_ERROR)
    const tx = db.transaction(['pending', 'confirmations'], 'readwrite')
    const unsubscribe = guard?.subscribe(() => {
      if (!guard.allowed()) { try { tx.abort() } catch { /* Already committed. */ } }
    })
    const committed = new Promise<void>((resolve, reject) => {
      tx.oncomplete = () => resolve(); tx.onabort = () => reject(new Error(STORAGE_ERROR)); tx.onerror = () => reject(new Error(STORAGE_ERROR))
    })
    try {
      const result = await action(tx.objectStore('pending'), tx.objectStore('confirmations'))
      await committed
      return result
    } catch (error) {
      try { tx.abort() } catch { /* Already completed/aborted. */ }
      await committed.catch(() => undefined)
      throw error
    } finally { unsubscribe?.() }
  } catch (error) {
    if (error instanceof DOMException) throw new Error(STORAGE_ERROR)
    throw error
  } finally { db.close() }
}
async function get(store: IDBObjectStore, key: string): Promise<PendingScore | null> {
  const value: unknown = await request(store.get(key))
  return value === undefined ? null : decodePending(value)
}
async function confirmation(store: IDBObjectStore, key: string): Promise<ConfirmationLease | null> {
  const value: unknown = await request(store.get(key))
  return value === undefined ? null : decodeLease(value)
}
function write(store: IDBObjectStore, key: string, item: PendingScore | null): void {
  if (item === null) store.delete(key); else store.put(item)
}
export const queueDatabase = {
  retainForReview: (item: PendingScore): Promise<void> => transaction(async (pending, confirmations) => {
    const lock = await confirmation(confirmations, item.cardKey)
    if (lock && lock.until > Date.now()) throw new Error('Scorekortet bekreftes i en annen fane. Prøv igjen om litt.')
    if (await get(pending, item.key)) throw new Error('En lagret endring finnes allerede for dette hullet. Se gjennom den før du prøver igjen.')
    write(pending, item.key, { ...item, phase: 'conflict' })
  }),
  list: (accountId: string): Promise<PendingScore[]> => transaction(async pending => {
    const values: unknown[] = await request(pending.index('account').getAll(accountId))
    return values.map(decodePending)
  }),
  enqueue: (target: QueueTarget, desired: number, expected: ExpectedScore, guard?: PersistenceGuard): Promise<void> => transaction(async (pending, confirmations) => {
    const lock = await confirmation(confirmations, cardKey(target))
    if (lock && lock.until > Date.now()) throw new Error('Scorekortet bekreftes i en annen fane. Prøv igjen om litt.')
    const key = queueKey(target)
    const current = await get(pending, key)
    if (guard && current && current.head.request_id !== guard.expectedHead) throw new Error(STALE_ERROR)
    const next = enqueue(current, target, desired, expected)
    write(pending, key, next)
    guard?.written(next.head.request_id)
  }, guard),
  enqueueFourBall: (target: FourBallTarget, desired: FourBallInput, expected: ExpectedScore, guard?: PersistenceGuard): Promise<void> => transaction(async (pending, confirmations) => {
    const lock = await confirmation(confirmations, cardKey(target))
    if (lock && lock.until > Date.now()) throw new Error('Scorekortet bekreftes i en annen fane. Prøv igjen om litt.')
    const key = queueKey(target)
    const current = await get(pending, key)
    if (guard && current && current.head.request_id !== guard.expectedHead) throw new Error(STALE_ERROR)
    const next = enqueueFourBall(current, target, desired, expected)
    write(pending, key, next)
    guard?.written(next.head.request_id)
  }, guard),
  enqueueStableford: (target: StablefordTarget, desired: FourBallInput, expected: ExpectedScore, guard?: PersistenceGuard): Promise<void> => transaction(async (pending, confirmations) => {
    const lock = await confirmation(confirmations, cardKey(target))
    if (lock && lock.until > Date.now()) throw new Error('Scorekortet bekreftes i en annen fane. Prøv igjen om litt.')
    const key = queueKey(target)
    const current = await get(pending, key)
    if (guard && current && current.head.request_id !== guard.expectedHead) throw new Error(STALE_ERROR)
    const next = enqueueStableford(current, target, desired, expected)
    write(pending, key, next)
    guard?.written(next.head.request_id)
  }, guard),
  claim: (key: string, leaseId: string): Promise<PendingScore | null> => transaction(async pending => {
    const item = await get(pending, key)
    if (!item || item.phase !== 'queued' || hasLease(item) || item.retryAt > Date.now()) return null
    const claimed = { ...item, lease: { id: leaseId, until: Date.now() + LEASE_MS } }
    write(pending, key, claimed)
    return claimed
  }),
  acknowledge: (key: string, ack: ScoreAcknowledgement): Promise<void> => transaction(async pending => {
    const item = await get(pending, key)
    if (item) write(pending, key, acknowledge(item, ack))
  }),
  fail: (key: string, requestId: string, leaseId: string, phase: QueuePhase): Promise<void> => transaction(async pending => {
    const item = await get(pending, key)
    if (!item || item.head.request_id !== requestId || item.lease?.id !== leaseId) return
    const attempts = item.attempts + 1
    write(pending, key, { ...item, phase, lease: null, attempts, retryAt: Date.now() + Math.min(30_000, 1000 * 2 ** Math.min(attempts, 5)) })
  }),
  retry: (key: string, generation: string): Promise<void> => transaction(async pending => {
    const item = await get(pending, key)
    if (!item || item.generation !== generation || hasLease(item)) throw new Error(STALE_ERROR)
    if (item.phase === 'conflict') return
    write(pending, key, { ...item, phase: 'queued', retryAt: 0 })
  }),
  resolve: (key: string, generation: string, expected: ExpectedScore | null): Promise<void> => transaction(async pending => {
    const item = await get(pending, key)
    if (!item || item.generation !== generation || hasLease(item)) throw new Error(STALE_ERROR)
    write(pending, key, expected === null ? null : resolveOperation(item, expected))
  }),
  acquireConfirmation: (target: string | readonly string[], id: string): Promise<number> => transaction(async (pending, confirmations) => {
    const keys = [...new Set(typeof target === 'string' ? [target] : target)]
    if (keys.length === 0) throw new Error('Scorekort mangler')
    for (const key of keys) {
      const count = await request(pending.index('card').count(key))
      const lock = await confirmation(confirmations, key)
      if (count > 0) throw new Error('Alle lokale endringer må være levert før du bekrefter.')
      if (lock && lock.until > Date.now()) throw new Error('Scorekortet bekreftes allerede i en annen fane.')
    }
    const until = Date.now() + LEASE_MS
    for (const key of keys) confirmations.put({ key, id, until })
    return until
  }),
  releaseConfirmation: (target: string | readonly string[], id: string): Promise<void> => transaction(async (_pending, confirmations) => {
    for (const key of new Set(typeof target === 'string' ? [target] : target)) {
      const lock = await confirmation(confirmations, key)
      if (lock?.id === id) confirmations.delete(key)
    }
  }),
}
