import type { LeaderboardMetric, TournamentTieBreakPolicy } from '../types'
import type { ScoreVisibility } from '../visibility'

export interface ResultShareGrant { id: string; created_at: string; expires_at: string; revoked_at: string | null }
export interface ResultShareStatus { tournament_id: string; grant: ResultShareGrant | null }
export interface ResultShareReceipt { tournament_id: string; grant: ResultShareGrant; token: string }
interface PublicStandingBase {
  position: number | null
  tied: boolean
  display_name: string
  completed_rounds: number
  counted_contributions: number
  eligible: boolean
  provisional: boolean
  provisional_holes_scored: number
}
export type PublicStanding = PublicStandingBase & ({ value?: undefined;
  total: number
  par_total: number
  score_to_par: number
  tie_break_score_to_par: number | null
} | { value: { type: 'overall_equivalent'; version: 1; selected: number; tie_break: number | null }; total?: never; par_total?: never; score_to_par?: never; tie_break_score_to_par?: never })
export interface PublicResults {
  grant_id: string
  expires_at: string
  tournament_name: string
  metric: LeaderboardMetric
  required_counted_rounds: number
  final_round_number: number
  tie_break_policy: TournamentTieBreakPolicy
  visibility: ScoreVisibility
  entries: PublicStanding[]
}
