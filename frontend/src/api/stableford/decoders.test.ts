import { describe, expect, it } from 'vitest'
import { stablefordFixture } from './fixtures'
import { decodeStablefordRead, decodeStablefordScoring } from './decoders'
import { decodeRoundLeaderboard } from '../leaderboards/roundDecoder'
import { decodeTournamentLeaderboard } from '../leaderboards/tournamentDecoder'
import { decodePublicResults } from '../resultSharing/decoders'
import { publicFixture } from '../resultSharing/__tests__/fixtures'
const parse = (card: unknown) => { const identity = stablefordFixture(); return decodeStablefordScoring(card, identity.round_id, identity.owner.id) }
describe('Stableford input, points, equivalents and privacy', () => {
  it.each(['blank', 'numeric', 'pickup'] as const)('keeps %s distinct and checks resolved progress', state => {
    const card = stablefordFixture(state)
    expect(parse(card)).toEqual(card)
    expect(card.complete).toBe(state !== 'blank')
    expect(card.values?.gross_points).toBe(state === 'blank' ? undefined : state === 'numeric' ? 36 : 0)
    expect(card.values?.gross_equivalent).toBe(state === 'pickup' ? 36 : state === 'blank' ? undefined : 0)
  })
  it.each([
    { format: 'four_ball_stroke_play' }, { complete: false }, { holes_scored: 17 }, { number_of_holes: 9 },
    { values: { ...stablefordFixture('pickup').values, actual_gross_total: 72 } },
    { values: { ...stablefordFixture('pickup').values, gross_points: 36 } },
  ])('rejects malformed metadata/values %j', change => expect(() => parse({ ...stablefordFixture('pickup'), ...change })).toThrow())
  it('rejects zero strokes, spoofed points/revision, and wrong input ownership', () => {
    for (const change of [{ input: { type: 'numeric', gross_strokes: 0 } }, { revision: '0' }, { owner: { type: 'team', id: crypto.randomUUID() } }]) {
      const card = stablefordFixture('numeric'); const hole = card.holes[0]; if (!hole) throw new Error('fixture')
      expect(() => parse({ ...card, holes: [{ ...hole, score: { ...hole.score, ...change } }, ...card.holes.slice(1)] })).toThrow()
    }
    const card = stablefordFixture('numeric'); if (card.holes[0]) card.holes[0].net_points = 0
    expect(() => parse(card)).toThrow()
  })
  it('accepts signed plus allocation and checks no net-stroke floor or point ceiling', () => {
    const card = stablefordFixture('numeric'); card.playing_handicap = -18
    card.holes = card.holes.map(h => ({ ...h, handicap_strokes: -1, net_strokes: 5, net_points: 1 }))
    card.values = { gross_points: 36, net_points: 18, gross_equivalent: 0, net_equivalent: 18, actual_gross_total: 72, actual_net_total: 90 }
    expect(parse(card).values?.net_points).toBe(18)
    card.playing_handicap = 126
    card.holes = card.holes.map(h => ({ ...h, handicap_strokes: 7, net_strokes: -3, net_points: 9 }))
    card.values = { gross_points: 36, net_points: 162, gross_equivalent: 0, net_equivalent: -126, actual_gross_total: 72, actual_net_total: -54 }
    expect(parse(card).values?.net_points).toBe(162)
  })
  it('permits a read projection of nine pickups but never exposes completion or actual totals', () => {
    const card = stablefordFixture('pickup')
    const projected = { ...card, holes: card.holes.slice(0, 9).map(h => ({ ...h, score: h.score && { id: h.score.id, input: h.score.input } })),
      holes_scored: 9, visible_hole_count: 9, complete: null, confirmed: null, visibility: { mode: 'front_nine' },
      values: { ...card.values, gross_equivalent: 18, net_equivalent: 18 } }
    expect(decodeStablefordRead(projected, card.round_id, card.owner.id).complete).toBeNull()
    expect(() => parse(projected)).toThrow()
    expect(() => decodeStablefordRead({ ...projected, confirmed: false }, card.round_id, card.owner.id)).toThrow()
  })
})
const tournamentId = '00000000-0000-0000-0000-000000000009'
function roundBoard() {
  const card = stablefordFixture('pickup')
  return { round_id: card.round_id, tournament_id: tournamentId, status: 'open', scoring_format: 'individual_stableford', metric: 'gross',
    number_of_holes: 18, visible_hole_count: 18, visibility: { mode: 'full' }, entries: [{ position: 1, tied: false,
      owner: card.owner, owner_name: card.owner_name, members: [], playing_handicap: 0, holes_scored: 18, number_of_holes: 18, complete: true, confirmed: false,
      value: { type: 'stableford', version: 1, ...card.values } }] }
}
it('ranks zero-point resolved cards ahead of unstarted cards and rejects ascending points', () => {
  const board = roundBoard(); const first = board.entries[0]; if (!first) throw new Error('fixture')
  const blank = { ...first, owner: { type: 'player', id: crypto.randomUUID() }, position: null, holes_scored: 0, complete: false,
    value: { ...first.value, gross_equivalent: 0, net_equivalent: 0 } }
  const read = (entries: unknown[]) => decodeRoundLeaderboard({ ...board, entries }, board.round_id, tournamentId, 'gross')
  expect(read([first, blank]).entries[1]?.position).toBeNull()
  expect(() => read([blank, first])).toThrow()
  expect(() => read([{ ...first, gross_total: 72 }])).toThrow()
  const better = { ...first, position: 2, owner: { type: 'player', id: crypto.randomUUID() }, value: { ...first.value, gross_points: 40, gross_equivalent: -4 } }
  expect(() => read([first, better])).toThrow()
})
it('validates mixed overall selection and totals without using points as strokes', () => {
  const card = stablefordFixture('pickup')
  const sf = { round_id: card.round_id, owner: card.owner, owner_name: card.owner_name, provisional: false, holes_scored: 18, number_of_holes: 18,
    counted: false, mandatory: false, value: { type: 'stableford', version: 1, ...card.values } }
  const stroke = { round_id: crypto.randomUUID(), owner: card.owner, owner_name: card.owner_name, provisional: false, holes_scored: 18, number_of_holes: 18,
    counted: true, mandatory: false, gross_total: 74, net_total: 72, par_total: 72, score_to_par: 2 }
  const entry = { position: 1, tied: false, player_id: card.owner.id, display_name: card.owner_name, status: 'active', completed_rounds: 2,
    counted_contributions: 1, eligible: true, current_team: null, contributions: [sf, stroke], value: { type: 'overall_equivalent', version: 1, gross: 2, net: 0, selected: 2, tie_break: null } }
  const board = { tournament_id: tournamentId, metric: 'gross', required_counted_rounds: 1, final_round_number: 2, mandatory_round_id: null,
    current_round_id: null, included_round_ids: [sf.round_id, stroke.round_id], visibility: { mode: 'full' }, tie_break_policy: 'shared_positions', entries: [entry] }
  expect(decodeTournamentLeaderboard(board, tournamentId, 'gross').entries[0]?.value?.selected).toBe(2)
  expect(() => decodeTournamentLeaderboard({ ...board, entries: [{ ...entry, value: { ...entry.value, selected: 74 } }] }, tournamentId, 'gross')).toThrow()
  expect(() => decodeTournamentLeaderboard({ ...board, entries: [{ ...entry, contributions: [{ ...sf, counted: true }, { ...stroke, counted: false }] }] }, tournamentId, 'gross')).toThrow()
})
it('public equivalents are selected-only and reject private, numeric-total or unknown variant fields', () => {
  const original = publicFixture()
  const board = { ...original, entries: original.entries.map(entry => {
    const base = Object.fromEntries(Object.entries(entry).filter(([key]) => !['total', 'par_total', 'score_to_par', 'tie_break_score_to_par'].includes(key)))
    return { ...base, value: { type: 'overall_equivalent', version: 1, selected: entry.score_to_par, tie_break: entry.tie_break_score_to_par } }
  }) }
  expect(decodePublicResults(board, board.grant_id, board.metric).entries[0]?.value?.type).toBe('overall_equivalent')
  for (const change of [{ gross: 3 }, { type: 'stableford' }, { version: 2 }]) {
    const first = board.entries[0]; if (!first) throw new Error('fixture')
    expect(() => decodePublicResults({ ...board, entries: [{ ...first, value: { ...first.value, ...change } }, ...board.entries.slice(1)] }, board.grant_id, board.metric)).toThrow()
  }
})

