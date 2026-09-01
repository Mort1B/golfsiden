import { describe, expect, it } from 'vitest'
import type { Round } from './types'
import { matchMandatoryRound, validateMandatoryRound } from './mandatoryRounds'

const targetRound: Round = {
  id: '00000000-0000-0000-0000-000000000011',
  tournament_id: '00000000-0000-0000-0000-000000000001',
  round_number: 1,
  name: 'Finale',
  round_date: '2026-09-01',
  course_id: null,
  course_name: '',
  tee_id: null,
  tee_name: '',
  number_of_holes: 18,
  status: 'draft',
  handicap_enabled: true,
  handicap_allowance_percent: 100,
  scoring_format: 'individual_stroke_play',
  created_at: '2026-09-01T10:00:00Z',
  updated_at: '2026-09-01T10:00:00Z',
}

describe('mandatory round coherence', () => {
  it('resolves none and an exact loaded target round', () => {
    expect(matchMandatoryRound(null, [targetRound])).toEqual({ state: 'none', round: null })
    expect(validateMandatoryRound(
      targetRound.id,
      [targetRound],
      'turneringsdata',
      'tournament.mandatory_round_id',
    )).toBe(targetRound)
  })

  it('fails closed for missing and cross-target round ids', () => {
    const otherTargetRound = {
      ...targetRound,
      id: '00000000-0000-0000-0000-000000000022',
      tournament_id: '00000000-0000-0000-0000-000000000002',
    }
    expect(matchMandatoryRound(otherTargetRound.id, [targetRound])).toEqual({ state: 'missing', round: null })
    expect(() => validateMandatoryRound(
      otherTargetRound.id,
      [targetRound],
      'resultatdata',
      'leaderboard.mandatory_round_id round identity',
    )).toThrow('mandatory_round_id round identity')
  })
})
