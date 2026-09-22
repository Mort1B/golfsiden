import { afterEach, expect, it, vi } from 'vitest'
import { decodeListing, decodePlayerListing, matchApi, matchKeys } from '../matchPlay'
import { matchFixture, matchIds } from './fixtures'
function card() { const { revision: _revision, accepted_events: _events, ...read } = matchFixture(); void _revision; void _events; return read }
function listing() { return { round_id: matchIds.round, player_id: matchIds.first, matches: [card()], writable_match_ids: [matchIds.match] } }
afterEach(() => vi.unstubAllGlobals())
it('validates the round and selected player without weakening full-card evidence', () => {
  expect(decodePlayerListing(listing(), matchIds.round, matchIds.first)).toEqual(listing())
  expect(decodePlayerListing({ ...listing(), player_id: matchIds.second }, matchIds.round, matchIds.second).matches).toHaveLength(1)
  for (const value of [
    { ...listing(), round_id: matchIds.tournament }, { ...listing(), player_id: matchIds.second },
    { ...listing(), player_id: undefined }, { ...listing(), extra: true },
    { ...listing(), matches: [{ ...card(), lead: 1 }] },
    { ...listing(), writable_match_ids: [matchIds.user] },
    { ...listing(), matches: [card(), { ...card(), match_id: matchIds.user }] },
  ]) expect(() => decodePlayerListing(value, matchIds.round, matchIds.first)).toThrow()
  expect(() => decodePlayerListing({ ...listing(), player_id: matchIds.user }, matchIds.round, matchIds.user)).toThrow()
  expect(() => decodeListing(listing(), matchIds.round)).toThrow()
})
it('accepts empty and restricted lists with precise writable membership', () => {
  expect(decodePlayerListing({ ...listing(), matches: [], writable_match_ids: [] }, matchIds.round, matchIds.first).matches).toEqual([])
  const c = card(), hidden = { ...c, holes: c.holes.slice(0, 9), visibility: { mode: 'front_nine' }, confirmed: null, correction_pending: null }
  expect(decodePlayerListing({ ...listing(), matches: [hidden], writable_match_ids: [] }, matchIds.round, matchIds.first).matches[0]?.confirmed).toBeNull()
  expect(() => decodePlayerListing({ ...listing(), matches: [{ ...hidden, confirmed: false }] }, matchIds.round, matchIds.first)).toThrow()
})
it('uses a distinct target key, encoded parameter, AbortSignal and strict response identity', async () => {
  const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(listing()), { status: 200 }))
  vi.stubGlobal('fetch', fetch)
  const abort = new AbortController()
  await expect(matchApi.listForPlayer(matchIds.round, matchIds.first, abort.signal)).resolves.toEqual(listing())
  expect(fetch).toHaveBeenCalledWith(`/api/rounds/${matchIds.round}/match-play/matches?player_id=${matchIds.first}`, expect.objectContaining({ signal: abort.signal, credentials: 'include' }))
  expect(matchKeys.listForPlayer('user', matchIds.round, matchIds.first)).toEqual([...matchKeys.list('user', matchIds.round), 'player', matchIds.first])
  expect(matchKeys.listForPlayer('user', matchIds.round, matchIds.first)).not.toEqual(matchKeys.listForPlayer('user', matchIds.round, matchIds.second))
  fetch.mockResolvedValue(new Response(JSON.stringify({ ...listing(), player_id: matchIds.second }), { status: 200 }))
  await expect(matchApi.listForPlayer(matchIds.round, matchIds.first)).rejects.toThrow()
})
