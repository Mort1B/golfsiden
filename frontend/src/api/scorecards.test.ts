import { describe, expect, it } from 'vitest'
import { decodeCompletionValidation, decodeScorecard } from './scorecards'
import { readScorerConfig } from './scorerConfig'

const roundId = '00000000-0000-0000-0000-000000004001'
const playerId = '00000000-0000-0000-0000-000000001001'
const userId = '00000000-0000-0000-0000-000000000001'
const holeId = '00000000-0000-0000-0000-000000003201'
const owner = { type: 'player' as const, id: playerId }

describe('scorecard boundaries', () => {
  it('accepts seed UUIDs and exact scorecard timestamps', () => {
    const card = decodeScorecard({
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
    }, roundId, owner)
    expect(card.holes[0]?.score?.gross_strokes).toBe(5)
    expect(readScorerConfig(userId)).toEqual({ ready: true, userId })
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
    }
    expect(decodeCompletionValidation(response, roundId, 'player').owners).toHaveLength(1)
    expect(() => decodeCompletionValidation(response, roundId, 'team')).toThrow('owner.type')
  })

  it('keeps missing and malformed scorer configuration read-only', () => {
    expect(readScorerConfig(undefined).ready).toBe(false)
    expect(readScorerConfig('not-a-uuid').ready).toBe(false)
  })
})
