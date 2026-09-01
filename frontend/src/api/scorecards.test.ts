import { describe, expect, it } from 'vitest'
import { decodeCompletionValidation, decodeReadScorecard, decodeScoreAccess, decodeScoringScorecard, scoringKeys } from './scorecards'

const roundId = '00000000-0000-0000-0000-000000004001'
const playerId = '00000000-0000-0000-0000-000000001001'
const secondPlayerId = '00000000-0000-0000-0000-000000001002'
const userId = '00000000-0000-0000-0000-000000000001'
const holeId = '00000000-0000-0000-0000-000000003201'
const owner = { type: 'player' as const, id: playerId }

describe('scorecard boundaries', () => {
  it('owns scorecard caches by the current account', () => {
    expect(scoringKeys.read('user-one', roundId, owner)).toEqual([
      'private-workspace', 'user-one', 'rounds', roundId, 'scorecards', 'read', 'player', playerId,
    ])
    expect(scoringKeys.scoring('user-one', roundId, owner)).not.toEqual(
      scoringKeys.read('user-one', roundId, owner),
    )
    expect(scoringKeys.read('user-one', roundId, owner)).not.toEqual(
      scoringKeys.read('user-two', roundId, owner),
    )
  })

  it('accepts seed UUIDs and exact scorecard timestamps', () => {
    const response = {
      round_id: roundId,
      owner,
      holes: [{
        hole_id: holeId,
        hole_number: 1,
        par: 4,
        stroke_index: 1,
        score: {
          id: '00000000-0000-0000-0000-000000009001',
          round_id: roundId,
          hole_id: holeId,
          owner,
          gross_strokes: 5,
          submitted_by: userId,
          submitted_at: '2026-09-10T10:00:00Z',
          updated_at: '2026-09-10T10:00:01.123Z',
        },
        net_strokes: 4,
      }],
      gross_total: 5,
      net_total: 4,
      playing_handicap: 1,
      holes_scored: 1,
      number_of_holes: 1,
      complete: true,
      confirmed: false,
      confirmed_by: null,
      confirmed_at: null,
    }
    const card = decodeScoringScorecard(response, roundId, owner)
    expect(card.holes[0]?.score?.gross_strokes).toBe(5)
    expect(() => decodeScoringScorecard({ ...response, round_id: secondPlayerId }, roundId, owner)).toThrow('identity')
    expect(() => decodeScoringScorecard({
      ...response,
      owner: { type: 'player', id: secondPlayerId },
    }, roundId, owner)).toThrow('identity')
  })

  it('accepts an actor-free restricted read and rejects hidden or actor-bearing facts', () => {
    const holes = Array.from({ length: 9 }, (_, index) => ({
      hole_id: `00000000-0000-0000-0000-${String(3201 + index).padStart(12, '0')}`,
      hole_number: index + 1,
      par: 4,
      stroke_index: index + 10,
      score: index === 0 ? { id: '00000000-0000-0000-0000-000000009001', gross_strokes: 5 } : null,
      net_strokes: index === 0 ? 4 : null,
    }))
    const response = {
      round_id: roundId,
      owner,
      holes,
      gross_total: 5,
      net_total: 4,
      playing_handicap: 9,
      holes_scored: 1,
      number_of_holes: 18,
      visible_hole_count: 9,
      complete: null,
      confirmed: null,
      confirmed_at: null,
      visibility: { mode: 'front_nine', observed_at: '2026-09-10T10:00:00Z', hidden_until: null },
    }
    expect(decodeReadScorecard(response, roundId, owner).holes).toHaveLength(9)
    expect(() => decodeReadScorecard({ ...response, confirmed: false }, roundId, owner)).toThrow('visibility')
    expect(() => decodeReadScorecard({ ...response, confirmed_by: userId }, roundId, owner)).toThrow('confirmed_by')
    expect(() => decodeReadScorecard({
      ...response,
      holes: [{ ...holes[0], score: { ...holes[0]?.score, submitted_by: userId } }, ...holes.slice(1)],
    }, roundId, owner)).toThrow('submitted_by')
  })

  it('decodes completion owners and rejects a format mismatch', () => {
    const response = {
      round_id: roundId,
      status: 'open',
      owners: [{ owner, owner_name: 'Player', holes_scored: 0, required_holes: 18, complete: false, confirmed: false }],
      ready_to_complete: false,
      ready_to_lock: false,
      issues: [
        { code: 'incomplete_scorecards', message: 'incomplete' },
        { code: 'unconfirmed_scorecards', message: 'unconfirmed' },
        { code: 'round_not_completed', message: 'not completed' },
      ],
      visibility: { mode: 'full', observed_at: '2026-09-10T10:00:00Z', hidden_until: null },
    }
    expect(decodeCompletionValidation(response, roundId, 'player').owners).toHaveLength(1)
    expect(() => decodeCompletionValidation(response, roundId, 'team')).toThrow('owner.type')
  })

  it('accepts restricted completion progress without inferring hidden readiness', () => {
    const response = {
      round_id: roundId,
      status: 'open',
      owners: [{ owner, owner_name: 'Player', holes_scored: 9, required_holes: 9, complete: null, confirmed: null }],
      ready_to_complete: null,
      ready_to_lock: null,
      issues: [],
      visibility: { mode: 'front_nine', observed_at: '2026-09-10T10:00:00Z', hidden_until: null },
    }
    expect(decodeCompletionValidation(response, roundId, 'player').owners[0]).toMatchObject({
      holes_scored: 9,
      required_holes: 9,
      complete: null,
      confirmed: null,
    })
    expect(() => decodeCompletionValidation({
      ...response,
      owners: [{ ...response.owners[0], complete: true }],
    }, roundId, 'player')).toThrow('progress')
    expect(() => decodeCompletionValidation({ ...response, ready_to_complete: false }, roundId, 'player'))
      .toThrow('visibility')
    expect(() => decodeCompletionValidation({
      ...response,
      issues: [{ code: 'unconfirmed_scorecards', message: 'hidden' }],
    }, roundId, 'player')).toThrow('visibility')
  })

  it('decodes a deterministic writable owner set and rejects duplicates', () => {
    expect(decodeScoreAccess({
      round_id: roundId,
      writable_owners: [owner, { type: 'player', id: secondPlayerId }],
    }, roundId).writable_owners).toEqual([owner, { type: 'player', id: secondPlayerId }])
    expect(() => decodeScoreAccess({
      round_id: roundId,
      writable_owners: [owner, owner],
    }, roundId)).toThrow('access.identity')
  })
})
