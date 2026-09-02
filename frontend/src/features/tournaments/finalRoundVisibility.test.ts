import { describe, expect, it } from 'vitest'
import { ApiHttpError } from '../../api/http'
import type { Round, Tournament } from '../../api/types'
import { applicableFinalRound, finalRoundVisibilityFailure } from './finalRoundVisibility'

const tournament: Tournament = {
  id: '00000000-0000-0000-0000-000000000001',
  name: 'Tur', description: '', start_date: '2026-09-01', end_date: '2026-09-03',
  number_of_rounds: 3, counted_rounds: 2, mandatory_round_id: null, status: 'active',
  scoring_mode: 'combined', created_at: '2026-09-01T10:00:00Z', updated_at: '2026-09-01T10:00:00Z',
}

function round(overrides: Partial<Round> = {}): Round {
  return {
    id: '00000000-0000-0000-0000-000000000003', tournament_id: tournament.id,
    round_number: 3, name: 'Finale', round_date: '2026-09-03',
    course_id: '00000000-0000-0000-0000-000000000010', course_name: 'Finalebanen',
    tee_id: '00000000-0000-0000-0000-000000000011', tee_name: 'Gul', number_of_holes: 18,
    status: 'open', handicap_enabled: true, handicap_allowance_percent: 100,
    scoring_format: 'individual_stroke_play', created_at: '2026-09-01T10:00:00Z',
    updated_at: '2026-09-01T10:00:00Z', ...overrides,
  }
}

describe('final-round visibility control state', () => {
  it('finds only the configured 18-hole round with the authoritative final number', () => {
    const final = round()
    expect(applicableFinalRound(tournament, [round({ round_number: 2 }), final])).toBe(final)
    expect(applicableFinalRound(tournament, [round({ number_of_holes: 9 })])).toBeNull()
    expect(applicableFinalRound(tournament, [round({ course_id: null })])).toBeNull()
    expect(applicableFinalRound(tournament, [round({ tee_id: null })])).toBeNull()
    expect(applicableFinalRound(tournament, [round({
      tournament_id: '00000000-0000-0000-0000-000000000099',
    })])).toBeNull()
  })

  it('maps stale writes to authoritative refetch and retains other server errors', () => {
    expect(finalRoundVisibilityFailure(new ApiHttpError(
      409,
      'final_round_visibility_stale',
      'stale',
    ))).toMatchObject({ refetch: true, message: expect.stringContaining('endret et annet sted') })
    expect(finalRoundVisibilityFailure(new ApiHttpError(500, 'internal_error', 'Serverfeil')))
      .toEqual({ refetch: false, message: 'Serverfeil' })
  })
})
