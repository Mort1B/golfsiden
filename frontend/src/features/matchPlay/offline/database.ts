import type { MatchNoteCommand, MatchRequest } from '../../../api/matchPlay'
import { decodeDraft, LEASE_MS, matchDraftKey, nextRevision, STORAGE_ERROR, STALE_ERROR, type MatchDraft, type MatchTarget, type Delivery } from './model'
export const MATCH_DATABASE = 'golf-match-notes-v1'
function request<T>(req: IDBRequest<T>): Promise<T> { return new Promise((resolve, reject) => { req.onsuccess = () => resolve(req.result); req.onerror = () => reject(new Error(STORAGE_ERROR)) }) }
function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === 'undefined') { reject(new Error(STORAGE_ERROR)); return }
    const req = indexedDB.open(MATCH_DATABASE, 1)
    req.onupgradeneeded = () => { req.result.createObjectStore('matches', { keyPath: 'key' }).createIndex('account', 'accountId') }
    req.onerror = req.onblocked = () => reject(new Error(STORAGE_ERROR))
    req.onsuccess = () => { req.result.onversionchange = () => req.result.close(); resolve(req.result) }
  })
}
async function transaction<T>(work: (store: IDBObjectStore) => Promise<T>): Promise<T> {
  const db = await open()
  try {
    const tx = db.transaction('matches', 'readwrite')
    const done = new Promise<void>((resolve, reject) => { tx.oncomplete = () => resolve(); tx.onabort = tx.onerror = () => reject(new Error(STORAGE_ERROR)) })
    try { const result = await work(tx.objectStore('matches')); await done; return result }
    catch (e) { try { tx.abort() } catch { /* finished */ } await done.catch(() => undefined); throw e }
  } finally { db.close() }
}
async function read(store: IDBObjectStore, key: string): Promise<MatchDraft | null> {
  const value: unknown = await request(store.get(key)); return value === undefined ? null : decodeDraft(value)
}
function initial(target: MatchTarget): MatchDraft { return { ...target, protocol: 'match_notes_v1', key: matchDraftKey(target), generation: crypto.randomUUID(), notes: [], action: null, lease: null, retryAt: 0 } }
function busy(item: MatchDraft): boolean { return item.lease !== null && item.lease.until > Date.now() }
function put(store: IDBObjectStore, item: MatchDraft): MatchDraft { const next = { ...item, generation: crypto.randomUUID() }; store.put(next); return next }
export const matchDatabase = {
  list: (account: string): Promise<MatchDraft[]> => transaction(async s => { const values: unknown[] = await request(s.index('account').getAll(account)); return values.map(decodeDraft) }),
  enqueue: (target: MatchTarget, observed: string | null, revision: string, command: MatchNoteCommand, oldValue: number | null): Promise<MatchDraft> => transaction(async s => {
    const current = await read(s, matchDraftKey(target))
    if ((current?.generation ?? null) !== observed) throw new Error(STALE_ERROR)
    const item = current ?? initial(target)
    if (busy(item) || item.action || item.notes.some(n => n.phase === 'blocked' || n.phase === 'acknowledged')) throw new Error('Matchen kontrolleres. Vent med nye notater.')
    const tail = item.notes.at(-1)
    const head: MatchRequest = { request_id: crypto.randomUUID(), expected_revision: tail ? nextRevision(tail.request.expected_revision) : revision, command }
    const delivery: Delivery = { request: head, predecessor: tail?.request.request_id ?? null, oldValue, phase: 'queued', error: null }
    return put(s, { ...item, notes: [...item.notes, delivery], retryAt: 0 })
  }),
  claim: (key: string, leaseId: string): Promise<MatchDraft | null> => transaction(async s => {
    const item = await read(s, key); const next = item?.action ?? item?.notes[0]
    if (!item || !next || next.phase === 'blocked' || busy(item) || item.retryAt > Date.now()) return null
    const marked = { ...next, phase: next.phase === 'acknowledged' ? 'acknowledged' as const : 'unknown' as const }
    return put(s, { ...item, lease: { id: leaseId, until: Date.now() + LEASE_MS }, ...(item.action ? { action: marked } : { notes: [marked, ...item.notes.slice(1)] }) })
  }),
  lease: (target: MatchTarget, id: string): Promise<void> => transaction(async s => {
    const item = await read(s, matchDraftKey(target)) ?? initial(target)
    if (busy(item) || item.notes.length || item.action) throw new Error('Alle lokale endringer og leveranser må være kontrollert først.')
    put(s, { ...item, lease: { id, until: Date.now() + LEASE_MS } })
  }),
  dispatch: (key: string, leaseId: string, request: MatchRequest): Promise<void> => transaction(async s => {
    const item = await read(s, key)
    if (!item || item.lease?.id !== leaseId || !busy(item) || item.notes.length || item.action) throw new Error(STALE_ERROR)
    put(s, { ...item, action: { request, predecessor: null, oldValue: null, phase: 'unknown', error: null } })
  }),
  mark: (key: string, id: string, leaseId: string, phase: Delivery['phase'], error: string | null): Promise<void> => transaction(async s => {
    const item = await read(s, key), head = item?.action ?? item?.notes[0]
    if (!item || !head || head.request.request_id !== id || item.lease?.id !== leaseId) return
    const marked = { ...head, phase, error }
    put(s, { ...item, ...(item.action ? { action: marked } : { notes: [marked, ...item.notes.slice(1)] }) })
  }),
  verified: (key: string, id: string, leaseId: string, locked = false): Promise<void> => transaction(async s => {
    const item = await read(s, key), head = item?.action ?? item?.notes[0]
    if (!item || !head || head.phase !== 'acknowledged' || head.request.request_id !== id || item.lease?.id !== leaseId) return
    put(s, { ...item, ...(item.action ? { action: null } : { notes: item.notes.slice(1).map(n => locked ? { ...n, phase: 'blocked' as const, error: 'Runden er låst. Gjennomgå notatet; det blir ikke sendt.' } : n) }), lease: null, retryAt: 0 })
  }),
  release: (key: string, id: string): Promise<void> => transaction(async s => {
    const item = await read(s, key)
    if (item?.lease?.id === id) put(s, { ...item, lease: null, retryAt: Date.now() + 2000 })
  }),
  discard: (key: string, generation: string): Promise<void> => transaction(async s => {
    const item = await read(s, key)
    if (!item || item.generation !== generation || busy(item)) throw new Error(STALE_ERROR)
    if (item.action || item.notes.some(n => n.phase === 'unknown' || n.phase === 'acknowledged')) throw new Error('En levert eller ukjent forespørsel må kontrolleres før notatene kan forkastes.')
    put(s, { ...item, notes: [], retryAt: 0 })
  }),
  resolve: (key: string, generation: string, revision: string, keep: boolean): Promise<void> => transaction(async s => {
    const item = await read(s, key)
    if (!item || item.generation !== generation || busy(item)) throw new Error(STALE_ERROR)
    if (item.action && item.action.phase !== 'blocked' || item.notes.some(n => n.phase === 'unknown' || n.phase === 'acknowledged')) throw new Error('Ukjent eller mottatt levering må kontrolleres først.')
    let expected = revision, predecessor: string | null = null
    const notes: Delivery[] = keep ? item.notes.map(n => {
      const request = { ...n.request, request_id: crypto.randomUUID(), expected_revision: expected }
      const next: Delivery = { ...n, request, predecessor, phase: 'queued', error: null }
      expected = nextRevision(expected); predecessor = request.request_id
      return next
    }) : []
    put(s, { ...item, notes, action: null, retryAt: 0 })
  }),
  retry: (key: string): Promise<void> => transaction(async s => { const item = await read(s, key); if (item && !busy(item)) put(s, { ...item, retryAt: 0 }) }),
}
