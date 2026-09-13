// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { QueryClient } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { matchApi } from '../../../api/matchPlay'
import { ApiHttpError } from '../../../api/http'
import { matchFixture, matchIds } from '../../../api/matchPlay/fixtures'
import { matchDatabase } from './database'
import { MatchRuntime } from './runtime'
const target = { accountId: matchIds.user, tournamentId: matchIds.tournament, roundId: matchIds.round, matchId: matchIds.match }
const note = { type: 'note' as const, player_id: matchIds.first, hole_number: 1, gross_strokes: 4 }
const stops: (() => void)[] = []
function runtime() { const client = new QueryClient({ defaultOptions: { queries: { retry: false } } }); const value = new MatchRuntime(target.accountId, 'csrf', client, () => true); stops.push(value.start(), () => client.clear()); return value }
beforeEach(() => { vi.stubGlobal('indexedDB', new IDBFactory()); vi.stubGlobal('BroadcastChannel', undefined) })
afterEach(() => { stops.splice(0).forEach(stop => stop()); vi.restoreAllMocks(); vi.unstubAllGlobals() })
it('lost ACK followed by authorization failure preserves the exact immutable request until restored replay', async () => {
  const item = await matchDatabase.enqueue(target, null, '1', note, null)
  const send = vi.spyOn(matchApi, 'command').mockRejectedValueOnce(new Error('lost ACK')).mockRejectedValueOnce(new ApiHttpError(403, 'forbidden', 'revoked')).mockImplementation(async (_r, m, req) => ({ request_id: req.request_id, match_id: m, applied_revision: '2' }))
  vi.spyOn(matchApi, 'scoring').mockResolvedValue({ ...matchFixture(), revision: '2' })
  const rt = runtime(); await vi.waitFor(() => expect(rt.current().items[0]?.notes[0]?.phase).toBe('unknown'))
  await matchDatabase.retry(item.key); await rt.wake()
  expect(rt.current().items[0]?.notes[0]?.phase).toBe('unknown')
  expect(rt.current().items[0]?.notes[0]?.request).toEqual(item.notes[0]?.request)
  await matchDatabase.retry(item.key); await rt.wake()
  expect(rt.current().items[0]?.notes).toEqual([])
  expect(send.mock.calls.map(call => call[2].request_id)).toEqual(Array(3).fill(item.notes[0]?.request.request_id))
})
it('a verified receipt after lock uses permitted read and keeps unsent successors blocked', async () => {
  const a = await matchDatabase.enqueue(target, null, '1', note, null)
  await matchDatabase.enqueue(target, a.generation, '1', { ...note, player_id: matchIds.second }, null)
  const send = vi.spyOn(matchApi, 'command').mockRejectedValueOnce(new Error('lost ACK')).mockImplementation(async (_r, m, req) => ({ request_id: req.request_id, match_id: m, applied_revision: '2' }))
  vi.spyOn(matchApi, 'scoring').mockRejectedValue(new ApiHttpError(403, 'forbidden', 'locked'))
  const { revision: _revision, accepted_events: _accepted, ...read } = matchFixture(); void _revision; void _accepted
  vi.spyOn(matchApi, 'read').mockResolvedValue({ ...read, round_status: 'locked', visibility: { mode: 'front_nine' }, holes: read.holes.slice(0, 9), confirmed: null, correction_pending: null })
  const rt = runtime(); await vi.waitFor(() => expect(rt.current().items[0]?.notes[0]?.phase).toBe('unknown'))
  await matchDatabase.retry(a.key); await rt.wake()
  expect(rt.current().items[0]?.notes).toHaveLength(1)
  expect(rt.current().items[0]?.notes[0]?.phase).toBe('blocked'); expect(send).toHaveBeenCalledTimes(2)
})
it('unknown online report remains retained on401 and refuses a second online action', async () => {
  vi.spyOn(matchApi, 'scoring').mockResolvedValue(matchFixture())
  vi.spyOn(matchApi, 'command').mockRejectedValue(new ApiHttpError(401, 'unauthenticated', 'expired'))
  const rt = runtime(); await vi.waitFor(() => expect(rt.current().loading).toBe(false))
  const command = { type: 'report' as const, event: { type: 'concession' as const, conceding_player_id: matchIds.second, communicated: true, after_hole: 0 } }
  await rt.onlineAction(target, matchFixture(), command)
  expect(rt.current().items[0]?.action?.phase).toBe('unknown')
  const req = rt.current().items[0]?.action?.request
  await expect(rt.onlineAction(target, matchFixture(), command)).rejects.toThrow()
  expect(rt.current().items[0]?.action?.request).toEqual(req)
})
it('fresh terminal verification blocks unsent successor notes without attempting or rebasing them', async () => {
  const a = await matchDatabase.enqueue(target, null, '1', note, null)
  const b = await matchDatabase.enqueue(target, a.generation, '1', { ...note, player_id: matchIds.second }, null)
  const send = vi.spyOn(matchApi, 'command').mockImplementation(async (_r, m, req) => ({ request_id: req.request_id, match_id: m, applied_revision: '2' }))
  vi.spyOn(matchApi, 'scoring').mockResolvedValue({ ...matchFixture(), revision: '3', finish: { type: 'conceded', winner: 'first' } })
  const rt = runtime()
  await vi.waitFor(() => expect(rt.current().items[0]?.notes[0]?.phase).toBe('blocked'))
  expect(rt.current().items[0]?.notes[0]?.request).toEqual(b.notes[1]?.request)
  await rt.wake(); expect(send).toHaveBeenCalledTimes(1)
})
