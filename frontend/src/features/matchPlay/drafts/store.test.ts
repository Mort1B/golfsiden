// @vitest-environment jsdom
import { IDBFactory, IDBObjectStore } from 'fake-indexeddb'
import { QueryClient } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { matchIds } from '../../../api/matchPlay/fixtures'
import { matchDatabase } from '../offline/database'
import { MatchRuntime } from '../offline/runtime'
import { MatchNoteStore, type MatchNoteIntent } from './store'
const intent: MatchNoteIntent = { target: { accountId: matchIds.user, tournamentId: matchIds.tournament, roundId: matchIds.round, matchId: matchIds.match }, playerId: matchIds.first, slot: 1, hole: 3, value: '7', revision: '1', observed: null, oldValue: 4 }
const stops: (() => void)[] = []
function runtime(account = matchIds.user) { const client = new QueryClient(); const rt = new MatchRuntime(account, 'csrf', client, () => true); const stop = rt.start(); stops.push(stop, () => client.clear()); return { rt, stop } }
beforeEach(() => { vi.stubGlobal('indexedDB', new IDBFactory()); vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(false) })
afterEach(() => { stops.splice(0).forEach(stop => stop()); vi.restoreAllMocks(); vi.unstubAllGlobals() })
function only(store: MatchNoteStore) { const note = store.current()[0]; if (!note) throw new Error('Missing intent'); return note }
it('retains original conditional identity when refreshed cards or queue snapshots change', async () => {
 const store = new MatchNoteStore(matchIds.user), { rt } = runtime()
 store.set(intent, rt)
 store.set({ ...intent, value: '8', revision: '9', observed: crypto.randomUUID(), oldValue: 12 }, rt)
 expect(only(store)).toMatchObject({ value: '8', revision: '1', observed: null, oldValue: 4, hole: 3 })
 const other = await matchDatabase.enqueue(intent.target, null, '1', { type: 'note', player_id: matchIds.second, hole_number: 3, gross_strokes: 5 }, null)
 await store.save(only(store).key, rt)
 expect(only(store).saving).toBe(false)
 expect(only(store).error).toContain('annen fane')
 expect((await matchDatabase.list(matchIds.user))[0]?.generation).toBe(other.generation)
})
it('advances a sibling only through our committed append and keeps player/hole identities', async () => {
 const store = new MatchNoteStore(matchIds.user), { rt } = runtime()
 store.set(intent, rt); store.set({ ...intent, playerId: matchIds.second, slot: 2, value: '5' }, rt)
 await store.save(only(store).key, rt)
 expect(only(store)).toMatchObject({ playerId: matchIds.second, value: '5' })
 await store.save(only(store).key, rt)
 expect(store.current()).toEqual([])
 const row = (await matchDatabase.list(matchIds.user))[0]
 expect(row?.notes.map(n => n.request.expected_revision)).toEqual(['1', '2'])
 expect(row?.notes.map(n => n.request.command)).toEqual([
  { type: 'note', player_id: matchIds.first, hole_number: 3, gross_strokes: 7 },
  { type: 'note', player_id: matchIds.second, hole_number: 3, gross_strokes: 5 },
 ])
})
it('settles committed retention even when the submitting runtime has stopped', async () => {
 const store = new MatchNoteStore(matchIds.user), { rt, stop } = runtime()
 const enqueue = matchDatabase.enqueue
 let release = () => {}; const held = new Promise<void>(resolve => { release = resolve })
 vi.spyOn(matchDatabase, 'enqueue').mockImplementation(async (...args) => { const saved = await enqueue(...args); await held; return saved })
 store.set(intent, rt); const saving = store.save(only(store).key, rt)
 await vi.waitFor(async () => expect((await matchDatabase.list(matchIds.user))[0]?.notes).toHaveLength(1))
 stop(); expect(only(store).saving).toBe(true)
 release(); await saving
 expect(store.current()).toEqual([])
})
it('retains an asynchronous save error across runtime replacement', async () => {
 const store = new MatchNoteStore(matchIds.user), { rt, stop } = runtime()
 let reject: (error: Error) => void = () => {}; const held = new Promise<never>((_resolve, fail) => { reject = fail })
 vi.spyOn(matchDatabase, 'enqueue').mockReturnValue(held)
 store.set(intent, rt); const saving = store.save(only(store).key, rt)
 stop(); reject(new Error('Device full')); await saving
 expect(only(store)).toMatchObject({ value: '7', saving: false, error: 'Device full' })
})
it('aborts an in-progress device write on account teardown and leaves no durable operation', async () => {
 const store = new MatchNoteStore(matchIds.user), { rt } = runtime()
 const put = IDBObjectStore.prototype.put
 vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementation(function (this: IDBObjectStore, value: unknown, key?: IDBValidKey) {
  const result = key === undefined ? put.call(this, value) : put.call(this, value, key)
  store.close()
  return result
 })
 store.set(intent, rt); await store.save(only(store).key, rt)
 expect(store.current()).toEqual([])
 expect(await matchDatabase.list(matchIds.user)).toEqual([])
})
it('late old-account completion cannot mutate a replacement account store', async () => {
 const old = new MatchNoteStore(matchIds.user), { rt } = runtime()
 const enqueue = matchDatabase.enqueue
 let release = () => {}; const held = new Promise<void>(resolve => { release = resolve })
 vi.spyOn(matchDatabase, 'enqueue').mockImplementation(async (...args) => { const saved = await enqueue(...args); await held; return saved })
 old.set(intent, rt); const saving = old.save(only(old).key, rt)
 await vi.waitFor(async () => expect((await matchDatabase.list(matchIds.user))[0]?.notes).toHaveLength(1))
 old.close()
 const account = crypto.randomUUID(), next = new MatchNoteStore(account), newRuntime = runtime(account)
 next.set({ ...intent, target: { ...intent.target, accountId: account }, value: '9' }, newRuntime.rt)
 release(); await saving
 expect(old.current()).toEqual([])
 expect(only(next).value).toBe('9')
})