it('checks private result units against configured rounds even when Stableford has no visible contribution', async () => {
  const { tieBoard, tieRounds } = await import('../leaderboards/__tests__/tieBreakFixtures')
  const { validateTournamentLeaderboardRounds } = await import('../leaderboards')
  const board = tieBoard('gross'); const rounds = tieRounds
  expect(() => validateTournamentLeaderboardRounds(board, rounds)).not.toThrow()
  const changed = rounds.map((r, index) => index === 0 ? { ...r, scoring_format: 'individual_stableford' as const } : r)
  expect(() => validateTournamentLeaderboardRounds(board, changed)).toThrow()
  const equivalents = decodeTournamentLeaderboard({ ...board, entries: board.entries.map(e => ({ ...Object.fromEntries(Object.entries(e).filter(([key]) => !['gross_total', 'net_total', 'par_total', 'score_to_par', 'tie_break_score_to_par'].includes(key))), value: { type: 'overall_equivalent', version: 1, gross: 0, net: 0, selected: 0, tie_break: e.tie_break_score_to_par } })) }, board.tournament_id, 'gross')
  expect(() => validateTournamentLeaderboardRounds(equivalents, rounds)).toThrow()
})
it('rejects native contribution units independently of a matching overall basis', async () => {
  const { tieBoard, tieRounds } = await import('../leaderboards/__tests__/tieBreakFixtures')
  const { validateTournamentLeaderboardRounds } = await import('../leaderboards')
  const source = tieBoard('gross')
  const rounds = tieRounds.map((r, index) => index === 1 ? { ...r, scoring_format: 'individual_stableford' as const } : r)
  const native = (c: typeof source.entries[number]['contributions'][number]) => {
    if (c.value) throw new Error('Expected stroke fixture')
    const base = Object.fromEntries(Object.entries(c).filter(([key]) => !['gross_total', 'net_total', 'par_total', 'score_to_par'].includes(key)))
    return { ...base, value: { type: 'stableford', version: 1, gross_points: 36 - (c.gross_total - c.par_total), net_points: 36 - (c.net_total - c.par_total),
      gross_equivalent: c.gross_total - c.par_total, net_equivalent: c.net_total - c.par_total, actual_gross_total: c.gross_total, actual_net_total: c.net_total } }
  }
  const board = (nativeIndexes: number[]) => decodeTournamentLeaderboard({ ...source, entries: source.entries.map(e => ({
    ...Object.fromEntries(Object.entries(e).filter(([key]) => !['gross_total', 'net_total', 'par_total', 'score_to_par', 'tie_break_score_to_par', 'contributions'].includes(key))),
    value: { type: 'overall_equivalent', version: 1, gross: 0, net: 0, selected: 0, tie_break: e.tie_break_score_to_par },
    contributions: e.contributions.map((c, i) => nativeIndexes.includes(i) ? native(c) : c),
  })) }, source.tournament_id, 'gross')
  expect(() => validateTournamentLeaderboardRounds(board([1]), rounds)).not.toThrow()
  expect(() => validateTournamentLeaderboardRounds(board([]), rounds)).toThrow('round identity')
  expect(() => validateTournamentLeaderboardRounds(board([0, 1]), rounds)).toThrow('round identity')
  const draftRound = { ...rounds[1], id: crypto.randomUUID(), round_number: 3, scoring_format: 'individual_stableford' as const, status: 'draft' as const }
  if (!rounds[1]) throw new Error('fixture')
  const allRounds = [...tieRounds, { ...rounds[1], ...draftRound }]
  expect(() => validateTournamentLeaderboardRounds(board([]), allRounds)).not.toThrow()
  expect(() => validateTournamentLeaderboardRounds(source, allRounds)).toThrow('overall value basis')
})
