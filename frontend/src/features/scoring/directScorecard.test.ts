import { describe, expect, it } from 'vitest'
import type { Round, RoundLeaderboard } from '../../api/types'
import { matchingRound, projectedOwner } from './directScorecard'

const round: Round = {
  id: 'round', tournament_id: 'tournament', round_number: 1, name: 'Finale', round_date: '2026-09-02',
  course_id: null, course_name: '', tee_id: null, tee_name: '', number_of_holes: 18, status: 'open',
  handicap_enabled: true, handicap_allowance_percent: 100, scoring_format: 'team_scramble',
  created_at: '2026-09-01T00:00:00Z', updated_at: '2026-09-01T00:00:00Z',
}
const leaderboard: RoundLeaderboard = {
  round_id: 'round', tournament_id: 'tournament', status: 'open', scoring_format: 'team_scramble', metric: 'net',
  number_of_holes: 18, visible_hole_count: 9,
  visibility: { mode: 'front_nine' },
  entries: [{
    position: 1, tied: false, owner: { type: 'team', id: 'team' }, owner_name: 'Langt historisk lag',
    members: [{ player_id: 'one', display_name: 'En', display_order: 1 }, { player_id: 'two', display_name: 'To', display_order: 2 }],
    holes_scored: 3, number_of_holes: 18, complete: null, confirmed: null, playing_handicap: 4,
    gross_total: 15, net_total: 14, par_played: 12, score_to_par: 2,
  }],
}

describe('direct scorecard target resolution', () => {
  it('fails closed for a cross-tournament round before owner resolution', () => {
    expect(matchingRound([round], 'other-tournament', 'round')).toBeNull()
  })

  it('requires the exact projected owner type and identity', () => {
    expect(projectedOwner(leaderboard, 'tournament', 'round', { type: 'team', id: 'team' })?.owner_name).toBe('Langt historisk lag')
    expect(projectedOwner(leaderboard, 'tournament', 'round', { type: 'player', id: 'team' })).toBeNull()
    expect(projectedOwner(leaderboard, 'tournament', 'round', { type: 'team', id: 'other' })).toBeNull()
  })
})
