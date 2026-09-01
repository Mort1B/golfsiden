import { describe, expect, it } from 'vitest'
import { ApiHttpError } from '../../api/http'
import type { Round } from '../../api/types'
import { countedRoundsAreEditable, countedRoundsFailure } from './countedRoundsEditor'

function round(number: number, status: Round['status']): Round {
  return {
    id: `round-${number}`,
    tournament_id: 'tournament',
    round_number: number,
    name: `Runde ${number}`,
    round_date: '2026-09-01',
    course_id: null,
    course_name: '',
    tee_id: null,
    tee_name: '',
    number_of_holes: 18,
    status,
    handicap_enabled: true,
    handicap_allowance_percent: 100,
    scoring_format: 'individual_stroke_play',
    created_at: '2026-08-16T12:00:00Z',
    updated_at: '2026-08-16T12:00:00Z',
  }
}

describe('counted rounds editor state', () => {
  it('enables presentation whenever every loaded round remains draft', () => {
    expect(countedRoundsAreEditable('draft', [round(1, 'draft'), round(2, 'draft')])).toBe(true)
    expect(countedRoundsAreEditable('draft', [round(1, 'draft')])).toBe(true)
    expect(countedRoundsAreEditable('draft', [])).toBe(true)
    expect(countedRoundsAreEditable('draft', [round(1, 'draft'), round(2, 'open')])).toBe(false)
    expect(countedRoundsAreEditable('draft', undefined)).toBe(false)
  })

  it('freezes the choice after tournament start even while all rounds remain draft', () => {
    const draftRounds = [round(1, 'draft'), round(2, 'draft')]
    expect(countedRoundsAreEditable('active', draftRounds)).toBe(false)
    expect(countedRoundsAreEditable('completed', draftRounds)).toBe(false)
    expect(countedRoundsAreEditable('archived', draftRounds)).toBe(false)
  })

  it('marks stale and locked failures for authoritative refetch', () => {
    expect(countedRoundsFailure(new ApiHttpError(409, 'tournament_configuration_stale', 'stale')))
      .toMatchObject({ refetch: true, message: expect.stringContaining('endret et annet sted') })
    expect(countedRoundsFailure(new ApiHttpError(409, 'tournament_configuration_locked', 'locked')))
      .toMatchObject({ refetch: true, message: expect.stringContaining('turneringen er startet') })
    expect(countedRoundsFailure(new ApiHttpError(500, 'internal_error', 'Serverfeil')))
      .toEqual({ refetch: false, message: 'Serverfeil' })
  })
})
