import { describe, expect, it } from 'vitest'
import type { Round, RoundStatus } from '../../api/types'
import { leaderboardSearch, preferredRound } from './selection'

function round(roundNumber: number, status: RoundStatus): Round {
  return {
    id: `00000000-0000-0000-0000-${String(roundNumber).padStart(12, '0')}`,
    tournament_id: '00000000-0000-0000-0000-000000002001',
    round_number: roundNumber,
    name: `Runde ${roundNumber}`,
    round_date: '2026-09-10',
    course_id: null,
    course_name: 'Fjord Golfklubb',
    tee_id: null,
    tee_name: 'Gul',
    number_of_holes: 18,
    status,
    handicap_enabled: true,
    handicap_allowance_percent: 100,
    scoring_format: 'team_scramble',
    created_at: '2026-08-16T12:00:00Z',
    updated_at: '2026-08-16T12:00:00Z',
  }
}

describe('leaderboard selection', () => {
  it('prefers the latest open round, then latest finished, then earliest draft', () => {
    expect(preferredRound([round(1, 'draft'), round(2, 'open'), round(3, 'open')])?.round_number).toBe(3)
    expect(preferredRound([round(1, 'locked'), round(2, 'completed'), round(3, 'draft')])?.round_number).toBe(2)
    expect(preferredRound([round(3, 'draft'), round(1, 'draft')])?.round_number).toBe(1)
  })

  it('writes all shareable selectors in a stable order', () => {
    expect(leaderboardSearch('tournament-id', 'round', 'round-id', 'net').toString())
      .toBe('tournament=tournament-id&scope=round&round=round-id&metric=net')
  })
})
