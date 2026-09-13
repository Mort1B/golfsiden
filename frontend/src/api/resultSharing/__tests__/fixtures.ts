import { tieBoard } from '../../leaderboards/__tests__/tieBreakFixtures'
import type { LeaderboardMetric } from '../../types'
import type { PublicResults, ResultShareGrant } from '../contracts'
export const shareId = '00000000-0000-0000-0000-000000009001'
export const shareSecret = 'a'.repeat(43)
export const shareGrant: ResultShareGrant = { id: shareId, created_at: '2026-09-13T10:00:00Z', expires_at: '2099-01-01T10:00:00Z', revoked_at: null }
export function publicFixture(metric: LeaderboardMetric = 'gross'): PublicResults {
  const board = tieBoard(metric)
  return {
    grant_id: shareId, expires_at: shareGrant.expires_at, tournament_name: 'En lang turneringstittel for offentlig resultatdeling',
    metric, required_counted_rounds: board.required_counted_rounds, final_round_number: board.final_round_number,
    tie_break_policy: board.tie_break_policy, visibility: board.visibility,
    entries: board.entries.map(entry => ({
      position: entry.position, tied: entry.tied, display_name: entry.display_name, completed_rounds: entry.completed_rounds,
      counted_contributions: entry.counted_contributions, eligible: entry.eligible, total: metric === 'gross' ? entry.gross_total : entry.net_total,
      par_total: entry.par_total, score_to_par: entry.score_to_par, provisional: false, provisional_holes_scored: 0,
      tie_break_score_to_par: entry.tie_break_score_to_par,
    })),
  }
}
