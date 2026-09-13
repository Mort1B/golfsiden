import type { LeaderboardMetric, Round, TournamentLeaderboard, TournamentLeaderboardEntry, TournamentTieBreakPolicy } from '../../types'

export const tieTournamentId = '00000000-0000-0000-0000-000000008001'
export const tieRounds: Round[] = [1, 2].map((number) => ({
  id: `00000000-0000-0000-0000-00000000800${number + 1}`, tournament_id: tieTournamentId,
  round_number: number, name: number === 1 ? 'Første tellende runde' : 'Siste planlagte runde med et langt navn',
  round_date: '2026-09-13', course_id: null, course_name: 'Testbane', tee_id: null, tee_name: 'Gul',
  number_of_holes: 18, status: 'locked', handicap_enabled: true, handicap_allowance_percent: 100,
  scoring_format: 'individual_stroke_play', created_at: '2026-09-01T10:00:00Z', updated_at: '2026-09-01T10:00:00Z',
}))

export function tieBoard(metric: LeaderboardMetric = 'gross', policy: TournamentTieBreakPolicy = 'final_round_score'): TournamentLeaderboard {
  const entries: TournamentLeaderboardEntry[] = ['Anna med et svært langt etternavn', 'Bjørn', 'Clara'].map((name, index) => {
    const id = `00000000-0000-0000-0000-00000000801${index}`
    const grossFinal = index === 0 ? 2 : 5
    const netFinal = index === 0 ? 4 : 1
    const comparison = metric === 'gross' ? grossFinal : netFinal
    return {
      player_id: id, display_name: name, status: 'active', completed_rounds: 2, counted_contributions: 1,
      eligible: true, current_team: null, gross_total: 72, net_total: 72, par_total: 72, score_to_par: 0,
      position: policy === 'shared_positions' ? 1 : metric === 'gross' ? (index === 0 ? 1 : 2) : (index === 0 ? 3 : 1),
      tied: policy === 'shared_positions' || index !== 0,
      tie_break_score_to_par: policy === 'shared_positions' ? null : comparison,
      contributions: tieRounds.map((round) => ({
        round_id: round.id, owner: { type: 'player', id }, owner_name: name, provisional: false,
        holes_scored: 18, number_of_holes: 18, gross_total: 72 + (round.round_number === 1 ? 0 : grossFinal),
        net_total: 72 + (round.round_number === 1 ? 0 : netFinal), par_total: 72,
        score_to_par: round.round_number === 1 ? 0 : comparison, counted: round.round_number === 1, mandatory: false,
      })),
    }
  })
  if (metric === 'net' && policy === 'final_round_score') entries.push(...entries.splice(0, 1))
  return {
    tournament_id: tieTournamentId, metric, tie_break_policy: policy, final_round_number: 2,
    required_counted_rounds: 1, mandatory_round_id: null, current_round_id: null,
    included_round_ids: tieRounds.map((round) => round.id), visibility: { mode: 'full' }, entries,
  }
}
