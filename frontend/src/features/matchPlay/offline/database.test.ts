// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { matchDatabase } from './database'
import { matchIds } from '../../../api/matchPlay/fixtures'
import { type MatchTarget } from './model'
const target: MatchTarget = { accountId: matchIds.user, tournamentId: matchIds.tournament, roundId: matchIds.round, matchId: matchIds.match }
const first = { type: 'note' as const, player_id: matchIds.first, hole_number: 1, gross_strokes: 4 }, second = { ...first, player_id: matchIds.second, gross_strokes: 5 }
beforeEach(() => vi.stubGlobal('indexedDB', new IDBFactory()))
afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks() })
it('links both opponents through immutable expected revisions and rejects another tab stale generation', async () => {
  const a = await matchDatabase.enqueue(target, null, '20', first, null)
  const b = await matchDatabase.enqueue(target, a.generation, '20', second, null)
  expect(b.notes[0]?.request).toEqual(a.notes[0]?.request)
  expect(b.notes[1]?.request.expected_revision).toBe('21')
  expect(b.notes[1]?.predecessor).toBe(a.notes[0]?.request.request_id)
  await expect(matchDatabase.enqueue(target, a.generation, '20', first, 3)).rejects.toThrow('annen fane')
})
it('a match-wide online lease excludes both opponents and persistent uncertainty survives lease expiry', async () => {
  const lease = crypto.randomUUID(); await matchDatabase.lease(target, lease)
  const item = (await matchDatabase.list(target.accountId))[0]; if (!item) throw new Error('missing')
  await expect(matchDatabase.enqueue(target, item.generation, '1', second, null)).rejects.toThrow()
  const request = { request_id: crypto.randomUUID(), expected_revision: '1', command: { type: 'confirm' as const, result_agreed_or_awarded: true } }
  await matchDatabase.dispatch(item.key, lease, request); await matchDatabase.release(item.key, lease)
  const uncertain = (await matchDatabase.list(target.accountId))[0]; if (!uncertain) throw new Error('missing')
  expect(uncertain.action?.request).toEqual(request)
  await expect(matchDatabase.lease(target, crypto.randomUUID())).rejects.toThrow()
  await expect(matchDatabase.resolve(item.key, uncertain.generation, '2', false)).rejects.toThrow()
  await expect(matchDatabase.discard(item.key, uncertain.generation)).rejects.toThrow()
})
it('explicit rejected-action reset is separate from unknown-delivery recovery', async () => {
  const lease = crypto.randomUUID(); await matchDatabase.lease(target, lease)
  const item = (await matchDatabase.list(target.accountId))[0]; if (!item) throw new Error('missing')
  const req = { request_id: crypto.randomUUID(), expected_revision: '1', command: { type: 'confirm' as const, result_agreed_or_awarded: true } }
  await matchDatabase.dispatch(item.key, lease, req)
  await matchDatabase.mark(item.key, req.request_id, lease, 'blocked', 'rejected'); await matchDatabase.release(item.key, lease)
  const blocked = (await matchDatabase.list(target.accountId))[0]; if (!blocked) throw new Error('missing')
  await matchDatabase.resolve(item.key, blocked.generation, '2', false)
  expect((await matchDatabase.list(target.accountId))[0]?.action).toBeNull()
})
it('acknowledgement verifies only its head and terminal verification blocks fixed-revision successors', async () => {
  const a = await matchDatabase.enqueue(target, null, '1', first, null), b = await matchDatabase.enqueue(target, a.generation, '1', second, null)
  const lease = crypto.randomUUID(), claimed = await matchDatabase.claim(b.key, lease), head = claimed?.notes[0]
  if (!head) throw new Error('missing')
  await matchDatabase.mark(b.key, head.request.request_id, lease, 'acknowledged', null)
  await matchDatabase.verified(b.key, head.request.request_id, lease, true)
  const pending = (await matchDatabase.list(target.accountId))[0]
  expect(pending?.notes).toHaveLength(1); expect(pending?.notes[0]?.phase).toBe('blocked')
  expect(pending?.notes[0]?.request).toEqual(b.notes[1]?.request)
})
it('account isolation retains other accounts and unavailable storage fails closed', async () => {
  await matchDatabase.enqueue(target, null, '1', first, null)
  expect(await matchDatabase.list(matchIds.second)).toEqual([])
  vi.stubGlobal('indexedDB', undefined)
  await expect(matchDatabase.enqueue(target, null, '1', second, null)).rejects.toThrow('lagres')
})
