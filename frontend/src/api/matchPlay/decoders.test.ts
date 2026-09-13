import { expect, it } from 'vitest'
import { decodeCard, decodeTable, decodeListing } from '../matchPlay'
import { matchFixture, matchIds } from './fixtures'
import { decodeRequest } from './eventDecoder'
import { decodeOverall } from '../overall'
function readFixture() { const { revision: _revision, accepted_events: _events, ...card } = matchFixture(); void _revision; void _events; return card }
it('separates read and scoring projections and bounds decimal revisions without numeric loss', () => {
  const card = matchFixture()
  expect(decodeCard(card, matchIds.round, matchIds.match, true)).toEqual(card)
  expect(decodeCard(readFixture(), matchIds.round, matchIds.match)).toEqual(readFixture())
  expect(() => decodeCard(card, matchIds.round, matchIds.match)).toThrow()
  expect(() => decodeCard({ ...card, revision: '9223372036854775808' }, matchIds.round, matchIds.match, true)).toThrow()
  expect(() => decodeCard({ ...card, revision: 1 }, matchIds.round, matchIds.match, true)).toThrow()
})
it('rejects mixed commands, stale identity, missing snapshots and wrong relative handicaps', () => {
  expect(() => decodeRequest({ request_id: matchIds.user, expected_revision: '1', command: { type: 'clear_note', player_id: matchIds.first, hole_number: 1, gross_strokes: 4 } })).toThrow()
  const c = matchFixture()
  expect(() => decodeCard(c, matchIds.tournament, matchIds.match, true)).toThrow()
  expect(() => decodeCard({ ...c, opponents: [{ ...c.opponents[0], playing_handicap: null }, c.opponents[1]] }, matchIds.round, matchIds.match, true)).toThrow()
  expect(() => decodeCard({ ...c, relative_handicaps: [0, 14] }, matchIds.round, matchIds.match, true)).toThrow()
})
it('validates concession provenance, exact winner and confirmed half-points', () => {
  const c = readFixture(), event = { type: 'concession', conceding_player_id: matchIds.second, communicated: true, after_hole: 0 }
  const finished = { ...c, events: [event], finish: { type: 'conceded', winner: 'first' }, confirmed: true, half_points: [2, 0] }
  expect(decodeCard(finished, matchIds.round, matchIds.match).half_points).toEqual([2, 0])
  for (const bad of [{ ...finished, half_points: [0, 2] }, { ...finished, finish: { type: 'conceded', winner: 'second' } }, { ...finished, events: [{ ...event, conceding_player_id: matchIds.user }] }, { ...finished, events: [{ ...event, communicated: false }] }]) expect(() => decodeCard(bad, matchIds.round, matchIds.match)).toThrow()
})
it('does not accept hidden metadata or unpermitted effective points', () => {
  const c = readFixture(), hidden = { ...c, holes: c.holes.slice(0, 9), visibility: { mode: 'front_nine' }, confirmed: null, correction_pending: null }
  expect(decodeCard(hidden, matchIds.round, matchIds.match).confirmed).toBeNull()
  expect(() => decodeCard({ ...hidden, confirmed: false }, matchIds.round, matchIds.match)).toThrow()
  expect(() => decodeCard({ ...hidden, notes: [{ player_id: matchIds.first, hole_number: 10, gross_strokes: 4 }] }, matchIds.round, matchIds.match)).toThrow()
})
it('rejects fabricated table counts/ranks and writable IDs outside the listing', () => {
  const entry = { player_id: matchIds.first, display_name: 'A', half_points: 3, played: 3, wins: 1, draws: 1, losses: 1, position: 1 }
  expect(decodeTable({ tournament_id: matchIds.tournament, entries: [entry] }, matchIds.tournament).entries[0]).toEqual(entry)
  expect(() => decodeTable({ tournament_id: matchIds.tournament, entries: [{ ...entry, wins: 2 }] }, matchIds.tournament)).toThrow()
  expect(() => decodeListing({ round_id: matchIds.round, matches: [readFixture()], writable_match_ids: [matchIds.user] }, matchIds.round)).toThrow()
})
it('decodes the explicit match-only overall result without inventing an empty board', () => {
  const result = { type: 'not_applicable', reason: 'match_only', tournament_id: matchIds.tournament, metric: 'net' }
  expect(decodeOverall(result, matchIds.tournament, 'net')).toEqual(result)
  expect(() => decodeOverall({ ...result, entries: [] }, matchIds.tournament, 'net')).toThrow()
})
