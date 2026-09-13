import { describe, expect, it } from 'vitest'
import { decodeTournamentLeaderboard, validateTournamentLeaderboardRounds } from '../leaderboards'
import { tieBoard, tieRounds, tieTournamentId } from './__tests__/tieBreakFixtures'

function decode(value: unknown, metric: 'gross' | 'net' = 'gross') {
  return decodeTournamentLeaderboard(value, tieTournamentId, metric)
}
function validate(value: unknown, rounds = tieRounds) {
  return validateTournamentLeaderboardRounds(decode(value), rounds)
}

describe('tournament tie-break transport and round composition', () => {
  it.each(['gross', 'net'] as const)('accepts %s final outside best-N with residual shared competition places', (metric) => {
    const board = tieBoard(metric)
    expect(validateTournamentLeaderboardRounds(decode(board, metric), tieRounds)).toEqual(board)
    expect(board.entries.every((entry) => entry.contributions.at(-1)?.counted === false)).toBe(true)
  })
  it('preserves default shared positions without comparison metadata', () => {
    expect(validate(tieBoard('gross', 'shared_positions')).entries.map((entry) => entry.position)).toEqual([1, 1, 1])
  })
  it.each(['tie_break_policy', 'final_round_number'])('requires valid %s', (field) => {
    expect(() => decode({ ...tieBoard(), [field]: undefined })).toThrow(field)
    expect(() => decode({ ...tieBoard(), [field]: 'unsupported' })).toThrow(field)
  })
  it('requires nullable integer entry metadata and rejects metadata for a shared-position policy', () => {
    const board = tieBoard()
    expect(() => decode({ ...board, entries: board.entries.map((entry) => ({ ...entry, tie_break_score_to_par: undefined })) })).toThrow('tie_break_score_to_par')
    expect(() => decode({ ...board, entries: board.entries.map((entry) => ({ ...entry, tie_break_score_to_par: 1.5 })) })).toThrow('tie_break_score_to_par')
    expect(() => decode({ ...board, tie_break_policy: 'shared_positions' })).toThrow('tie_break_score_to_par')
  })
  it('rejects partial comparison groups, reversed final ordering and incorrect competition places', () => {
    const board = tieBoard()
    expect(() => decode({ ...board, entries: board.entries.map((entry, i) => ({ ...entry, tie_break_score_to_par: i === 0 ? null : entry.tie_break_score_to_par })) })).toThrow('tie_break_score_to_par')
    expect(() => decode({ ...board, entries: [...board.entries].reverse() })).toThrow('position')
    expect(() => decode({ ...board, entries: board.entries.map((entry, i) => ({ ...entry, position: i + 1 })) })).toThrow('position')
  })
  it('rejects a plausible comparison score that does not match the scheduled final', () => {
    const board = tieBoard()
    expect(() => validate({ ...board, entries: board.entries.map((entry) => ({ ...entry, tie_break_score_to_par: (entry.tie_break_score_to_par ?? 0) + 1 })) })).toThrow('final round')
  })
  it('requires comparison metadata for a fully comparable primary tie group', () => {
    const board = tieBoard('gross', 'shared_positions')
    expect(() => validate({ ...board, tie_break_policy: 'final_round_score' })).toThrow('final round')
  })
  it('does not mistake the latest loaded round for an absent scheduled final', () => {
    const board = tieBoard('gross', 'shared_positions')
    expect(validate({ ...board, tie_break_policy: 'final_round_score', final_round_number: 3 }).entries.every((entry) => entry.tie_break_score_to_par === null)).toBe(true)
    expect(() => validate({ ...tieBoard(), final_round_number: 3 })).toThrow('final round')
  })
  it('keeps the whole primary group shared if one final is missing', () => {
    const board = tieBoard('gross', 'shared_positions')
    board.tie_break_policy = 'final_round_score'
    const entry = board.entries[0]
    if (!entry) throw new Error('Missing fixture')
    entry.contributions.pop(); entry.completed_rounds = 1
    expect(validate(board)).toEqual(board)
    expect(() => decode({ ...board, entries: board.entries.map((item, i) => ({ ...item, tie_break_score_to_par: i === 0 ? null : 5 })) })).toThrow('tie_break_score_to_par')
  })
  it('accepts a hidden final only without final contributions or comparison metadata', () => {
    const board = tieBoard('gross', 'shared_positions')
    board.tie_break_policy = 'final_round_score'; board.visibility = { mode: 'front_nine' }
    board.included_round_ids = board.included_round_ids.slice(0, 1)
    board.entries = board.entries.map((entry) => ({ ...entry, contributions: entry.contributions.slice(0, 1), completed_rounds: 1 }))
    expect(validate(board)).toEqual(board)
    expect(() => validate({ ...tieBoard(), visibility: { mode: 'front_nine' } })).toThrow('hidden final round')
  })
  it('keeps a fully qualified group shared while selected provisional scores remain', () => {
    const board = tieBoard('gross', 'shared_positions')
    board.tie_break_policy = 'final_round_score'
    board.current_round_id = tieRounds[0]?.id ?? null
    board.included_round_ids = board.included_round_ids.slice(1)
    board.entries = board.entries.map((entry) => ({ ...entry, completed_rounds: 1,
      contributions: entry.contributions.map((item, index) => ({ ...item, provisional: index === 0 })),
    }))
    const rounds = tieRounds.map((round, index) => index === 0 ? { ...round, status: 'open' as const } : round)
    expect(validate(board, rounds)).toEqual(board)
    expect(() => decode({ ...board, entries: board.entries.map((entry) => ({ ...entry, tie_break_score_to_par: 2 })) })).toThrow('tie_break_score_to_par')
  })
  it('rejects comparison on singleton and ineligible groups', () => {
    const board = tieBoard()
    expect(() => decode({ ...board, entries: board.entries.slice(0, 1) })).toThrow('tie_break_score_to_par')
    expect(() => decode({ ...board, required_counted_rounds: 3, final_round_number: 3, entries: board.entries.map((entry) => ({ ...entry, eligible: false })) })).toThrow()
  })
})
